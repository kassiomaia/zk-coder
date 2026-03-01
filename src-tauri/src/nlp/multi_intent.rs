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
    pub fn with_params(intent: Intent, params: IntentParams, span: TextSpan, confidence: f64) -> Self {
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
    fn test_detected_intent() {
        let span = TextSpan::full("test");
        let detected = DetectedIntent::new(Intent::GenerateCode, span, 0.9);
        assert_eq!(detected.intent, Intent::GenerateCode);
        assert!((detected.confidence - 0.9).abs() < f64::EPSILON);
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
    fn test_intent_relationship_label() {
        assert_eq!(IntentRelationship::Sequential.label(), "then");
        assert_eq!(IntentRelationship::Parallel.label(), "and");
    }
}
