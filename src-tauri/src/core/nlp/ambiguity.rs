//! Ambiguity types for clarification requests.
//!
//! This module defines types for detecting and resolving
//! ambiguities in user input.

use serde::{Deserialize, Serialize};

use super::multi_intent::TextSpan;

/// Types of ambiguities that can be detected.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguityType {
    /// Referenced file not found
    MissingFile,
    /// Referenced symbol not found
    MissingSymbol,
    /// Reference could mean multiple things
    AmbiguousReference,
    /// Multiple matches found
    MultipleMatches,
    /// Scope is unclear
    UnclearScope,
    /// Required parameter missing
    MissingParameter,
}

impl AmbiguityType {
    /// Returns a human-readable label.
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

/// A detected ambiguity in user input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ambiguity {
    /// Unique identifier for this ambiguity
    pub id: String,
    /// Type of ambiguity
    pub ambiguity_type: AmbiguityType,
    /// Location in the input
    pub span: TextSpan,
    /// Severity level
    pub severity: AmbiguitySeverity,
    /// Request for clarification
    pub clarification: ClarificationRequest,
    /// Suggested resolutions
    pub suggestions: Vec<AmbiguitySuggestion>,
}

impl Ambiguity {
    /// Creates a new ambiguity with auto-generated ID.
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

    /// Creates with suggestions and auto-generated ID.
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

    /// Creates with a specific ID (for testing or reconstruction).
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

/// Severity of an ambiguity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguitySeverity {
    /// Can guess with high confidence
    Low,
    /// Should confirm but have likely answer
    Medium,
    /// Must clarify before proceeding
    High,
}

impl AmbiguitySeverity {
    /// Returns a CSS class name for styling.
    #[must_use]
    pub const fn css_class(&self) -> &'static str {
        match self {
            AmbiguitySeverity::Low => "low",
            AmbiguitySeverity::Medium => "medium",
            AmbiguitySeverity::High => "high",
        }
    }
}

/// Request for clarification from the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationRequest {
    /// Question to ask the user
    pub question: String,
    /// Predefined options
    pub options: Vec<ClarificationOption>,
    /// Whether freeform input is allowed
    pub allows_freeform: bool,
}

impl ClarificationRequest {
    /// Creates a new clarification request.
    #[must_use]
    pub fn new(question: &str) -> Self {
        Self {
            question: question.to_string(),
            options: Vec::new(),
            allows_freeform: true,
        }
    }

    /// Creates with options.
    #[must_use]
    pub fn with_options(question: &str, options: Vec<ClarificationOption>) -> Self {
        Self {
            question: question.to_string(),
            options,
            allows_freeform: false,
        }
    }

    /// Creates with options and freeform.
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

/// An option in a clarification request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationOption {
    /// Value to return if selected
    pub value: String,
    /// Display label
    pub label: String,
    /// Optional description
    pub description: Option<String>,
}

impl ClarificationOption {
    /// Creates a new option.
    #[must_use]
    pub fn new(value: &str, label: &str) -> Self {
        Self {
            value: value.to_string(),
            label: label.to_string(),
            description: None,
        }
    }

    /// Creates with description.
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
    /// Suggested value
    pub value: String,
    /// Confidence in this suggestion
    pub confidence: f64,
    /// Reason for the suggestion
    pub reason: String,
}

impl AmbiguitySuggestion {
    /// Creates a new suggestion.
    #[must_use]
    pub fn new(value: &str, confidence: f64, reason: &str) -> Self {
        Self {
            value: value.to_string(),
            confidence,
            reason: reason.to_string(),
        }
    }
}

/// Type of resolution applied to an ambiguity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionType {
    /// User selected from provided options
    Selected,
    /// User entered custom value
    Custom,
    /// User dismissed without resolving
    Dismissed,
}

impl ResolutionType {
    /// Returns a human-readable label.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            ResolutionType::Selected => "Selected",
            ResolutionType::Custom => "Custom",
            ResolutionType::Dismissed => "Dismissed",
        }
    }
}

/// A resolved ambiguity with the chosen resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedAmbiguity {
    /// ID of the original ambiguity
    pub id: String,
    /// The original ambiguous text
    pub original_text: String,
    /// The resolution value (selected option or custom input)
    pub resolution: String,
    /// How the resolution was made
    pub resolution_type: ResolutionType,
}

impl ResolvedAmbiguity {
    /// Creates a new resolved ambiguity.
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

    /// Creates from an ambiguity with a selected option.
    #[must_use]
    pub fn selected(ambiguity: &Ambiguity, resolution: String) -> Self {
        Self {
            id: ambiguity.id.clone(),
            original_text: ambiguity.span.text.clone(),
            resolution,
            resolution_type: ResolutionType::Selected,
        }
    }

    /// Creates from an ambiguity with a custom value.
    #[must_use]
    pub fn custom(ambiguity: &Ambiguity, resolution: String) -> Self {
        Self {
            id: ambiguity.id.clone(),
            original_text: ambiguity.span.text.clone(),
            resolution,
            resolution_type: ResolutionType::Custom,
        }
    }

    /// Creates a dismissed resolution.
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
