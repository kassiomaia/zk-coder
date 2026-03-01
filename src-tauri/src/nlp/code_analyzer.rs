//! Code analysis engine using tree-sitter.
//!
//! This module provides code parsing and analysis using tree-sitter.

use std::collections::HashMap;
use std::sync::Mutex;

use thiserror::Error;
use tree_sitter::{Node, Parser};

use super::code_understanding::{
    ClassInfo, CodeStructure, CodeUnderstanding, ComplexityMetrics, FunctionInfo,
    ProgrammingLanguage,
};

/// Code analyzer using tree-sitter.
pub struct CodeAnalyzer {
    /// Parsers for each language (Mutex for thread safety)
    parsers: HashMap<ProgrammingLanguage, Mutex<Parser>>,
}

impl CodeAnalyzer {
    /// Creates a new code analyzer.
    ///
    /// # Errors
    ///
    /// Returns an error if parser initialization fails.
    pub fn new() -> Result<Self, AnalyzerError> {
        let mut parsers = HashMap::new();

        // Initialize Rust parser
        let mut rust_parser = Parser::new();
        rust_parser
            .set_language(&tree_sitter_rust::language())
            .map_err(|e| AnalyzerError::ParserInit(format!("Rust: {e}")))?;
        parsers.insert(ProgrammingLanguage::Rust, Mutex::new(rust_parser));

        // Initialize TypeScript parser
        let mut ts_parser = Parser::new();
        ts_parser
            .set_language(&tree_sitter_typescript::language_typescript())
            .map_err(|e| AnalyzerError::ParserInit(format!("TypeScript: {e}")))?;
        parsers.insert(ProgrammingLanguage::TypeScript, Mutex::new(ts_parser));

        // Initialize JavaScript parser (using TypeScript parser)
        let mut js_parser = Parser::new();
        js_parser
            .set_language(&tree_sitter_typescript::language_typescript())
            .map_err(|e| AnalyzerError::ParserInit(format!("JavaScript: {e}")))?;
        parsers.insert(ProgrammingLanguage::JavaScript, Mutex::new(js_parser));

        // Initialize Python parser
        let mut python_parser = Parser::new();
        python_parser
            .set_language(&tree_sitter_python::language())
            .map_err(|e| AnalyzerError::ParserInit(format!("Python: {e}")))?;
        parsers.insert(ProgrammingLanguage::Python, Mutex::new(python_parser));

        Ok(Self { parsers })
    }

    /// Analyzes code and returns understanding.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails.
    pub fn analyze(
        &self,
        code: &str,
        language: ProgrammingLanguage,
    ) -> Result<CodeUnderstanding, AnalyzerError> {
        let parser_mutex = self
            .parsers
            .get(&language)
            .ok_or_else(|| AnalyzerError::UnsupportedLanguage(language.name().to_string()))?;

        let mut parser = parser_mutex.lock().map_err(|_| {
            AnalyzerError::ParseError("Failed to acquire parser lock".to_string())
        })?;
        let tree = parser
            .parse(code, None)
            .ok_or_else(|| AnalyzerError::ParseError("Failed to parse code".to_string()))?;

        let root = tree.root_node();

        let structure = self.extract_structure(&root, code, language)?;
        let complexity = self.calculate_complexity(&root, code);

        Ok(CodeUnderstanding::new(language, structure, complexity))
    }

    fn extract_structure(
        &self,
        root: &Node,
        source: &str,
        language: ProgrammingLanguage,
    ) -> Result<CodeStructure, AnalyzerError> {
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();

        Self::traverse(root, &mut |node| {
            match language {
                ProgrammingLanguage::Rust => {
                    match node.kind() {
                        "function_item" => {
                            if let Some(func) = Self::extract_rust_function(node, source) {
                                functions.push(func);
                            }
                        }
                        "struct_item" | "impl_item" => {
                            if let Some(class) = Self::extract_rust_class(node, source) {
                                classes.push(class);
                            }
                        }
                        "use_declaration" => {
                            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                                imports.push(text.to_string());
                            }
                        }
                        _ => {}
                    }
                }
                ProgrammingLanguage::TypeScript | ProgrammingLanguage::JavaScript => {
                    match node.kind() {
                        "function_declaration" | "arrow_function" | "function_expression" => {
                            if let Some(func) = Self::extract_ts_function(node, source) {
                                functions.push(func);
                            }
                        }
                        "class_declaration" | "interface_declaration" => {
                            if let Some(class) = Self::extract_ts_class(node, source) {
                                classes.push(class);
                            }
                        }
                        "import_statement" => {
                            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                                imports.push(text.to_string());
                            }
                        }
                        _ => {}
                    }
                }
                ProgrammingLanguage::Python => {
                    match node.kind() {
                        "function_definition" => {
                            if let Some(func) = Self::extract_python_function(node, source) {
                                functions.push(func);
                            }
                        }
                        "class_definition" => {
                            if let Some(class) = Self::extract_python_class(node, source) {
                                classes.push(class);
                            }
                        }
                        "import_statement" | "import_from_statement" => {
                            if let Ok(text) = node.utf8_text(source.as_bytes()) {
                                imports.push(text.to_string());
                            }
                        }
                        _ => {}
                    }
                }
                ProgrammingLanguage::Unknown => {}
            }
        });

        Ok(CodeStructure {
            functions,
            classes,
            imports,
        })
    }

    fn extract_rust_function(node: Node, source: &str) -> Option<FunctionInfo> {
        let name_node = node.child_by_field_name("name")?;
        let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();

        Some(FunctionInfo::new(
            &name,
            node.start_position().row,
            node.end_position().row,
        ))
    }

    fn extract_rust_class(node: Node, source: &str) -> Option<ClassInfo> {
        let name_node = node.child_by_field_name("name")?;
        let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();

        Some(ClassInfo::new(
            &name,
            node.start_position().row,
            node.end_position().row,
        ))
    }

    fn extract_ts_function(node: Node, source: &str) -> Option<FunctionInfo> {
        let name_node = node.child_by_field_name("name")?;
        let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();

        Some(FunctionInfo::new(
            &name,
            node.start_position().row,
            node.end_position().row,
        ))
    }

    fn extract_ts_class(node: Node, source: &str) -> Option<ClassInfo> {
        let name_node = node.child_by_field_name("name")?;
        let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();

        Some(ClassInfo::new(
            &name,
            node.start_position().row,
            node.end_position().row,
        ))
    }

    fn extract_python_function(node: Node, source: &str) -> Option<FunctionInfo> {
        let name_node = node.child_by_field_name("name")?;
        let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();

        Some(FunctionInfo::new(
            &name,
            node.start_position().row,
            node.end_position().row,
        ))
    }

    fn extract_python_class(node: Node, source: &str) -> Option<ClassInfo> {
        let name_node = node.child_by_field_name("name")?;
        let name = name_node.utf8_text(source.as_bytes()).ok()?.to_string();

        Some(ClassInfo::new(
            &name,
            node.start_position().row,
            node.end_position().row,
        ))
    }

    fn calculate_complexity(&self, root: &Node, source: &str) -> ComplexityMetrics {
        let mut cyclomatic = 1;
        let mut max_nesting = 0;

        Self::traverse_with_depth(root, &mut |node, depth| {
            match node.kind() {
                "if_statement"
                | "if_expression"
                | "while_statement"
                | "for_statement"
                | "match_expression"
                | "match_statement"
                | "if"
                | "for_in_clause"
                | "try_statement"
                | "catch_clause" => {
                    cyclomatic += 1;
                }
                "&&" | "||" | "and" | "or" => {
                    cyclomatic += 1;
                }
                _ => {}
            }
            max_nesting = max_nesting.max(depth);
        });

        ComplexityMetrics::new(cyclomatic, source.lines().count(), max_nesting)
    }

    fn traverse<F>(node: &Node, f: &mut F)
    where
        F: FnMut(Node),
    {
        f(node.clone());
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                Self::traverse(&child, f);
            }
        }
    }

    fn traverse_with_depth<F>(node: &Node, f: &mut F)
    where
        F: FnMut(Node, usize),
    {
        Self::traverse_with_depth_impl(node, f, 0);
    }

    fn traverse_with_depth_impl<F>(node: &Node, f: &mut F, depth: usize)
    where
        F: FnMut(Node, usize),
    {
        f(node.clone(), depth);
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                Self::traverse_with_depth_impl(&child, f, depth + 1);
            }
        }
    }
}

impl Default for CodeAnalyzer {
    fn default() -> Self {
        Self::new().expect("Failed to initialize code analyzer")
    }
}

/// Errors from code analysis.
#[derive(Debug, Error)]
pub enum AnalyzerError {
    /// Unsupported language
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
    /// Parse error
    #[error("Parse error: {0}")]
    ParseError(String),
    /// Parser initialization error
    #[error("Parser initialization failed: {0}")]
    ParserInit(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let analyzer = CodeAnalyzer::new();
        assert!(analyzer.is_ok());
    }

    #[test]
    fn test_analyze_rust_code() {
        let analyzer = CodeAnalyzer::new().expect("Failed to create analyzer");
        let code = r#"
fn main() {
    println!("Hello, world!");
}

struct User {
    name: String,
}
"#;

        let result = analyzer.analyze(code, ProgrammingLanguage::Rust);
        assert!(result.is_ok());

        let understanding = result.expect("Should analyze");
        assert_eq!(understanding.language, ProgrammingLanguage::Rust);
        assert!(!understanding.structure.functions.is_empty());
        assert!(!understanding.structure.classes.is_empty());
    }

    #[test]
    fn test_analyze_typescript_code() {
        let analyzer = CodeAnalyzer::new().expect("Failed to create analyzer");
        let code = r#"
function greet(name: string): void {
    console.log(`Hello, ${name}!`);
}

class User {
    name: string;
}
"#;

        let result = analyzer.analyze(code, ProgrammingLanguage::TypeScript);
        assert!(result.is_ok());

        let understanding = result.expect("Should analyze");
        assert_eq!(understanding.language, ProgrammingLanguage::TypeScript);
        assert!(!understanding.structure.functions.is_empty());
    }

    #[test]
    fn test_unsupported_language() {
        let analyzer = CodeAnalyzer::new().expect("Failed to create analyzer");
        let result = analyzer.analyze("code", ProgrammingLanguage::Unknown);
        assert!(result.is_err());
    }
}
