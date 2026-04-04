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
    pub const fn variants() -> &'static [Self] {
        &[
            Self::GenerateCode,
            Self::GenerateFunction,
            Self::GenerateClass,
            Self::GenerateTest,
            Self::RefactorCode,
            Self::OptimizeCode,
            Self::ExplainCode,
            Self::AnalyzeCode,
            Self::FindBugs,
            Self::DocumentCode,
            Self::FindFile,
            Self::FindSymbol,
            Self::GotoDefinition,
            Self::FindReferences,
            Self::RunTests,
            Self::BuildProject,
            Self::FormatCode,
            Self::LintCode,
            Self::GitStatus,
            Self::GitCommit,
            Self::GitDiff,
            Self::GitLog,
            Self::StartNewThread,
            Self::SwitchContext,
            Self::ClearHistory,
            Self::Unknown,
        ]
    }

    /// Returns a human-readable label for the intent.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::GenerateCode => "Generate Code",
            Self::GenerateFunction => "Generate Function",
            Self::GenerateClass => "Generate Class",
            Self::GenerateTest => "Generate Test",
            Self::RefactorCode => "Refactor",
            Self::OptimizeCode => "Optimize",
            Self::ExplainCode => "Explain",
            Self::AnalyzeCode => "Analyze",
            Self::FindBugs => "Find Bugs",
            Self::DocumentCode => "Document",
            Self::FindFile => "Find File",
            Self::FindSymbol => "Find Symbol",
            Self::GotoDefinition => "Go to Definition",
            Self::FindReferences => "Find References",
            Self::RunTests => "Run Tests",
            Self::BuildProject => "Build",
            Self::FormatCode => "Format",
            Self::LintCode => "Lint",
            Self::GitStatus => "Git Status",
            Self::GitCommit => "Git Commit",
            Self::GitDiff => "Git Diff",
            Self::GitLog => "Git Log",
            Self::StartNewThread => "New Thread",
            Self::SwitchContext => "Switch Context",
            Self::ClearHistory => "Clear History",
            Self::Unknown => "Unknown",
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
    pub const fn new(intent: Intent, confidence: f64) -> Self {
        Self {
            intent,
            confidence,
            alternative_intents: Vec::new(),
        }
    }

    /// Creates a classification with alternatives.
    #[must_use]
    pub const fn with_alternatives(
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
    pub const fn has_any(&self) -> bool {
        self.language.is_some()
            || self.file_path.is_some()
            || self.symbol_name.is_some()
            || self.code_snippet.is_some()
            || self.description.is_some()
    }
}
