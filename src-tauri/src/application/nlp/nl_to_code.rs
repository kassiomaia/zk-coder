//! Natural language to code concept mapping.
//!
//! This module maps natural language descriptions to code concepts
//! like data types, design patterns, and algorithms.

use serde::{Deserialize, Serialize};

use crate::core::nlp::code_understanding::ProgrammingLanguage;

/// Natural language to code mapper.
pub struct NLToCodeMapper {
    /// Concept mappings
    mappings: Vec<ConceptMapping>,
}

impl NLToCodeMapper {
    /// Creates a new mapper with built-in mappings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            mappings: Self::build_mappings(),
        }
    }

    fn build_mappings() -> Vec<ConceptMapping> {
        vec![
            // Data types
            ConceptMapping {
                keywords: vec!["list", "array", "sequence"],
                concept: CodeConcept::DataType("array".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["map", "dictionary", "hash", "object"],
                concept: CodeConcept::DataType("object".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["set", "unique collection"],
                concept: CodeConcept::DataType("set".to_string()),
                confidence: 0.85,
            },
            ConceptMapping {
                keywords: vec!["string", "text"],
                concept: CodeConcept::DataType("string".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["number", "integer", "float"],
                concept: CodeConcept::DataType("number".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["boolean", "bool", "flag"],
                concept: CodeConcept::DataType("boolean".to_string()),
                confidence: 0.9,
            },
            // Design patterns
            ConceptMapping {
                keywords: vec!["singleton", "single instance"],
                concept: CodeConcept::DesignPattern("Singleton".to_string()),
                confidence: 0.95,
            },
            ConceptMapping {
                keywords: vec!["factory", "creator"],
                concept: CodeConcept::DesignPattern("Factory".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["observer", "subscribe", "event listener"],
                concept: CodeConcept::DesignPattern("Observer".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["builder", "fluent", "chain"],
                concept: CodeConcept::DesignPattern("Builder".to_string()),
                confidence: 0.85,
            },
            ConceptMapping {
                keywords: vec!["adapter", "wrapper"],
                concept: CodeConcept::DesignPattern("Adapter".to_string()),
                confidence: 0.85,
            },
            ConceptMapping {
                keywords: vec!["strategy", " interchangeable algorithm"],
                concept: CodeConcept::DesignPattern("Strategy".to_string()),
                confidence: 0.85,
            },
            // Algorithms
            ConceptMapping {
                keywords: vec!["sort", "order", "arrange"],
                concept: CodeConcept::Algorithm("sort".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["search", "find", "lookup"],
                concept: CodeConcept::Algorithm("search".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["filter", "where", "select matching"],
                concept: CodeConcept::Algorithm("filter".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["map", "transform", "convert each"],
                concept: CodeConcept::Algorithm("map".to_string()),
                confidence: 0.85,
            },
            ConceptMapping {
                keywords: vec!["reduce", "aggregate", "fold"],
                concept: CodeConcept::Algorithm("reduce".to_string()),
                confidence: 0.85,
            },
            ConceptMapping {
                keywords: vec!["iterate", "loop", "traverse"],
                concept: CodeConcept::Algorithm("iterate".to_string()),
                confidence: 0.8,
            },
            // Frameworks and libraries
            ConceptMapping {
                keywords: vec!["react", "component", "jsx"],
                concept: CodeConcept::Framework("React".to_string()),
                confidence: 0.85,
            },
            ConceptMapping {
                keywords: vec!["svelte", "reactive", "store"],
                concept: CodeConcept::Framework("Svelte".to_string()),
                confidence: 0.85,
            },
            ConceptMapping {
                keywords: vec!["vue", "vuejs"],
                concept: CodeConcept::Framework("Vue".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["express", "expressjs", "node server"],
                concept: CodeConcept::Framework("Express".to_string()),
                confidence: 0.85,
            },
            ConceptMapping {
                keywords: vec!["tokio", "async rust", "rust async"],
                concept: CodeConcept::Framework("Tokio".to_string()),
                confidence: 0.9,
            },
            ConceptMapping {
                keywords: vec!["tauri", "desktop app"],
                concept: CodeConcept::Framework("Tauri".to_string()),
                confidence: 0.9,
            },
        ]
    }

    /// Extracts code concepts from a description.
    #[must_use]
    pub fn extract_concepts(&self, description: &str) -> Vec<(CodeConcept, f64)> {
        let mut found = Vec::new();
        let desc_lower = description.to_lowercase();

        for mapping in &self.mappings {
            for keyword in &mapping.keywords {
                if desc_lower.contains(keyword) {
                    found.push((mapping.concept.clone(), mapping.confidence));
                    break;
                }
            }
        }

        // Sort by confidence descending
        found.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        found
    }

    /// Suggests a programming language based on concepts.
    #[must_use]
    pub fn suggest_language(&self, description: &str) -> Option<ProgrammingLanguage> {
        let desc_lower = description.to_lowercase();

        if desc_lower.contains("rust") || desc_lower.contains("cargo") {
            return Some(ProgrammingLanguage::Rust);
        }
        if desc_lower.contains("typescript") || desc_lower.contains("ts file") {
            return Some(ProgrammingLanguage::TypeScript);
        }
        if desc_lower.contains("javascript")
            || desc_lower.contains("js file")
            || desc_lower.contains("node")
        {
            return Some(ProgrammingLanguage::JavaScript);
        }
        if desc_lower.contains("python") || desc_lower.contains("py file") {
            return Some(ProgrammingLanguage::Python);
        }

        // Infer from framework
        let concepts = self.extract_concepts(description);
        for (concept, _) in concepts {
            if let CodeConcept::Framework(fw) = &concept {
                match fw.as_str() {
                    "React" | "Vue" | "Svelte" | "Express" => {
                        return Some(ProgrammingLanguage::JavaScript);
                    }
                    "Tokio" | "Tauri" => {
                        return Some(ProgrammingLanguage::Rust);
                    }
                    _ => {}
                }
            }
        }

        None
    }
}

impl Default for NLToCodeMapper {
    fn default() -> Self {
        Self::new()
    }
}

/// A concept mapping.
#[derive(Debug, Clone)]
struct ConceptMapping {
    /// Keywords that trigger this mapping
    keywords: Vec<&'static str>,
    /// The code concept
    concept: CodeConcept,
    /// Confidence when matched
    confidence: f64,
}

/// A code concept extracted from natural language.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CodeConcept {
    /// A data type (array, object, etc.)
    DataType(String),
    /// A design pattern (Singleton, Factory, etc.)
    DesignPattern(String),
    /// An algorithm (sort, search, etc.)
    Algorithm(String),
    /// A framework or library (React, Svelte, etc.)
    Framework(String),
}

impl CodeConcept {
    /// Returns a human-readable label.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::DataType(s) => s,
            Self::DesignPattern(s) => s,
            Self::Algorithm(s) => s,
            Self::Framework(s) => s,
        }
    }

    /// Returns the concept type as a string.
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::DataType(_) => "Data Type",
            Self::DesignPattern(_) => "Design Pattern",
            Self::Algorithm(_) => "Algorithm",
            Self::Framework(_) => "Framework",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_data_types() {
        let mapper = NLToCodeMapper::new();

        let concepts = mapper.extract_concepts("create an array of strings");
        assert!(!concepts.is_empty());
        assert!(concepts
            .iter()
            .any(|(c, _)| matches!(c, CodeConcept::DataType(_))));
    }

    #[test]
    fn test_extract_design_patterns() {
        let mapper = NLToCodeMapper::new();

        let concepts = mapper.extract_concepts("implement a singleton pattern");
        assert!(!concepts.is_empty());
        assert!(concepts
            .iter()
            .any(|(c, _)| matches!(c, CodeConcept::DesignPattern(p) if p == "Singleton")));
    }

    #[test]
    fn test_extract_algorithms() {
        let mapper = NLToCodeMapper::new();

        let concepts = mapper.extract_concepts("sort the list");
        assert!(!concepts.is_empty());
        assert!(concepts
            .iter()
            .any(|(c, _)| matches!(c, CodeConcept::Algorithm(a) if a == "sort")));
    }

    #[test]
    fn test_extract_frameworks() {
        let mapper = NLToCodeMapper::new();

        let concepts = mapper.extract_concepts("create a react component");
        assert!(!concepts.is_empty());
        assert!(concepts
            .iter()
            .any(|(c, _)| matches!(c, CodeConcept::Framework(f) if f == "React")));
    }

    #[test]
    fn test_suggest_language() {
        let mapper = NLToCodeMapper::new();

        assert_eq!(
            mapper.suggest_language("write a rust function"),
            Some(ProgrammingLanguage::Rust)
        );
        assert_eq!(
            mapper.suggest_language("create a typescript interface"),
            Some(ProgrammingLanguage::TypeScript)
        );
        assert_eq!(
            mapper.suggest_language("python script"),
            Some(ProgrammingLanguage::Python)
        );
    }

    #[test]
    fn test_suggest_language_from_framework() {
        let mapper = NLToCodeMapper::new();

        assert_eq!(
            mapper.suggest_language("create a tauri app"),
            Some(ProgrammingLanguage::Rust)
        );
    }
}
