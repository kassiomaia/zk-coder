//! Intent types and taxonomy for the NLP system.
//!
//! This module defines the intent types that the system can recognize
//! from natural language input.

use serde::{Deserialize, Serialize};

/// Developer command intents supported by the system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Intent {
    // Code generation intents
    /// Generate code from a description
    GenerateCode,
    /// Generate a function
    GenerateFunction,
    /// Generate a class
    GenerateClass,
    /// Generate tests
    GenerateTest,
    /// Refactor existing code
    RefactorCode,
    /// Optimize code performance
    OptimizeCode,

    // Code understanding intents
    /// Explain code functionality
    ExplainCode,
    /// Analyze code structure
    AnalyzeCode,
    /// Find bugs in code
    FindBugs,
    /// Document code
    DocumentCode,

    // Navigation intents
    /// Find a file
    FindFile,
    /// Find a symbol
    FindSymbol,
    /// Go to definition
    GotoDefinition,
    /// Find references
    FindReferences,

    // Project management intents
    /// Run tests
    RunTests,
    /// Build the project
    BuildProject,
    /// Format code
    FormatCode,
    /// Lint code
    LintCode,

    // Git operations
    /// Check git status
    GitStatus,
    /// Commit changes
    GitCommit,
    /// Show git diff
    GitDiff,
    /// Show git log
    GitLog,

    // Conversation management
    /// Start a new thread
    StartNewThread,
    /// Switch context
    SwitchContext,
    /// Clear history
    ClearHistory,

    /// Unknown or unclassifiable intent
    Unknown,
}

impl Intent {
    /// Returns all intent variants as a slice.
    #[must_use]
    pub const fn variants() -> &'static [Intent] {
        &[
            Intent::GenerateCode,
            Intent::GenerateFunction,
            Intent::GenerateClass,
            Intent::GenerateTest,
            Intent::RefactorCode,
            Intent::OptimizeCode,
            Intent::ExplainCode,
            Intent::AnalyzeCode,
            Intent::FindBugs,
            Intent::DocumentCode,
            Intent::FindFile,
            Intent::FindSymbol,
            Intent::GotoDefinition,
            Intent::FindReferences,
            Intent::RunTests,
            Intent::BuildProject,
            Intent::FormatCode,
            Intent::LintCode,
            Intent::GitStatus,
            Intent::GitCommit,
            Intent::GitDiff,
            Intent::GitLog,
            Intent::StartNewThread,
            Intent::SwitchContext,
            Intent::ClearHistory,
            Intent::Unknown,
        ]
    }

    /// Returns a human-readable label for the intent.
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Intent::GenerateCode => "Generate Code",
            Intent::GenerateFunction => "Generate Function",
            Intent::GenerateClass => "Generate Class",
            Intent::GenerateTest => "Generate Test",
            Intent::RefactorCode => "Refactor",
            Intent::OptimizeCode => "Optimize",
            Intent::ExplainCode => "Explain",
            Intent::AnalyzeCode => "Analyze",
            Intent::FindBugs => "Find Bugs",
            Intent::DocumentCode => "Document",
            Intent::FindFile => "Find File",
            Intent::FindSymbol => "Find Symbol",
            Intent::GotoDefinition => "Go to Definition",
            Intent::FindReferences => "Find References",
            Intent::RunTests => "Run Tests",
            Intent::BuildProject => "Build",
            Intent::FormatCode => "Format",
            Intent::LintCode => "Lint",
            Intent::GitStatus => "Git Status",
            Intent::GitCommit => "Git Commit",
            Intent::GitDiff => "Git Diff",
            Intent::GitLog => "Git Log",
            Intent::StartNewThread => "New Thread",
            Intent::SwitchContext => "Switch Context",
            Intent::ClearHistory => "Clear History",
            Intent::Unknown => "Unknown",
        }
    }
}

/// Result of intent classification with confidence score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentClassification {
    /// The primary detected intent
    pub intent: Intent,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Alternative intents with their confidence scores
    pub alternative_intents: Vec<(Intent, f64)>,
}

impl IntentClassification {
    /// Creates a new intent classification.
    #[must_use]
    pub fn new(intent: Intent, confidence: f64) -> Self {
        Self {
            intent,
            confidence,
            alternative_intents: Vec::new(),
        }
    }

    /// Creates a classification with alternatives.
    #[must_use]
    pub fn with_alternatives(
        intent: Intent,
        confidence: f64,
        alternatives: Vec<(Intent, f64)>,
    ) -> Self {
        Self {
            intent,
            confidence,
            alternative_intents: alternatives,
        }
    }

    /// Returns true if confidence is high (> 0.8).
    #[must_use]
    pub const fn is_high_confidence(&self) -> bool {
        self.confidence > 0.8
    }

    /// Returns true if confidence is medium (> 0.5).
    #[must_use]
    pub const fn is_medium_confidence(&self) -> bool {
        self.confidence > 0.5
    }
}

/// Extracted parameters from user input.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IntentParams {
    /// Programming language mentioned
    pub language: Option<String>,
    /// File path mentioned
    pub file_path: Option<String>,
    /// Symbol name mentioned
    pub symbol_name: Option<String>,
    /// Code snippet provided
    pub code_snippet: Option<String>,
    /// Natural language description
    pub description: Option<String>,
}

impl IntentParams {
    /// Creates empty parameters.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            language: None,
            file_path: None,
            symbol_name: None,
            code_snippet: None,
            description: None,
        }
    }

    /// Returns true if any parameter is set.
    #[must_use]
    pub fn has_any(&self) -> bool {
        self.language.is_some()
            || self.file_path.is_some()
            || self.symbol_name.is_some()
            || self.code_snippet.is_some()
            || self.description.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test fixtures for various user inputs
    pub(crate) static GENERATE_CODE_INPUTS: &[&str] = &[
        "generate a function for user authentication",
        "create a class to handle database connections",
        "write code that parses JSON data",
        "implement a REST API endpoint",
        "build a sorting algorithm",
    ];

    pub(crate) static EXPLAIN_CODE_INPUTS: &[&str] = &[
        "explain how this function works",
        "what does this code do?",
        "tell me about the algorithm used here",
        "how does the authentication flow work?",
    ];

    pub(crate) static REFACTOR_INPUTS: &[&str] = &[
        "refactor this function to be more readable",
        "improve the code structure",
        "clean up this messy code",
        "make this code better organized",
    ];

    pub(crate) static FIND_BUGS_INPUTS: &[&str] = &[
        "find bugs in my code",
        "what's wrong with this function?",
        "detect any errors in this logic",
        "spot issues in this implementation",
    ];

    pub(crate) static GIT_INPUTS: &[&str] = &[
        "show git status",
        "commit these changes",
        "what's the git diff?",
        "show me the git log",
    ];

    pub(crate) static NAVIGATION_INPUTS: &[&str] = &[
        "find the auth.rs file",
        "where is the User class defined?",
        "go to definition of authenticate",
        "find all references to this function",
    ];

    #[test]
    fn test_intent_variants() {
        let variants = Intent::variants();
        assert!(!variants.is_empty());
        assert!(variants.contains(&Intent::GenerateCode));
        assert!(variants.contains(&Intent::Unknown));
    }

    #[test]
    fn test_intent_label() {
        assert_eq!(Intent::GenerateCode.label(), "Generate Code");
        assert_eq!(Intent::FindBugs.label(), "Find Bugs");
        assert_eq!(Intent::Unknown.label(), "Unknown");
    }

    #[test]
    fn test_intent_classification() {
        let classification = IntentClassification::new(Intent::GenerateCode, 0.95);
        assert_eq!(classification.intent, Intent::GenerateCode);
        assert!(classification.is_high_confidence());
        assert!(classification.alternative_intents.is_empty());
    }

    #[test]
    fn test_intent_classification_with_alternatives() {
        let classification = IntentClassification::with_alternatives(
            Intent::GenerateCode,
            0.7,
            vec![(Intent::RefactorCode, 0.3)],
        );
        assert!(!classification.is_high_confidence());
        assert!(classification.is_medium_confidence());
        assert_eq!(classification.alternative_intents.len(), 1);
    }

    #[test]
    fn test_intent_params() {
        let params = IntentParams::new();
        assert!(!params.has_any());

        let params = IntentParams {
            language: Some("Rust".to_string()),
            ..Default::default()
        };
        assert!(params.has_any());
    }

    #[test]
    fn test_all_intents_have_labels() {
        for intent in Intent::variants() {
            let label = intent.label();
            assert!(!label.is_empty(), "Intent {:?} has empty label", intent);
            assert!(!label.contains("rust"), "Label contains debug format: {label}");
        }
    }

    #[test]
    fn test_confidence_boundaries() {
        // High confidence threshold
        let high = IntentClassification::new(Intent::GenerateCode, 0.81);
        assert!(high.is_high_confidence());

        // Just below high confidence
        let medium = IntentClassification::new(Intent::GenerateCode, 0.79);
        assert!(!medium.is_high_confidence());
        assert!(medium.is_medium_confidence());

        // Low confidence
        let low = IntentClassification::new(Intent::Unknown, 0.3);
        assert!(!low.is_high_confidence());
        assert!(!low.is_medium_confidence());
    }

    #[test]
    fn test_edge_case_confidence_values() {
        // Test boundary conditions for confidence
        assert!(
            IntentClassification::new(Intent::GenerateCode, 1.0).is_high_confidence()
        );
        // 0.0 is low confidence, not medium
        assert!(
            !IntentClassification::new(Intent::GenerateCode, 0.0).is_medium_confidence()
        );
        // 0.5 is NOT medium confidence (needs to be > 0.5)
        assert!(!IntentClassification::new(Intent::GenerateCode, 0.5)
            .is_medium_confidence());
        assert!(!IntentClassification::new(Intent::GenerateCode, 0.5)
            .is_high_confidence());
        // 0.51 IS medium confidence
        assert!(IntentClassification::new(Intent::GenerateCode, 0.51)
            .is_medium_confidence());
    }

    #[test]
    fn test_intent_params_with_multiple_fields() {
        let params = IntentParams {
            language: Some("TypeScript".to_string()),
            file_path: Some("src/auth.ts".to_string()),
            symbol_name: Some("authenticate".to_string()),
            code_snippet: Some("fn auth() {}".to_string()),
            description: Some("Create authentication".to_string()),
        };

        assert!(params.has_any());
        assert_eq!(params.language, Some("TypeScript".to_string()));
        assert_eq!(params.file_path, Some("src/auth.ts".to_string()));
    }

    #[test]
    fn test_intent_params_serialization() {
        let params = IntentParams {
            language: Some("Rust".to_string()),
            file_path: Some("main.rs".to_string()),
            ..Default::default()
        };

        let json = serde_json::to_string(&params).expect("Should serialize");
        assert!(json.contains("Rust"));
        assert!(json.contains("main.rs"));

        let deserialized: IntentParams =
            serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.language, params.language);
        assert_eq!(deserialized.file_path, params.file_path);
    }

    #[test]
    fn test_intent_classification_serialization() {
        let classification = IntentClassification::with_alternatives(
            Intent::GenerateCode,
            0.85,
            vec![(Intent::RefactorCode, 0.6), (Intent::OptimizeCode, 0.4)],
        );

        let json = serde_json::to_string(&classification).expect("Should serialize");
        assert!(json.contains("generate_code"));

        let deserialized: IntentClassification =
            serde_json::from_str(&json).expect("Should deserialize");
        assert_eq!(deserialized.intent, Intent::GenerateCode);
        assert_eq!(deserialized.confidence, 0.85);
        assert_eq!(deserialized.alternative_intents.len(), 2);
    }

    #[test]
    fn test_unknown_intent_is_valid_variant() {
        let variants = Intent::variants();
        assert!(variants.contains(&Intent::Unknown));
        assert_eq!(Intent::Unknown.label(), "Unknown");
    }

    #[test]
    fn test_all_code_generation_intents_exist() {
        let expected = [
            Intent::GenerateCode,
            Intent::GenerateFunction,
            Intent::GenerateClass,
            Intent::GenerateTest,
        ];

        for intent in expected {
            assert!(Intent::variants().contains(&intent));
        }
    }

    #[test]
    fn test_all_code_understanding_intents_exist() {
        let expected = [
            Intent::ExplainCode,
            Intent::AnalyzeCode,
            Intent::FindBugs,
            Intent::DocumentCode,
        ];

        for intent in expected {
            assert!(Intent::variants().contains(&intent));
        }
    }

    #[test]
    fn test_all_navigation_intents_exist() {
        let expected = [
            Intent::FindFile,
            Intent::FindSymbol,
            Intent::GotoDefinition,
            Intent::FindReferences,
        ];

        for intent in expected {
            assert!(Intent::variants().contains(&intent));
        }
    }

    #[test]
    fn test_all_project_management_intents_exist() {
        let expected = [
            Intent::RunTests,
            Intent::BuildProject,
            Intent::FormatCode,
            Intent::LintCode,
        ];

        for intent in expected {
            assert!(Intent::variants().contains(&intent));
        }
    }

    #[test]
    fn test_all_git_intents_exist() {
        let expected = [
            Intent::GitStatus,
            Intent::GitCommit,
            Intent::GitDiff,
            Intent::GitLog,
        ];

        for intent in expected {
            assert!(Intent::variants().contains(&intent));
        }
    }

    #[test]
    fn test_all_conversation_management_intents_exist() {
        let expected = [
            Intent::StartNewThread,
            Intent::SwitchContext,
            Intent::ClearHistory,
        ];

        for intent in expected {
            assert!(Intent::variants().contains(&intent));
        }
    }
}
