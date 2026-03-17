//! Intent classification engine.
//!
//! This module provides intent classification using pattern matching
//! and AI-based classification.

use serde::Deserialize;
use std::sync::Arc;
use thiserror::Error;

use super::context::ConversationContext;
use super::intent::{Intent, IntentClassification, IntentParams};
use super::patterns::IntentPatterns;
use crate::ai::{ClaudeClient, ClaudeError};

/// Intent classifier combining pattern matching and AI.
pub struct IntentClassifier {
    /// Claude API client
    claude: Arc<ClaudeClient>,
    /// Pattern matcher for fast-path classification
    patterns: IntentPatterns,
}

impl IntentClassifier {
    /// Creates a new intent classifier.
    #[must_use]
    pub fn new(claude: Arc<ClaudeClient>) -> Self {
        Self {
            claude,
            patterns: IntentPatterns::new(),
        }
    }

    /// Classifies intent from user input.
    ///
    /// # Errors
    ///
    /// Returns an error if AI classification fails.
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

        // Slow path: AI classification
        self.classify_with_ai(input, context).await
    }

    /// Classifies with additional alternatives.
    ///
    /// # Errors
    ///
    /// Returns an error if AI classification fails.
    pub async fn classify_with_alternatives(
        &self,
        input: &str,
        context: &ConversationContext,
    ) -> Result<IntentClassification, ClassifierError> {
        // Fast path with alternatives
        let matches = self.patterns.match_all(input);
        if !matches.is_empty() {
            let (primary, confidence) = matches[0];
            let alternatives: Vec<(Intent, f64)> = matches[1..]
                .iter()
                .map(|(i, c)| ((*i).clone(), *c))
                .collect();

            return Ok(IntentClassification::with_alternatives(
                primary.clone(),
                confidence,
                alternatives,
            ));
        }

        // Slow path: AI classification
        self.classify_with_ai(input, context).await
    }

    async fn classify_with_ai(
        &self,
        input: &str,
        context: &ConversationContext,
    ) -> Result<IntentClassification, ClassifierError> {
        let system_prompt = self.build_prompt(context);
        let response = self
            .claude
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

    /// Extracts parameters from input text.
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
        // Look for file patterns like "file.ts", "path/to/file.rs"
        let re = regex::Regex::new(r"[a-zA-Z0-9_/\-]+\.[a-zA-Z0-9]{1,4}").ok()?;
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

/// Response from AI classification.
#[derive(Debug, Deserialize)]
struct ClassificationResponse {
    intent: Intent,
    confidence: f64,
    alternatives: Vec<(Intent, f64)>,
}

/// Errors from intent classification.
#[derive(Debug, Error)]
pub enum ClassifierError {
    /// AI error
    #[error("AI error: {0}")]
    AiError(#[from] ClaudeError),
    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nlp::test_utils;

    fn create_test_classifier() -> IntentClassifier {
        IntentClassifier::new(Arc::new(ClaudeClient::new("test-key")))
    }

    #[test]
    fn test_extract_file_path() {
        let classifier = create_test_classifier();

        let result = classifier.extract_file_path("explain auth.ts");
        assert_eq!(result, Some("auth.ts".to_string()));

        let result = classifier.extract_file_path("check src/main.rs");
        assert_eq!(result, Some("src/main.rs".to_string()));
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
    fn test_extract_params() {
        let classifier = create_test_classifier();

        let params = classifier.extract_params("generate a rust function in auth.rs");
        assert_eq!(params.language, Some("Rust".to_string()));
        assert_eq!(params.file_path, Some("auth.rs".to_string()));
        assert!(params.description.is_some());
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

    #[test]
    fn test_parse_response_invalid_json_falls_back_to_unknown() {
        let classifier = create_test_classifier();

        let result = classifier
            .parse_response("not-json-response")
            .expect("Fallback should parse");

        assert_eq!(result.intent, Intent::Unknown);
        assert!((result.confidence - 0.5).abs() < f64::EPSILON);
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
}
