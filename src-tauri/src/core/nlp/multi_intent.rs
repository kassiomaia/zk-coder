//! Multi-intent types for complex queries.
//!
//! This module defines types for detecting and handling multiple
//! intents in a single user query.

use serde::{Deserialize, Serialize};

use super::intent::{Intent, IntentParams};

/// A detected intent with its position in the input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedIntent {
    /// The detected intent
    pub intent: Intent,
    /// Extracted parameters
    pub params: IntentParams,
    /// Position in the original text
    pub span: TextSpan,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
}

impl DetectedIntent {
    /// Creates a new detected intent.
    #[must_use]
    pub fn new(intent: Intent, span: TextSpan, confidence: f64) -> Self {
        Self {
            intent,
            params: IntentParams::default(),
            span,
            confidence,
        }
    }

    /// Creates with parameters.
    #[must_use]
    pub fn with_params(
        intent: Intent,
        params: IntentParams,
        span: TextSpan,
        confidence: f64,
    ) -> Self {
        Self {
            intent,
            params,
            span,
            confidence,
        }
    }
}

/// A span of text in the original input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSpan {
    /// Start character position
    pub start: usize,
    /// End character position
    pub end: usize,
    /// The text at this span
    pub text: String,
}

impl TextSpan {
    /// Creates a new text span.
    #[must_use]
    pub fn new(start: usize, end: usize, text: &str) -> Self {
        Self {
            start,
            end,
            text: text.to_string(),
        }
    }

    /// Creates a span for the entire input.
    #[must_use]
    pub fn full(text: &str) -> Self {
        Self {
            start: 0,
            end: text.len(),
            text: text.to_string(),
        }
    }
}

/// Result of multi-intent detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiIntentResult {
    /// Detected intents
    pub intents: Vec<DetectedIntent>,
    /// Relationships between intents
    pub relationships: Vec<IntentRelationship>,
    /// Suggested execution order (indices into intents)
    pub execution_order: Vec<usize>,
}

impl MultiIntentResult {
    /// Creates a single-intent result.
    #[must_use]
    pub fn single(intent: DetectedIntent) -> Self {
        Self {
            intents: vec![intent],
            relationships: vec![],
            execution_order: vec![0],
        }
    }

    /// Returns true if there are multiple intents.
    #[must_use]
    pub fn is_multi(&self) -> bool {
        self.intents.len() > 1
    }

    /// Returns the number of intents.
    #[must_use]
    pub fn len(&self) -> usize {
        self.intents.len()
    }

    /// Returns true if there are no intents.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.intents.is_empty()
    }
}

/// Relationships between detected intents.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntentRelationship {
    /// First intent must complete before second
    Sequential,
    /// Intents can execute simultaneously
    Parallel,
    /// Second intent depends on results of first
    Dependent,
    /// Only one intent should be executed
    Exclusive,
}

impl IntentRelationship {
    /// Returns a human-readable label.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            IntentRelationship::Sequential => "then",
            IntentRelationship::Parallel => "and",
            IntentRelationship::Dependent => "using",
            IntentRelationship::Exclusive => "or",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test fixtures for multi-intent scenarios
    fn create_test_span(text: &str) -> TextSpan {
        TextSpan::full(text)
    }

    fn create_test_result_multiple() -> MultiIntentResult {
        MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::FindBugs, TextSpan::new(0, 10, "find bugs"), 0.9),
                DetectedIntent::new(
                    Intent::GenerateTest,
                    TextSpan::new(15, 25, "write tests"),
                    0.85,
                ),
                DetectedIntent::new(
                    Intent::DocumentCode,
                    TextSpan::new(30, 45, "add documentation"),
                    0.8,
                ),
            ],
            relationships: vec![
                IntentRelationship::Sequential,
                IntentRelationship::Sequential,
            ],
            execution_order: vec![0, 1, 2],
        }
    }

    #[test]
    fn test_text_span() {
        let span = TextSpan::new(0, 5, "hello");
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 5);
        assert_eq!(span.text, "hello");
    }

    #[test]
    fn test_text_span_full() {
        let span = TextSpan::full("hello world");
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 11);
    }

    #[test]
    fn test_text_span_with_offset() {
        let span = TextSpan::new(10, 20, "middle part");
        assert_eq!(span.start, 10);
        assert_eq!(span.end, 20);
        assert_eq!(span.text, "middle part");
    }

    #[test]
    fn test_detected_intent() {
        let span = TextSpan::full("test");
        let detected = DetectedIntent::new(Intent::GenerateCode, span, 0.9);
        assert_eq!(detected.intent, Intent::GenerateCode);
        assert!((detected.confidence - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn test_detected_intent_with_params() {
        let span = create_test_span("generate auth function");
        let params = crate::core::nlp::intent::IntentParams {
            language: Some("Rust".to_string()),
            description: Some("auth function".to_string()),
            ..Default::default()
        };

        let detected =
            DetectedIntent::with_params(Intent::GenerateFunction, params, span.clone(), 0.95);

        assert_eq!(detected.intent, Intent::GenerateFunction);
        assert_eq!(detected.params.language, Some("Rust".to_string()));
        assert_eq!(detected.span.text, "generate auth function");
    }

    #[test]
    fn test_multi_intent_result_single() {
        let span = TextSpan::full("test");
        let detected = DetectedIntent::new(Intent::ExplainCode, span, 0.85);
        let result = MultiIntentResult::single(detected);

        assert!(!result.is_multi());
        assert_eq!(result.len(), 1);
        assert_eq!(result.execution_order, vec![0]);
    }

    #[test]
    fn test_multi_intent_result_multiple() {
        let result = create_test_result_multiple();

        assert!(result.is_multi());
        assert_eq!(result.len(), 3);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_multi_intent_result_empty() {
        let result = MultiIntentResult {
            intents: vec![],
            relationships: vec![],
            execution_order: vec![],
        };

        assert!(result.is_empty());
        assert_eq!(result.len(), 0);
        assert!(!result.is_multi());
    }

    #[test]
    fn test_intent_relationship_label() {
        assert_eq!(IntentRelationship::Sequential.label(), "then");
        assert_eq!(IntentRelationship::Parallel.label(), "and");
        assert_eq!(IntentRelationship::Dependent.label(), "using");
        assert_eq!(IntentRelationship::Exclusive.label(), "or");
    }

    #[test]
    fn test_all_relationship_types_have_labels() {
        let relationships = [
            IntentRelationship::Sequential,
            IntentRelationship::Parallel,
            IntentRelationship::Dependent,
            IntentRelationship::Exclusive,
        ];

        for rel in relationships {
            let label = rel.label();
            assert!(!label.is_empty(), "Relationship {:?} has empty label", rel);
        }
    }

    #[test]
    fn test_confidence_bounds() {
        // Test boundary confidence values
        let span = create_test_span("test");

        let high = DetectedIntent::new(Intent::GenerateCode, span.clone(), 1.0);
        assert!((high.confidence - 1.0).abs() < f64::EPSILON);

        let low = DetectedIntent::new(Intent::Unknown, span.clone(), 0.0);
        assert!((low.confidence - 0.0).abs() < f64::EPSILON);

        let mid = DetectedIntent::new(Intent::ExplainCode, span, 0.5);
        assert!((mid.confidence - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_execution_order_validity() {
        let result = create_test_result_multiple();

        // All indices should be valid
        for &idx in &result.execution_order {
            assert!(
                idx < result.intents.len(),
                "Execution order index {idx} out of bounds"
            );
        }
    }

    #[test]
    fn test_execution_order_can_be_empty() {
        let mut result = create_test_result_multiple();
        result.execution_order.clear();

        // Empty execution order is valid (will use relationships)
        assert!(result.execution_order.is_empty());
    }

    #[test]
    fn test_relationships_count_matches_intents() {
        // For N intents, there should be N-1 relationships
        let result = create_test_result_multiple();
        assert_eq!(result.relationships.len(), result.intents.len() - 1);
    }

    #[test]
    fn test_text_span_serialization() {
        let span = TextSpan::new(0, 10, "test text");

        let json = serde_json::to_string(&span).expect("Should serialize");
        assert!(json.contains("test text"));

        let deserialized: TextSpan = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.start, 0);
        assert_eq!(deserialized.end, 10);
        assert_eq!(deserialized.text, "test text");
    }

    #[test]
    fn test_detected_intent_serialization() {
        let span = TextSpan::full("test");
        let detected = DetectedIntent::new(Intent::GenerateCode, span, 0.9);

        let json = serde_json::to_string(&detected).expect("Should serialize");
        assert!(json.contains("generate_code"));

        let deserialized: DetectedIntent = serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.intent, Intent::GenerateCode);
    }

    #[test]
    fn test_multi_intent_result_serialization() {
        let result = MultiIntentResult {
            intents: vec![DetectedIntent::new(
                Intent::ExplainCode,
                TextSpan::full("explain"),
                0.85,
            )],
            relationships: vec![],
            execution_order: vec![0],
        };

        let json = serde_json::to_string(&result).expect("Should serialize");

        let deserialized: MultiIntentResult =
            serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.len(), 1);
    }

    #[test]
    fn test_intent_relationship_equality() {
        assert_eq!(
            IntentRelationship::Sequential,
            IntentRelationship::Sequential
        );
        assert_eq!(IntentRelationship::Parallel, IntentRelationship::Parallel);
        assert_ne!(IntentRelationship::Sequential, IntentRelationship::Parallel);
    }

    #[test]
    fn test_text_span_full_creates_correct_span() {
        let text = "The quick brown fox";
        let span = TextSpan::full(text);

        assert_eq!(span.start, 0);
        assert_eq!(span.end, text.len());
        assert_eq!(span.text, text);
    }

    #[test]
    fn test_detected_intent_clone() {
        let span = TextSpan::full("test");
        let detected = DetectedIntent::new(Intent::FindBugs, span, 0.8);

        let cloned = detected.clone();
        assert_eq!(cloned.intent, detected.intent);
        assert_eq!(cloned.confidence, detected.confidence);
    }

    #[test]
    fn test_multi_intent_result_with_parallel_relationships() {
        let result = MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::FormatCode, TextSpan::new(0, 5, "format"), 0.9),
                DetectedIntent::new(Intent::LintCode, TextSpan::new(6, 10, "lint"), 0.9),
            ],
            relationships: vec![IntentRelationship::Parallel],
            execution_order: vec![0, 1],
        };

        assert!(result.is_multi());
        assert_eq!(result.relationships[0], IntentRelationship::Parallel);
    }

    #[test]
    fn test_multi_intent_result_with_dependent_relationships() {
        let result = MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::FindBugs, TextSpan::new(0, 8, "find bugs"), 0.9),
                DetectedIntent::new(
                    Intent::GenerateTest,
                    TextSpan::new(9, 20, "write tests"),
                    0.85,
                ),
            ],
            relationships: vec![IntentRelationship::Dependent],
            execution_order: vec![0, 1],
        };

        assert_eq!(result.relationships[0], IntentRelationship::Dependent);
    }

    #[test]
    fn test_multi_intent_result_with_exclusive_relationships() {
        let result = MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::RefactorCode, TextSpan::new(0, 8, "refactor"), 0.8),
                DetectedIntent::new(Intent::OptimizeCode, TextSpan::new(9, 18, "optimize"), 0.8),
            ],
            relationships: vec![IntentRelationship::Exclusive],
            execution_order: vec![0, 1],
        };

        assert_eq!(result.relationships[0], IntentRelationship::Exclusive);
    }

    #[test]
    fn test_detected_intent_params_default() {
        let span = TextSpan::full("test");
        let detected = DetectedIntent::new(Intent::GenerateCode, span, 0.9);

        // Params should be default
        assert!(!detected.params.has_any());
    }

    #[test]
    fn test_multi_intent_result_len_matches_intents() {
        let intents_count = 5;
        let intents: Vec<_> = (0..intents_count)
            .map(|i| {
                DetectedIntent::new(
                    Intent::Unknown,
                    TextSpan::new(i * 10, (i + 1) * 10, &format!("intent_{i}")),
                    0.5,
                )
            })
            .collect();

        let relationships = vec![IntentRelationship::Sequential; intents_count - 1];
        let execution_order: Vec<usize> = (0..intents_count).collect();

        let result = MultiIntentResult {
            intents,
            relationships,
            execution_order,
        };

        assert_eq!(result.len(), intents_count);
    }

    #[test]
    fn test_text_span_edge_cases() {
        // Zero-length span
        let span = TextSpan::new(5, 5, "");
        assert_eq!(span.start, 5);
        assert_eq!(span.end, 5);

        // Span at start of text
        let span = TextSpan::new(0, 5, "hello");
        assert_eq!(span.start, 0);

        // Span with special characters
        let span = TextSpan::new(0, 10, "test@#$%^&*");
        assert_eq!(span.text, "test@#$%^&*");
    }

    #[test]
    fn test_multi_intent_with_varied_confidence_scores() {
        let result = MultiIntentResult {
            intents: vec![
                DetectedIntent::new(Intent::ExplainCode, TextSpan::full("explain"), 0.95),
                DetectedIntent::new(Intent::AnalyzeCode, TextSpan::full("analyze"), 0.75),
                DetectedIntent::new(Intent::Unknown, TextSpan::full("maybe"), 0.5),
            ],
            relationships: vec![IntentRelationship::Sequential, IntentRelationship::Parallel],
            execution_order: vec![0, 1, 2],
        };

        // Verify all confidence scores are present
        assert_eq!(result.intents[0].confidence, 0.95);
        assert_eq!(result.intents[1].confidence, 0.75);
        assert_eq!(result.intents[2].confidence, 0.5);
    }
}
