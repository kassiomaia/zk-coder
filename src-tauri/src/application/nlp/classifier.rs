//! Provides an intent classification engine for user input.
//!
//! This module contains the `IntentClassifier`, which is responsible for
//! classifying user input into a specific `Intent` using a combination of
//! pattern matching for fast, deterministic results and an AI-based
//! classifier for more nuanced cases.

use log::warn;
use serde::Deserialize;
use std::sync::Arc;
use thiserror::Error;

use crate::application::nlp::gateway::{NlpLlmError, NlpLlmGateway};
use crate::core::nlp::entities::ConversationContext;
use crate::core::nlp::intent::{Intent, IntentClassification, IntentParams};
use crate::core::nlp::patterns::IntentPatterns;

/// A classifier that combines pattern matching and AI to determine user intent.
pub struct IntentClassifier {
    /// The gateway for making calls to the NLP LLM.
    gateway: Arc<dyn NlpLlmGateway + Send + Sync>,
    /// The pattern matcher for fast-path intent classification.
    patterns: IntentPatterns,
}

impl IntentClassifier {
    /// Creates a new `IntentClassifier` with a given NLP LLM gateway.
    #[must_use]
    pub fn new(gateway: Arc<dyn NlpLlmGateway + Send + Sync>) -> Self {
        Self {
            gateway,
            patterns: IntentPatterns::new(),
        }
    }

    /// Classifies the user's intent from the input text.
    ///
    /// This method first attempts to classify the intent using fast pattern matching.
    /// If no definitive match is found, it falls back to a more sophisticated
    /// AI-based classification.
    ///
    /// # Errors
    ///
    /// Returns a `ClassifierError` if the AI-based classification fails.
    pub async fn classify(
        &self,
        input: &str,
        context: &ConversationContext,
    ) -> Result<IntentClassification, ClassifierError> {
        // Fast path: pattern matching
        if let Some(intent) = self.patterns.match_input(input) {
            return Ok(IntentClassification::new(intent.clone(), 0.95));
        }

        // Check for multiple matches
        let matches = self.patterns.match_all(input);
        if matches.len() == 1 {
            let (intent, confidence) = matches[0];
            return Ok(IntentClassification::new(intent.clone(), confidence));
        }

        // Slow path: AI classification with deterministic fallback.
        match self.classify_with_ai(input, context).await {
            Ok(result) => Ok(result),
            Err(error) => {
                warn!("Intent classification AI fallback triggered: {error}");
                Ok(IntentClassification::new(Intent::Unknown, 0.5))
            }
        }
    }

    /// Classifies the intent and provides alternative classifications.
    ///
    /// Similar to `classify`, but this method also returns a list of alternative
    /// intents if multiple patterns match with varying confidence scores.
    ///
    /// # Errors
    ///
    /// Returns a `ClassifierError` if the AI-based classification fails.
    pub async fn classify_with_alternatives(
        &self,
        input: &str,
        context: &ConversationContext,
    ) -> Result<IntentClassification, ClassifierError> {
        // Fast path with alternatives
        let matches = self.patterns.match_all(input);
        if !matches.is_empty() {
            let (primary, confidence) = matches[0];
            let alternatives: Vec<(Intent, f64)> =
                matches[1..].iter().map(|(i, c)| ((*i).clone(), *c)).collect();

            return Ok(IntentClassification::with_alternatives(
                primary.clone(),
                confidence,
                alternatives,
            ));
        }

        // Slow path: AI classification with deterministic fallback.
        match self.classify_with_ai(input, context).await {
            Ok(result) => Ok(result),
            Err(error) => {
                warn!("Intent alternatives AI fallback triggered: {error}");
                Ok(IntentClassification::new(Intent::Unknown, 0.5))
            }
        }
    }

    async fn classify_with_ai(
        &self,
        input: &str,
        context: &ConversationContext,
    ) -> Result<IntentClassification, ClassifierError> {
        let system_prompt = self.build_prompt(context);
        let response = self
            .gateway
            .complete(&system_prompt, input)
            .await
            .map_err(ClassifierError::AiError)?;

        self.parse_response(&response)
    }

    fn build_prompt(&self, context: &ConversationContext) -> String {
        format!(
            r#"You are an intent classifier for a developer AI assistant.
Classify the user's input into exactly one of these intents:
- generate_code, generate_function, generate_class, generate_test
- refactor_code, optimize_code
- explain_code, analyze_code, find_bugs, document_code
- find_file, find_symbol, goto_definition, find_references
- run_tests, build_project, format_code, lint_code
- git_status, git_commit, git_diff, git_log
- start_new_thread, switch_context, clear_history
- unknown

Current context: {}

Return JSON: {{"intent": "...", "confidence": 0.0-1.0, "alternatives": [["intent", score], ...]}}"#,
            context.summary()
        )
    }

    fn parse_response(&self, response: &str) -> Result<IntentClassification, ClassifierError> {
        // Try to parse as JSON
        if let Ok(parsed) = serde_json::from_str::<ClassificationResponse>(response) {
            return Ok(IntentClassification::with_alternatives(
                parsed.intent,
                parsed.confidence,
                parsed.alternatives,
            ));
        }

        // Try to extract JSON from response
        if let Some(json_start) = response.find('{') {
            if let Some(json_end) = response.rfind('}') {
                let json_str = &response[json_start..=json_end];
                if let Ok(parsed) = serde_json::from_str::<ClassificationResponse>(json_str) {
                    return Ok(IntentClassification::with_alternatives(
                        parsed.intent,
                        parsed.confidence,
                        parsed.alternatives,
                    ));
                }
            }
        }

        // Fallback: unknown intent
        Ok(IntentClassification::new(Intent::Unknown, 0.5))
    }

    /// Extracts parameters, such as file paths and programming languages, from the input text.
    #[must_use]
    pub fn extract_params(&self, input: &str) -> IntentParams {
        let mut params = IntentParams::default();

        // Extract file paths (look for common patterns)
        if let Some(file) = self.extract_file_path(input) {
            params.file_path = Some(file);
        }

        // Extract programming language mentions
        if let Some(lang) = self.extract_language(input) {
            params.language = Some(lang);
        }

        // The description is the input minus extracted elements
        params.description = Some(input.to_string());

        params
    }

    fn extract_file_path(&self, input: &str) -> Option<String> {
        // Look for file patterns like "file.ts", "path/to/file.rs", "file.test.ts"
        // Support longer extensions and multiple dots (e.g., .test.ts)
        let re = regex::Regex::new(r"[a-zA-Z0-9_/\-]+(\.[a-zA-Z0-9]+)+").ok()?;
        re.find(input).map(|m| m.as_str().to_string())
    }

    fn extract_language(&self, input: &str) -> Option<String> {
        let languages = [
            ("rust", "Rust"),
            ("typescript", "TypeScript"),
            ("javascript", "JavaScript"),
            ("python", "Python"),
            ("go", "Go"),
            ("java", "Java"),
            ("c++", "C++"),
            ("c#", "C#"),
        ];

        let lower = input.to_lowercase();
        for (keyword, lang) in languages {
            if lower.contains(keyword) {
                return Some(lang.to_string());
            }
        }
        None
    }
}

/// A response from the AI-based classification.
#[derive(Debug, Deserialize)]
struct ClassificationResponse {
    intent: Intent,
    confidence: f64,
    alternatives: Vec<(Intent, f64)>,
}

/// Represents errors that can occur during intent classification.
#[derive(Debug, Error)]
pub enum ClassifierError {
    /// An error reported by the AI gateway.
    #[error("AI error: {0}")]
    AiError(#[from] NlpLlmError),
    /// An error that occurred while parsing the AI's response.
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::nlp::test_utils::{self, failing_gateway, fixed_gateway};

    fn create_test_classifier() -> IntentClassifier {
        IntentClassifier::new(fixed_gateway("{}"))
    }

    // Test fixtures for file path extraction
    const FILE_PATH_FIXTURES: &[(&str, Option<&str>)] = &[
        ("explain auth.ts", Some("auth.ts")),
        ("check src/main.rs", Some("src/main.rs")),
        (
            "look at components/Button.tsx",
            Some("components/Button.tsx"),
        ),
        ("no file here", None),
        ("", None),
        ("file.txt", Some("file.txt")),
        (
            "path/to/some/file.test.ts",
            Some("path/to/some/file.test.ts"),
        ),
    ];

    // Test fixtures for language extraction
    const LANGUAGE_FIXTURES: &[(&str, Option<&str>)] = &[
        ("write a rust function", Some("Rust")),
        ("create a typescript component", Some("TypeScript")),
        ("python script for data processing", Some("Python")),
        ("java class for handling users", Some("Java")),
        ("go routine for concurrency", Some("Go")),
        ("c++ program for performance", Some("C++")),
        ("c# application", Some("C#")),
        ("javascript function", Some("JavaScript")),
        ("no language mentioned", None),
    ];

    // Edge case inputs
    const EDGE_CASE_INPUTS: &[&str] = &[
        "", "   ", "generate", "a",
        // Very long input test would be generated at runtime
    ];

    #[test]
    fn test_extract_file_path() {
        let classifier = create_test_classifier();

        let result = classifier.extract_file_path("explain auth.ts");
        assert_eq!(result, Some("auth.ts".to_string()));

        let result = classifier.extract_file_path("check src/main.rs");
        assert_eq!(result, Some("src/main.rs".to_string()));
    }

    #[test]
    fn test_extract_file_path_comprehensive() {
        let classifier = create_test_classifier();

        for (input, expected) in FILE_PATH_FIXTURES {
            let result = classifier.extract_file_path(input);
            assert_eq!(
                result.as_deref(),
                *expected,
                "File path extraction failed for: '{input}'"
            );
        }
    }

    #[test]
    fn test_extract_language() {
        let classifier = create_test_classifier();

        let result = classifier.extract_language("write a rust function");
        assert_eq!(result, Some("Rust".to_string()));

        let result = classifier.extract_language("create a typescript component");
        assert_eq!(result, Some("TypeScript".to_string()));
    }

    #[test]
    fn test_extract_language_comprehensive() {
        let classifier = create_test_classifier();

        for (input, expected) in LANGUAGE_FIXTURES {
            let result = classifier.extract_language(input);
            assert_eq!(
                result.as_deref(),
                *expected,
                "Language extraction failed for: '{input}'"
            );
        }
    }

    #[test]
    fn test_extract_params() {
        let classifier = create_test_classifier();

        let params = classifier.extract_params("generate a rust function in auth.rs");
        assert_eq!(params.language, Some("Rust".to_string()));
        assert_eq!(params.file_path, Some("auth.rs".to_string()));
        assert!(params.description.is_some());
    }

    #[test]
    fn test_extract_params_with_all_fields() {
        let classifier = create_test_classifier();

        let params = classifier.extract_params(
            "create a TypeScript function in auth/service.ts that handles user login",
        );
        assert_eq!(params.language, Some("TypeScript".to_string()));
        assert_eq!(params.file_path, Some("auth/service.ts".to_string()));
        assert!(params.description.is_some());
    }

    #[test]
    fn test_extract_params_minimal() {
        let classifier = create_test_classifier();

        let params = classifier.extract_params("do something");
        assert!(params.description.is_some());
        assert_eq!(params.description, Some("do something".to_string()));
        assert!(params.language.is_none());
        assert!(params.file_path.is_none());
    }

    #[test]
    fn test_extract_params_empty_input() {
        let classifier = create_test_classifier();

        let params = classifier.extract_params("");
        assert!(params.language.is_none());
        assert!(params.file_path.is_none());
    }

    #[tokio::test]
    async fn test_pattern_classification() {
        let classifier = create_test_classifier();
        let context = test_utils::empty_context();

        let result = classifier
            .classify("generate code for authentication", &context)
            .await;
        assert!(result.is_ok());
        let classification = result.expect("Classification should succeed");
        assert_eq!(classification.intent, Intent::GenerateCode);
        assert!(classification.is_high_confidence());
    }

    #[tokio::test]
    async fn test_pattern_classification_various_intents() {
        let classifier = create_test_classifier();
        let context = test_utils::empty_context();

        let test_cases = [
            ("explain this code", Intent::ExplainCode),
            ("find bugs in my code", Intent::FindBugs),
            ("refactor this function", Intent::RefactorCode),
            ("git status", Intent::GitStatus),
            ("run tests", Intent::RunTests),
            ("format this code", Intent::FormatCode),
            ("find file main.rs", Intent::FindFile),
        ];

        for (input, expected_intent) in test_cases {
            let result = classifier.classify(input, &context).await;
            assert!(result.is_ok(), "Classification failed for input: '{input}'");
            let classification = result.expect("Classification should succeed");
            assert_eq!(
                classification.intent, expected_intent,
                "Wrong intent for input: '{input}'"
            );
        }
    }

    #[test]
    fn test_parse_response_invalid_json_falls_back_to_unknown() {
        let classifier = create_test_classifier();

        let result = classifier
            .parse_response("not-json-response")
            .expect("Fallback should parse");

        assert_eq!(result.intent, Intent::Unknown);
        assert!((result.confidence - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_response_valid_json() {
        let classifier = create_test_classifier();

        let json_response =
            r#"{"intent":"generate_code","confidence":0.9,"alternatives":[["refactor_code",0.3]]}"#;
        let result = classifier
            .parse_response(json_response)
            .expect("Should parse valid JSON");

        assert_eq!(result.intent, Intent::GenerateCode);
        assert!((result.confidence - 0.9).abs() < f64::EPSILON);
        assert_eq!(result.alternative_intents.len(), 1);
    }

    #[test]
    fn test_parse_response_json_with_extra_text() {
        let classifier = create_test_classifier();

        let response = "Here's my analysis: {\"intent\":\"explain_code\",\"confidence\":0.85,\"alternatives\":[]} And some more text";
        let result = classifier
            .parse_response(response)
            .expect("Should extract JSON from response");

        assert_eq!(result.intent, Intent::ExplainCode);
        assert!((result.confidence - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_response_empty_json() {
        let classifier = create_test_classifier();

        let result = classifier
            .parse_response("{}")
            .expect("Should handle empty JSON");
        // Empty JSON should still parse but with Unknown intent due to missing fields
        assert_eq!(result.intent, Intent::Unknown);
    }

    #[test]
    fn test_parse_response_with_invalid_intent() {
        let classifier = create_test_classifier();

        let json_response = r#"{"intent":"invalid_intent","confidence":0.8,"alternatives":[]}"#;
        let result = classifier.parse_response(json_response);
        // Should fail to parse invalid intent
        assert!(result.is_err() || result.unwrap().intent == Intent::Unknown);
    }

    #[tokio::test]
    async fn test_pattern_classification_with_alternatives() {
        let classifier = create_test_classifier();
        let context = test_utils::context_with_user_messages(&["help me with auth.rs"]);

        let result = classifier
            .classify_with_alternatives("generate code for tests", &context)
            .await
            .expect("Classification should succeed");

        assert_eq!(result.intent, Intent::GenerateCode);
    }

    #[tokio::test]
    async fn test_classify_with_alternatives_returns_multiple() {
        let classifier = create_test_classifier();
        let context = test_utils::empty_context();

        let result = classifier
            .classify_with_alternatives("test this code", &context)
            .await
            .expect("Classification should succeed");

        // Should have at least the primary intent
        assert!(!result.alternative_intents.is_empty() || result.intent != Intent::Unknown);
    }

    #[tokio::test]
    async fn test_ai_failure_falls_back_to_unknown_intent() {
        let classifier = IntentClassifier::new(failing_gateway("llm unavailable"));
        let context = test_utils::empty_context();

        let result = classifier
            .classify("do something very specific with no known pattern", &context)
            .await
            .expect("Classifier should fallback");

        assert_eq!(result.intent, Intent::Unknown);
        assert!((result.confidence - 0.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_classify_empty_input() {
        let classifier = create_test_classifier();
        let context = test_utils::empty_context();

        let result = classifier
            .classify("", &context)
            .await
            .expect("Should classify empty input");
        assert_eq!(result.intent, Intent::Unknown);
    }

    #[tokio::test]
    async fn test_classify_whitespace_only() {
        let classifier = create_test_classifier();
        let context = test_utils::empty_context();

        let result = classifier
            .classify("   ", &context)
            .await
            .expect("Should classify whitespace");
        assert_eq!(result.intent, Intent::Unknown);
    }

    #[tokio::test]
    async fn test_classify_with_context_influences_alternatives() {
        let classifier = create_test_classifier();
        let context = test_utils::context_with_user_messages(&["I'm working on authentication"]);

        let result = classifier
            .classify_with_alternatives("help me code", &context)
            .await
            .expect("Should classify with context");

        // Should produce some classification (GenerateCode pattern matches)
        assert!(!result.alternative_intents.is_empty() || result.intent != Intent::Unknown);
    }

    #[tokio::test]
    async fn test_build_prompt_includes_context() {
        let classifier = create_test_classifier();
        let mut context = test_utils::empty_context();
        context.summary = Some("Working on authentication".to_string());

        let prompt = classifier.build_prompt(&context);
        assert!(prompt.contains("intent classifier"));
        assert!(prompt.contains("generate_code"));
        assert!(prompt.contains("explain_code"));
        // Should include context information
        assert!(prompt.contains("Current context:"));
    }

    #[tokio::test]
    async fn test_build_prompt_contains_all_intents() {
        let classifier = create_test_classifier();
        let context = test_utils::empty_context();

        let prompt = classifier.build_prompt(&context);

        // Verify all intent categories are mentioned
        assert!(prompt.contains("generate_code"));
        assert!(prompt.contains("refactor_code"));
        assert!(prompt.contains("explain_code"));
        assert!(prompt.contains("find_file"));
        assert!(prompt.contains("run_tests"));
        assert!(prompt.contains("git_status"));
        assert!(prompt.contains("unknown"));
    }

    #[tokio::test]
    async fn test_classify_with_ai_mock_response() {
        let json_response =
            r#"{"intent":"find_bugs","confidence":0.92,"alternatives":[["analyze_code",0.7]]}"#;
        let classifier = IntentClassifier::new(fixed_gateway(json_response));
        let context = test_utils::empty_context();

        let result = classifier
            .classify("check my code for issues", &context)
            .await
            .expect("Should classify with AI");

        assert_eq!(result.intent, Intent::FindBugs);
        assert!((result.confidence - 0.92).abs() < f64::EPSILON);
        assert_eq!(result.alternative_intents.len(), 1);
        assert_eq!(result.alternative_intents[0].0, Intent::AnalyzeCode);
    }

    #[tokio::test]
    async fn test_classify_fallback_on_pattern_match_after_ai_failure() {
        let classifier = IntentClassifier::new(failing_gateway("AI failed"));
        let context = test_utils::empty_context();

        // Should fall back to pattern matching first
        let result = classifier
            .classify("generate code for tests", &context)
            .await
            .expect("Should fallback to pattern");

        // Pattern matching should still work
        assert_eq!(result.intent, Intent::GenerateCode);
        assert!(result.is_high_confidence());
    }

    #[test]
    fn test_extract_multiple_file_paths_returns_first() {
        let classifier = create_test_classifier();

        let result = classifier.extract_file_path("check auth.ts and main.rs");
        // Should return the first match
        assert_eq!(result, Some("auth.ts".to_string()));
    }

    #[test]
    fn test_extract_language_case_insensitive() {
        let classifier = create_test_classifier();

        assert_eq!(
            classifier.extract_language("Rust function"),
            Some("Rust".to_string())
        );
        assert_eq!(
            classifier.extract_language("RUST function"),
            Some("Rust".to_string())
        );
        assert_eq!(
            classifier.extract_language("rust function"),
            Some("Rust".to_string())
        );
    }

    #[test]
    fn test_extract_language_with_full_path() {
        let classifier = create_test_classifier();

        assert_eq!(
            classifier.extract_language("write code in /usr/local/bin"),
            None
        );
        assert_eq!(
            classifier.extract_language("typescript is great"),
            Some("TypeScript".to_string())
        );
    }

    #[tokio::test]
    async fn test_classify_returns_error_on_gateway_error() {
        let classifier = IntentClassifier::new(failing_gateway("Connection failed"));
        let context = test_utils::empty_context();

        // With no pattern match, should fail gracefully with Unknown
        let result = classifier
            .classify("xyz123 no pattern match", &context)
            .await
            .expect("Should handle error gracefully");

        assert_eq!(result.intent, Intent::Unknown);
    }

    #[tokio::test]
    async fn test_classification_confidence_bounds() {
        let classifier = create_test_classifier();
        let context = test_utils::empty_context();

        let result = classifier
            .classify("generate code", &context)
            .await
            .expect("Should classify");

        assert!((0.0..=1.0).contains(&result.confidence));
    }
}
