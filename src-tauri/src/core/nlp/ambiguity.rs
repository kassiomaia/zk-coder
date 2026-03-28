//! Defines types for detecting, clarifying, and resolving ambiguities in user input.
//!
//! This module provides the data structures needed to represent ambiguities that
//! may arise during natural language processing, as well as the mechanisms for
//! requesting clarification from the user and tracking their resolutions.

use serde::{Deserialize, Serialize};

use super::multi_intent::TextSpan;

/// Represents the different types of ambiguities that can be detected.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguityType {
    /// A referenced file was not found in the current context.
    MissingFile,
    /// A referenced symbol (e.g., a function or class) was not found.
    MissingSymbol,
    /// A reference could apply to multiple things (e.g., a common term).
    AmbiguousReference,
    /// Multiple matches were found for a search or reference.
    MultipleMatches,
    /// The scope of a request is unclear (e.g., "fix this" without context).
    UnclearScope,
    /// A required parameter for an intent is missing.
    MissingParameter,
}

impl AmbiguityType {
    /// Returns a human-readable label for the ambiguity type.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            AmbiguityType::MissingFile => "Missing File",
            AmbiguityType::MissingSymbol => "Missing Symbol",
            AmbiguityType::AmbiguousReference => "Ambiguous Reference",
            AmbiguityType::MultipleMatches => "Multiple Matches",
            AmbiguityType::UnclearScope => "Unclear Scope",
            AmbiguityType::MissingParameter => "Missing Parameter",
        }
    }
}

/// Represents a detected ambiguity in the user's input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ambiguity {
    /// A unique identifier for this ambiguity.
    pub id: String,
    /// The type of the ambiguity.
    pub ambiguity_type: AmbiguityType,
    /// The location of the ambiguous text in the user's input.
    pub span: TextSpan,
    /// The severity level of the ambiguity.
    pub severity: AmbiguitySeverity,
    /// The request for clarification to be presented to the user.
    pub clarification: ClarificationRequest,
    /// A list of suggested resolutions for the ambiguity.
    pub suggestions: Vec<AmbiguitySuggestion>,
}

impl Ambiguity {
    /// Creates a new `Ambiguity` with an automatically generated ID.
    #[must_use]
    pub fn new(
        ambiguity_type: AmbiguityType,
        span: TextSpan,
        severity: AmbiguitySeverity,
        clarification: ClarificationRequest,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            ambiguity_type,
            span,
            severity,
            clarification,
            suggestions: Vec::new(),
        }
    }

    /// Creates a new `Ambiguity` with suggestions and an auto-generated ID.
    #[must_use]
    pub fn with_suggestions(
        ambiguity_type: AmbiguityType,
        span: TextSpan,
        severity: AmbiguitySeverity,
        clarification: ClarificationRequest,
        suggestions: Vec<AmbiguitySuggestion>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            ambiguity_type,
            span,
            severity,
            clarification,
            suggestions,
        }
    }

    /// Creates a new `Ambiguity` with a specific ID, for testing or reconstruction.
    #[must_use]
    pub fn with_id(
        id: String,
        ambiguity_type: AmbiguityType,
        span: TextSpan,
        severity: AmbiguitySeverity,
        clarification: ClarificationRequest,
        suggestions: Vec<AmbiguitySuggestion>,
    ) -> Self {
        Self {
            id,
            ambiguity_type,
            span,
            severity,
            clarification,
            suggestions,
        }
    }
}

/// The severity level of an ambiguity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguitySeverity {
    /// The system can make a high-confidence guess, but it's worth noting.
    Low,
    /// The system has a likely answer but should confirm with the user.
    Medium,
    /// The system must clarify with the user before proceeding.
    High,
}

impl AmbiguitySeverity {
    /// Returns a CSS class name for styling based on the severity.
    #[must_use]
    pub const fn css_class(&self) -> &'static str {
        match self {
            AmbiguitySeverity::Low => "low",
            AmbiguitySeverity::Medium => "medium",
            AmbiguitySeverity::High => "high",
        }
    }
}

/// A request for clarification to be presented to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationRequest {
    /// The question to ask the user.
    pub question: String,
    /// A list of predefined options for the user to choose from.
    pub options: Vec<ClarificationOption>,
    /// A flag indicating whether the user can provide freeform input.
    pub allows_freeform: bool,
}

impl ClarificationRequest {
    /// Creates a new `ClarificationRequest` with a given question.
    #[must_use]
    pub fn new(question: &str) -> Self {
        Self {
            question: question.to_string(),
            options: Vec::new(),
            allows_freeform: true,
        }
    }

    /// Creates a new `ClarificationRequest` with predefined options.
    #[must_use]
    pub fn with_options(question: &str, options: Vec<ClarificationOption>) -> Self {
        Self {
            question: question.to_string(),
            options,
            allows_freeform: false,
        }
    }

    /// Creates a new `ClarificationRequest` with options and freeform input control.
    #[must_use]
    pub fn with_options_and_freeform(
        question: &str,
        options: Vec<ClarificationOption>,
        allows_freeform: bool,
    ) -> Self {
        Self {
            question: question.to_string(),
            options,
            allows_freeform,
        }
    }
}

/// An option presented to the user in a `ClarificationRequest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationOption {
    /// The value to be returned if this option is selected.
    pub value: String,
    /// The label to be displayed for this option.
    pub label: String,
    /// An optional, more detailed description of the option.
    pub description: Option<String>,
}

impl ClarificationOption {
    /// Creates a new `ClarificationOption` with a value and a label.
    #[must_use]
    pub fn new(value: &str, label: &str) -> Self {
        Self {
            value: value.to_string(),
            label: label.to_string(),
            description: None,
        }
    }

    /// Creates a new `ClarificationOption` with a description.
    #[must_use]
    pub fn with_description(value: &str, label: &str, description: &str) -> Self {
        Self {
            value: value.to_string(),
            label: label.to_string(),
            description: Some(description.to_string()),
        }
    }
}

/// A suggested resolution for an ambiguity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbiguitySuggestion {
    /// The suggested value for resolving the ambiguity.
    pub value: String,
    /// The confidence score for this suggestion, from 0 to 1.
    pub confidence: f64,
    /// The reason for providing this suggestion.
    pub reason: String,
}

impl AmbiguitySuggestion {
    /// Creates a new `AmbiguitySuggestion`.
    #[must_use]
    pub fn new(value: &str, confidence: f64, reason: &str) -> Self {
        Self {
            value: value.to_string(),
            confidence,
            reason: reason.to_string(),
        }
    }
}

/// The type of resolution that was applied to an ambiguity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionType {
    /// The user selected one of the provided options.
    Selected,
    /// The user provided a custom value.
    Custom,
    /// The user dismissed the clarification without resolving it.
    Dismissed,
}

impl ResolutionType {
    /// Returns a human-readable label for the resolution type.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            ResolutionType::Selected => "Selected",
            ResolutionType::Custom => "Custom",
            ResolutionType::Dismissed => "Dismissed",
        }
    }
}

/// Represents a resolved ambiguity, including the chosen resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedAmbiguity {
    /// The ID of the original ambiguity that was resolved.
    pub id: String,
    /// The original text that was ambiguous.
    pub original_text: String,
    /// The value of the resolution, either from a selected option or custom input.
    pub resolution: String,
    /// The type of resolution that was applied.
    pub resolution_type: ResolutionType,
}

impl ResolvedAmbiguity {
    /// Creates a new `ResolvedAmbiguity`.
    #[must_use]
    pub fn new(
        id: String,
        original_text: String,
        resolution: String,
        resolution_type: ResolutionType,
    ) -> Self {
        Self {
            id,
            original_text,
            resolution,
            resolution_type,
        }
    }

    /// Creates a `ResolvedAmbiguity` from a selected option.
    #[must_use]
    pub fn selected(ambiguity: &Ambiguity, resolution: String) -> Self {
        Self {
            id: ambiguity.id.clone(),
            original_text: ambiguity.span.text.clone(),
            resolution,
            resolution_type: ResolutionType::Selected,
        }
    }

    /// Creates a `ResolvedAmbiguity` from a custom user-provided value.
    #[must_use]
    pub fn custom(ambiguity: &Ambiguity, resolution: String) -> Self {
        Self {
            id: ambiguity.id.clone(),
            original_text: ambiguity.span.text.clone(),
            resolution,
            resolution_type: ResolutionType::Custom,
        }
    }

    /// Creates a `ResolvedAmbiguity` for a dismissed clarification.
    #[must_use]
    pub fn dismissed(ambiguity: &Ambiguity) -> Self {
        Self {
            id: ambiguity.id.clone(),
            original_text: ambiguity.span.text.clone(),
            resolution: String::new(),
            resolution_type: ResolutionType::Dismissed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ambiguity_type_label() {
        assert_eq!(AmbiguityType::MissingFile.label(), "Missing File");
        assert_eq!(AmbiguityType::MultipleMatches.label(), "Multiple Matches");
    }

    #[test]
    fn test_ambiguity_severity() {
        assert_eq!(AmbiguitySeverity::High.css_class(), "high");
        assert_eq!(AmbiguitySeverity::Medium.css_class(), "medium");
    }

    #[test]
    fn test_clarification_request() {
        let req = ClarificationRequest::new("Which file?");
        assert_eq!(req.question, "Which file?");
        assert!(req.options.is_empty());
        assert!(req.allows_freeform);
    }

    #[test]
    fn test_clarification_option() {
        let opt = ClarificationOption::new("file.rs", "file.rs");
        assert_eq!(opt.value, "file.rs");
        assert!(opt.description.is_none());

        let opt = ClarificationOption::with_description("file.rs", "file.rs", "Main file");
        assert_eq!(opt.description, Some("Main file".to_string()));
    }

    #[test]
    fn test_ambiguity_suggestion() {
        let sug = AmbiguitySuggestion::new("auth.rs", 0.85, "fuzzy match");
        assert_eq!(sug.value, "auth.rs");
        assert!((sug.confidence - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn test_ambiguity_has_id() {
        let ambiguity = Ambiguity::new(
            AmbiguityType::MissingFile,
            TextSpan::new(0, 5, "test"),
            AmbiguitySeverity::High,
            ClarificationRequest::new("Which file?"),
        );
        assert!(!ambiguity.id.is_empty());
    }

    #[test]
    fn test_resolution_type_label() {
        assert_eq!(ResolutionType::Selected.label(), "Selected");
        assert_eq!(ResolutionType::Custom.label(), "Custom");
        assert_eq!(ResolutionType::Dismissed.label(), "Dismissed");
    }

    #[test]
    fn test_resolved_ambiguity() {
        let ambiguity = Ambiguity::new(
            AmbiguityType::MultipleMatches,
            TextSpan::new(0, 7, "auth.rs"),
            AmbiguitySeverity::Medium,
            ClarificationRequest::new("Which file?"),
        );

        let resolved = ResolvedAmbiguity::selected(&ambiguity, "src/auth.rs".to_string());
        assert_eq!(resolved.id, ambiguity.id);
        assert_eq!(resolved.original_text, "auth.rs");
        assert_eq!(resolved.resolution, "src/auth.rs");
        assert_eq!(resolved.resolution_type, ResolutionType::Selected);
    }
}
