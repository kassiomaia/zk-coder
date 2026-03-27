//! Code understanding types for analysis.
//!
//! This module defines types for understanding and analyzing code.

use serde::{Deserialize, Serialize};

/// Information extracted from code analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeUnderstanding {
    /// Detected language
    pub language: ProgrammingLanguage,
    /// Code structure
    pub structure: CodeStructure,
    /// Complexity metrics
    pub complexity: ComplexityMetrics,
}

impl CodeUnderstanding {
    /// Creates a new code understanding result.
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

/// Supported programming languages.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProgrammingLanguage {
    /// Rust
    Rust,
    /// TypeScript
    TypeScript,
    /// JavaScript
    JavaScript,
    /// Python
    Python,
    /// Unknown language
    Unknown,
}

impl ProgrammingLanguage {
    /// Returns the file extension for this language.
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

    /// Detects language from file extension.
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

    /// Returns a human-readable name.
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

/// Structure of code.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodeStructure {
    /// Functions found
    pub functions: Vec<FunctionInfo>,
    /// Classes/structs found
    pub classes: Vec<ClassInfo>,
    /// Import statements
    pub imports: Vec<String>,
}

impl CodeStructure {
    /// Returns true if no structure was found.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty() && self.classes.is_empty() && self.imports.is_empty()
    }

    /// Returns the total number of items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.functions.len() + self.classes.len() + self.imports.len()
    }
}

/// Information about a function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionInfo {
    /// Function name
    pub name: String,
    /// Parameter names (simplified)
    pub parameters: Vec<String>,
    /// Return type if known
    pub return_type: Option<String>,
    /// Start line (0-indexed)
    pub line_start: usize,
    /// End line (0-indexed)
    pub line_end: usize,
}

impl FunctionInfo {
    /// Creates a new function info.
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

    /// Returns the number of lines.
    #[must_use]
    pub const fn line_count(&self) -> usize {
        self.line_end.saturating_sub(self.line_start) + 1
    }
}

/// Information about a class/struct.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassInfo {
    /// Class name
    pub name: String,
    /// Methods
    pub methods: Vec<FunctionInfo>,
    /// Start line (0-indexed)
    pub line_start: usize,
    /// End line (0-indexed)
    pub line_end: usize,
}

impl ClassInfo {
    /// Creates a new class info.
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

/// Complexity metrics for code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Cyclomatic complexity
    pub cyclomatic: usize,
    /// Lines of code
    pub lines: usize,
    /// Maximum nesting depth
    pub nesting: usize,
}

impl ComplexityMetrics {
    /// Creates a new complexity metrics.
    #[must_use]
    pub const fn new(cyclomatic: usize, lines: usize, nesting: usize) -> Self {
        Self {
            cyclomatic,
            lines,
            nesting,
        }
    }

    /// Returns a complexity rating.
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

/// Complexity rating levels.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComplexityRating {
    /// Low complexity (1-5)
    Low,
    /// Medium complexity (6-15)
    Medium,
    /// High complexity (16-25)
    High,
    /// Very high complexity (26+)
    VeryHigh,
}

impl ComplexityRating {
    /// Returns a human-readable label.
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
