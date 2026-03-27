//! Multi-intent detection engine.
//!
//! This module detects and parses multiple intents from complex queries.

use serde::Deserialize;
use std::sync::Arc;
use thiserror::Error;

use crate::application::nlp::gateway::{NlpLlmError, NlpLlmGateway};
use crate::core::nlp::entities::ConversationContext;
use crate::core::nlp::intent::Intent;
use crate::core::nlp::multi_intent::{
    DetectedIntent, IntentRelationship, MultiIntentResult, TextSpan,
};
use crate::core::nlp::patterns::IntentPatterns;

/// Multi-intent detector.
pub struct MultiIntentDetector {
    /// LLM gateway for multi-intent decomposition
    gateway: Arc<dyn NlpLlmGateway + Send + Sync>,
    /// Pattern matcher for fast-path
    patterns: IntentPatterns,
    /// Conjunction words that indicate multiple intents
    conjunctions: Vec<&'static str>,
}

impl MultiIntentDetector {
    /// Creates a new multi-intent detector with an abstract NLP LLM gateway.
    #[must_use]
    pub fn new(gateway: Arc<dyn NlpLlmGateway + Send + Sync>) -> Self {
        Self {
            gateway,
            patterns: IntentPatterns::new(),
            conjunctions: vec![" and ", " then ", " after ", " before ", " while "],
        }
    }

    /// Detects intents in the input.
    ///
    /// # Errors
    ///
    /// Returns an error if AI detection fails.
    pub async fn detect(
        &self,
        input: &str,
        context: &ConversationContext,
    ) -> Result<MultiIntentResult, DetectorError> {
        // Check for conjunctions
        if !self.has_conjunctions(input) {
            // Single intent - use pattern matching or return unknown
            return Ok(self.detect_single(input));
        }

        // Multi-intent: use AI with deterministic fallback.
        match self.detect_with_ai(input, context).await {
            Ok(result) => Ok(result),
            Err(_) => Ok(self.detect_single(input)),
        }
    }

    fn has_conjunctions(&self, input: &str) -> bool {
        let lower = input.to_lowercase();
        self.conjunctions.iter().any(|c| lower.contains(c))
    }

    fn detect_single(&self, input: &str) -> MultiIntentResult {
        if let Some(intent) = self.patterns.match_input(input) {
            let detected = DetectedIntent::new(intent.clone(), TextSpan::full(input), 0.95);
            return MultiIntentResult::single(detected);
        }

        // Unknown intent
        let detected = DetectedIntent::new(Intent::Unknown, TextSpan::full(input), 0.5);
        MultiIntentResult::single(detected)
    }

    async fn detect_with_ai(
        &self,
        input: &str,
        context: &ConversationContext,
    ) -> Result<MultiIntentResult, DetectorError> {
        let prompt = self.build_prompt(input, context);
        let response = self
            .gateway
            .complete(
                "You are a multi-intent detector for developer commands.",
                &prompt,
            )
            .await
            .map_err(DetectorError::AiError)?;

        self.parse_response(&response, input)
    }

    fn build_prompt(&self, input: &str, context: &ConversationContext) -> String {
        format!(
            r#"Analyze this developer query for multiple intents: '{}'

Context: {}

Return JSON:
{{
  "intents": [
    {{"intent": "...", "span": {{"start": n, "end": n}}, "confidence": 0.0-1.0}}
  ],
  "relationships": ["sequential" | "parallel" | "dependent" | "exclusive"],
  "execution_order": [0, 1, ...]
}}

Valid intents: generate_code, generate_function, generate_class, generate_test, refactor_code, optimize_code, explain_code, analyze_code, find_bugs, document_code, find_file, find_symbol, run_tests, build_project, format_code, git_status, git_commit, unknown"#,
            input,
            context.summary()
        )
    }

    fn parse_response(
        &self,
        response: &str,
        original_input: &str,
    ) -> Result<MultiIntentResult, DetectorError> {
        // Try to extract JSON
        if let Some(json_start) = response.find('{') {
            if let Some(json_end) = response.rfind('}') {
                let json_str = &response[json_start..=json_end];
                if let Ok(parsed) = serde_json::from_str::<MultiIntentResponse>(json_str) {
                    return Ok(self.convert_response(parsed, original_input));
                }
            }
        }

        // Fallback: single unknown intent
        Ok(self.detect_single(original_input))
    }

    fn convert_response(
        &self,
        parsed: MultiIntentResponse,
        original_input: &str,
    ) -> MultiIntentResult {
        let intents: Vec<DetectedIntent> = parsed
            .intents
            .into_iter()
            .map(|i| {
                let span = i.span.map_or_else(
                    || TextSpan::full(original_input),
                    |s| {
                        TextSpan::new(
                            s.start,
                            s.end,
                            &original_input[s.start..s.end.min(original_input.len())],
                        )
                    },
                );
                DetectedIntent::new(i.intent, span, i.confidence)
            })
            .collect();

        MultiIntentResult {
            execution_order: parsed.execution_order,
            relationships: parsed.relationships,
            intents,
        }
    }
}

/// Response from AI multi-intent detection.
#[derive(Debug, Deserialize)]
struct MultiIntentResponse {
    intents: Vec<IntentItem>,
    relationships: Vec<IntentRelationship>,
    execution_order: Vec<usize>,
}

/// Single intent in the response.
#[derive(Debug, Deserialize)]
struct IntentItem {
    intent: Intent,
    span: Option<SpanItem>,
    confidence: f64,
}

/// Span in the response.
#[derive(Debug, Deserialize)]
struct SpanItem {
    start: usize,
    end: usize,
}

/// Errors from multi-intent detection.
#[derive(Debug, Error)]
pub enum DetectorError {
    /// AI error
    #[error("AI error: {0}")]
    AiError(#[from] NlpLlmError),
    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::nlp::test_utils::{empty_context, failing_gateway, fixed_gateway};

    fn create_test_detector() -> MultiIntentDetector {
        MultiIntentDetector::new(fixed_gateway("{}"))
    }

    #[test]
    fn test_has_conjunctions() {
        let detector = create_test_detector();

        assert!(detector.has_conjunctions("find bugs and write tests"));
        assert!(detector.has_conjunctions("explain code then refactor"));
        assert!(!detector.has_conjunctions("explain this code"));
    }

    #[test]
    fn test_detect_single() {
        let detector = create_test_detector();

        let result = detector.detect_single("generate code for authentication");
        assert!(!result.is_multi());
        assert_eq!(result.intents[0].intent, Intent::GenerateCode);
    }

    #[tokio::test]
    async fn test_detect_no_conjunctions() {
        let detector = create_test_detector();
        let context = empty_context();

        let result = detector
            .detect("explain this code", &context)
            .await
            .expect("Should detect");
        assert!(!result.is_multi());
    }

    #[test]
    fn test_parse_response_invalid_json_falls_back_to_single() {
        let detector = create_test_detector();
        let result = detector
            .parse_response("not-json-response", "find bugs and then fix them")
            .expect("Fallback should parse");

        assert!(!result.is_multi());
        assert_eq!(result.intents.len(), 1);
    }

    #[tokio::test]
    async fn llm_failure_falls_back_to_single_intent() {
        let detector = MultiIntentDetector::new(failing_gateway("gateway unavailable"));
        let context = empty_context();

        let result = detector
            .detect("find bugs and then write tests", &context)
            .await
            .expect("Should return fallback result");

        assert!(!result.is_multi());
        assert_eq!(result.intents.len(), 1);
    }
}
