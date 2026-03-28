//! Defines types for understanding and analyzing the structure of code.
//!
//! This module provides the data structures needed to represent the results of
//! code analysis, including information about the programming language, code
//! structure, and complexity metrics.

use serde::{Deserialize, Serialize};

/// Represents the comprehensive result of a code analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeUnderstanding {
    /// The programming language that was detected.
    pub language: ProgrammingLanguage,
    /// The structural elements of the code, such as functions and classes.
    pub structure: CodeStructure,
    /// The complexity metrics calculated for the code.
    pub complexity: ComplexityMetrics,
}

impl CodeUnderstanding {
    /// Creates a new `CodeUnderstanding` result.
    #[must_use]
    pub fn new(
        language: ProgrammingLanguage,
        structure: CodeStructure,
        complexity: ComplexityMetrics,
    ) -> Self {
        Self {
            language,
            structure,
            complexity,
        }
    }
}

/// An enum representing the supported programming languages.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProgrammingLanguage {
    /// The Rust programming language.
    Rust,
    /// The TypeScript programming language.
    TypeScript,
    /// The JavaScript programming language.
    JavaScript,
    /// The Python programming language.
    Python,
    /// An unknown or unsupported programming language.
    Unknown,
}

impl ProgrammingLanguage {
    /// Returns the typical file extension for this language.
    #[must_use]
    pub const fn extension(&self) -> &'static str {
        match self {
            ProgrammingLanguage::Rust => "rs",
            ProgrammingLanguage::TypeScript => "ts",
            ProgrammingLanguage::JavaScript => "js",
            ProgrammingLanguage::Python => "py",
            ProgrammingLanguage::Unknown => "",
        }
    }

    /// Detects the programming language from a file extension.
    #[must_use]
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => ProgrammingLanguage::Rust,
            "ts" | "tsx" => ProgrammingLanguage::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => ProgrammingLanguage::JavaScript,
            "py" | "pyw" => ProgrammingLanguage::Python,
            _ => ProgrammingLanguage::Unknown,
        }
    }

    /// Returns a human-readable name for the programming language.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            ProgrammingLanguage::Rust => "Rust",
            ProgrammingLanguage::TypeScript => "TypeScript",
            ProgrammingLanguage::JavaScript => "JavaScript",
            ProgrammingLanguage::Python => "Python",
            ProgrammingLanguage::Unknown => "Unknown",
        }
    }
}

/// Represents the structural elements of a piece of code.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeStructure {
    /// A list of functions found in the code.
    pub functions: Vec<FunctionInfo>,
    /// A list of classes or structs found in the code.
    pub classes: Vec<ClassInfo>,
    /// A list of import statements found in the code.
    pub imports: Vec<String>,
}

impl CodeStructure {
    /// Returns `true` if no structural elements were found.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty() && self.classes.is_empty() && self.imports.is_empty()
    }

    /// Returns the total number of structural items found.
    #[must_use]
    pub fn len(&self) -> usize {
        self.functions.len() + self.classes.len() + self.imports.len()
    }
}

/// Information about a function found in the code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    /// The name of the function.
    pub name: String,
    /// A simplified list of the function's parameter names.
    pub parameters: Vec<String>,
    /// The return type of the function, if known.
    pub return_type: Option<String>,
    /// The starting line number of the function (0-indexed).
    pub line_start: usize,
    /// The ending line number of the function (0-indexed).
    pub line_end: usize,
}

impl FunctionInfo {
    /// Creates a new `FunctionInfo`.
    #[must_use]
    pub fn new(name: &str, line_start: usize, line_end: usize) -> Self {
        Self {
            name: name.to_string(),
            parameters: Vec::new(),
            return_type: None,
            line_start,
            line_end,
        }
    }

    /// Returns the number of lines in the function.
    #[must_use]
    pub const fn line_count(&self) -> usize {
        self.line_end.saturating_sub(self.line_start) + 1
    }
}

/// Information about a class or struct found in the code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    /// The name of the class or struct.
    pub name: String,
    /// A list of methods within the class or struct.
    pub methods: Vec<FunctionInfo>,
    /// The starting line number of the class or struct (0-indexed).
    pub line_start: usize,
    /// The ending line number of the class or struct (0-indexed).
    pub line_end: usize,
}

impl ClassInfo {
    /// Creates a new `ClassInfo`.
    #[must_use]
    pub fn new(name: &str, line_start: usize, line_end: usize) -> Self {
        Self {
            name: name.to_string(),
            methods: Vec::new(),
            line_start,
            line_end,
        }
    }
}

/// A set of complexity metrics for a piece of code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// The cyclomatic complexity score.
    pub cyclomatic: usize,
    /// The number of lines of code.
    pub lines: usize,
    /// The maximum nesting depth.
    pub nesting: usize,
}

impl ComplexityMetrics {
    /// Creates a new `ComplexityMetrics`.
    #[must_use]
    pub const fn new(cyclomatic: usize, lines: usize, nesting: usize) -> Self {
        Self {
            cyclomatic,
            lines,
            nesting,
        }
    }

    /// Returns a `ComplexityRating` based on the cyclomatic complexity.
    #[must_use]
    pub const fn rating(&self) -> ComplexityRating {
        if self.cyclomatic <= 5 {
            ComplexityRating::Low
        } else if self.cyclomatic <= 15 {
            ComplexityRating::Medium
        } else if self.cyclomatic <= 25 {
            ComplexityRating::High
        } else {
            ComplexityRating::VeryHigh
        }
    }
}

impl Default for ComplexityMetrics {
    fn default() -> Self {
        Self::new(1, 0, 0)
    }
}

/// The different levels of complexity rating.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComplexityRating {
    /// Low complexity (cyclomatic score of 1-5).
    Low,
    /// Medium complexity (cyclomatic score of 6-15).
    Medium,
    /// High complexity (cyclomatic score of 16-25).
    High,
    /// Very high complexity (cyclomatic score of 26+).
    VeryHigh,
}

impl ComplexityRating {
    /// Returns a human-readable label for the complexity rating.
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            ComplexityRating::Low => "Low",
            ComplexityRating::Medium => "Medium",
            ComplexityRating::High => "High",
            ComplexityRating::VeryHigh => "Very High",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_programming_language_from_extension() {
        assert_eq!(
            ProgrammingLanguage::from_extension("rs"),
            ProgrammingLanguage::Rust
        );
        assert_eq!(
            ProgrammingLanguage::from_extension("ts"),
            ProgrammingLanguage::TypeScript
        );
        assert_eq!(
            ProgrammingLanguage::from_extension("JS"),
            ProgrammingLanguage::JavaScript
        );
        assert_eq!(
            ProgrammingLanguage::from_extension("unknown"),
            ProgrammingLanguage::Unknown
        );
    }

    #[test]
    fn test_programming_language_extension() {
        assert_eq!(ProgrammingLanguage::Rust.extension(), "rs");
        assert_eq!(ProgrammingLanguage::Python.extension(), "py");
    }

    #[test]
    fn test_function_info() {
        let func = FunctionInfo::new("my_function", 10, 20);
        assert_eq!(func.name, "my_function");
        assert_eq!(func.line_count(), 11);
    }

    #[test]
    fn test_complexity_metrics_rating() {
        let low = ComplexityMetrics::new(3, 100, 2);
        assert_eq!(low.rating(), ComplexityRating::Low);

        let medium = ComplexityMetrics::new(10, 100, 3);
        assert_eq!(medium.rating(), ComplexityRating::Medium);

        let high = ComplexityMetrics::new(20, 100, 5);
        assert_eq!(high.rating(), ComplexityRating::High);

        let very_high = ComplexityMetrics::new(30, 100, 7);
        assert_eq!(very_high.rating(), ComplexityRating::VeryHigh);
    }

    #[test]
    fn test_code_structure() {
        let structure = CodeStructure::default();
        assert!(structure.is_empty());
        assert_eq!(structure.len(), 0);
    }
}
