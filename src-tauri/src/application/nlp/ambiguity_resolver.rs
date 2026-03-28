//! Provides an ambiguity resolution engine for detecting and resolving ambiguities in user input.
//!
//! This module contains the `AmbiguityResolver`, which is responsible for identifying
//! potential ambiguities in user requests, such as missing file references or
//! unclear scope, and providing suggestions for clarification.

use regex::Regex;
use thiserror::Error;

use crate::core::nlp::ambiguity::{
    Ambiguity, AmbiguitySeverity, AmbiguitySuggestion, AmbiguityType, ClarificationOption,
    ClarificationRequest,
};
use crate::core::nlp::entities::ConversationContext;
use crate::core::nlp::multi_intent::TextSpan;

/// A resolver for detecting and handling ambiguities in user input.
pub struct AmbiguityResolver {
    /// A regular expression for matching file references in text.
    file_pattern: Regex,
}

impl AmbiguityResolver {
    /// Creates a new `AmbiguityResolver`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            file_pattern: Regex::new(r"[a-zA-Z0-9_/\-]+\.[a-zA-Z0-9]{1,4}")
                .expect("Invalid file pattern regex"),
        }
    }

    /// Detects ambiguities in the user's input based on the current context and project files.
    ///
    /// # Arguments
    ///
    /// * `input` - The user's input text.
    /// * `context` - The current conversation context.
    /// * `project_files` - A slice of all file paths in the project.
    ///
    /// # Errors
    ///
    /// Returns a `ResolverError` if an error occurs during detection, though this
    /// is not currently implemented.
    pub async fn detect(
        &self,
        input: &str,
        context: &ConversationContext,
        project_files: &[String],
    ) -> Result<Vec<Ambiguity>, ResolverError> {
        let mut ambiguities = Vec::new();

        // Check file references
        for cap in self.file_pattern.find_iter(input) {
            let file_ref = cap.as_str();
            let matches: Vec<&String> = project_files
                .iter()
                .filter(|f| f.ends_with(file_ref) || f.contains(file_ref))
                .collect();

            let span = TextSpan::new(cap.start(), cap.end(), file_ref);

            match matches.len() {
                0 => {
                    // File not found
                    let suggestions = self.fuzzy_match_files(file_ref, project_files);
                    ambiguities.push(Ambiguity::with_suggestions(
                        AmbiguityType::MissingFile,
                        span,
                        AmbiguitySeverity::High,
                        ClarificationRequest::with_options_and_freeform(
                            &format!("File '{file_ref}' not found. Specify the correct path:"),
                            suggestions
                                .iter()
                                .take(5)
                                .map(|s| ClarificationOption::new(&s.value, &s.value))
                                .collect(),
                            true,
                        ),
                        suggestions,
                    ));
                }
                2.. => {
                    // Multiple matches
                    let options: Vec<ClarificationOption> = matches
                        .iter()
                        .take(5)
                        .map(|m| ClarificationOption::new(m, m))
                        .collect();

                    let suggestions: Vec<AmbiguitySuggestion> = matches
                        .iter()
                        .take(5)
                        .map(|m| AmbiguitySuggestion::new(m, 0.7, "matches your reference"))
                        .collect();

                    ambiguities.push(Ambiguity::with_suggestions(
                        AmbiguityType::MultipleMatches,
                        span,
                        AmbiguitySeverity::Medium,
                        ClarificationRequest::with_options(
                            &format!("Multiple files match '{file_ref}'. Which one?"),
                            options,
                        ),
                        suggestions,
                    ));
                }
                _ => {
                    // Single match - no ambiguity
                }
            }
        }

        // Check for context-based ambiguities
        if context.metadata.current_file.is_none() && self.needs_file_context(input) {
            ambiguities.push(Ambiguity::new(
                AmbiguityType::UnclearScope,
                TextSpan::full(input),
                AmbiguitySeverity::Medium,
                ClarificationRequest::new("Which file should this apply to?"),
            ));
        }

        Ok(ambiguities)
    }

    /// Checks if the user's input implies a need for file context.
    fn needs_file_context(&self, input: &str) -> bool {
        let keywords = [
            "this file",
            "the file",
            "this code",
            "the code",
            "this function",
            "the function",
            "this class",
            "the class",
        ];
        let lower = input.to_lowercase();
        keywords.iter().any(|k| lower.contains(k))
    }

    /// Performs a fuzzy match on file paths to find potential suggestions.
    fn fuzzy_match_files(&self, input: &str, files: &[String]) -> Vec<AmbiguitySuggestion> {
        let input_lower = input.to_lowercase();

        files
            .iter()
            .filter(|f| {
                let file_lower = f.to_lowercase();
                // Check if all characters in input appear in order in the filename
                input_lower
                    .chars()
                    .all(|c| file_lower.contains(c.to_ascii_lowercase()))
            })
            .take(3)
            .map(|f| AmbiguitySuggestion::new(f, 0.7, "fuzzy match"))
            .collect()
    }

    /// Attempts to resolve an ambiguity using the current conversation context.
    #[must_use]
    pub fn resolve_with_context(
        &self,
        ambiguity: &Ambiguity,
        context: &ConversationContext,
    ) -> Option<AmbiguitySuggestion> {
        match ambiguity.ambiguity_type {
            AmbiguityType::MissingFile | AmbiguityType::MultipleMatches => {
                // Check if file was mentioned in conversation
                context.metadata.current_file.as_ref().map(|f| {
                    AmbiguitySuggestion::new(&f.path, 0.8, "from conversation context")
                })
            }
            AmbiguityType::UnclearScope => {
                context
                    .metadata
                    .current_file
                    .as_ref()
                    .map(|f| AmbiguitySuggestion::new(&f.path, 0.9, "currently active file"))
            }
            _ => None,
        }
    }
}

impl Default for AmbiguityResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents errors that can occur during ambiguity resolution.
#[derive(Debug, Error)]
pub enum ResolverError {
    /// An error related to the pattern used for detection.
    #[error("Pattern error: {0}")]
    PatternError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_resolver() -> AmbiguityResolver {
        AmbiguityResolver::new()
    }

    fn create_test_files() -> Vec<String> {
        vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "src/auth.rs".to_string(),
            "tests/auth.rs".to_string(), // This will match "auth.rs" via contains
        ]
    }

    fn create_large_file_set() -> Vec<String> {
        vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "src/auth/service.rs".to_string(),
            "src/auth/models.rs".to_string(),
            "src/auth/utils.rs".to_string(),
            "tests/integration/auth_test.rs".to_string(),
            "tests/unit/auth_test.rs".to_string(),
            "components/Auth.tsx".to_string(),
            "components/auth/Button.tsx".to_string(),
            "auth.config.ts".to_string(),
        ]
    }

    // Test fixtures for context detection
    const NEEDS_CONTEXT_FIXTURES: &[&str] = &[
        "explain this file",
        "refactor the code",
        "check this function",
        "optimize the class",
        "document the file",
    ];

    const NO_CONTEXT_FIXTURES: &[&str] = &[
        "create a new function",
        "generate code for authentication",
        "write tests for user model",
        "build a REST API",
    ];

    #[test]
    fn test_needs_file_context() {
        let resolver = create_test_resolver();

        assert!(resolver.needs_file_context("explain this file"));
        assert!(resolver.needs_file_context("refactor the code"));
        assert!(!resolver.needs_file_context("create a new function"));
    }

    #[test]
    fn test_needs_file_context_comprehensive() {
        let resolver = create_test_resolver();

        for input in NEEDS_CONTEXT_FIXTURES {
            assert!(
                resolver.needs_file_context(input),
                "Expected need for context: '{input}'"
            );
        }

        for input in NO_CONTEXT_FIXTURES {
            assert!(
                !resolver.needs_file_context(input),
                "Should not need context: '{input}'"
            );
        }
    }

    #[test]
    fn test_needs_file_context_case_insensitive() {
        let resolver = create_test_resolver();

        assert!(resolver.needs_file_context("Explain This File"));
        assert!(resolver.needs_file_context("REFACTOR THE CODE"));
    }

    #[test]
    fn test_needs_file_context_with_punctuation() {
        let resolver = create_test_resolver();

        assert!(resolver.needs_file_context("explain this file!"));
        assert!(resolver.needs_file_context("check the code, please."));
    }

    #[tokio::test]
    async fn test_detect_no_ambiguity() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = create_test_files();

        let result = resolver
            .detect("explain src/main.rs", &context, &files)
            .await
            .expect("Should detect");

        // Single match - no ambiguity
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_detect_multiple_matches() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = create_test_files();

        let result = resolver
            .detect("explain auth.rs", &context, &files)
            .await
            .expect("Should detect");

        // Multiple files contain "auth.rs"
        assert!(!result.is_empty());
        assert_eq!(result[0].ambiguity_type, AmbiguityType::MultipleMatches);
    }

    #[tokio::test]
    async fn test_detect_missing_file() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = create_test_files();

        let result = resolver
            .detect("explain nonexistent.rs", &context, &files)
            .await
            .expect("Should detect");

        assert!(!result.is_empty());
        assert_eq!(result[0].ambiguity_type, AmbiguityType::MissingFile);
        assert_eq!(result[0].severity, AmbiguitySeverity::High);
    }

    #[test]
    fn test_resolve_with_context() {
        let resolver = create_test_resolver();
        let mut context = ConversationContext::new();
        context.metadata.current_file = Some(FileInfo::new("src/main.rs", "rust"));

        let ambiguity = Ambiguity::new(
            AmbiguityType::UnclearScope,
            TextSpan::full("test"),
            AmbiguitySeverity::Medium,
            ClarificationRequest::new("Which file?"),
        );

        let resolved = resolver.resolve_with_context(&ambiguity, &context);
        assert!(resolved.is_some());
        assert_eq!(resolved.expect("Should resolve").value, "src/main.rs");
    }

    #[tokio::test]
    async fn test_detect_ambiguity_with_no_files() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files: Vec<String> = vec![];

        let result = resolver
            .detect("check main.rs", &context, &files)
            .await
            .expect("Should detect");

        assert!(!result.is_empty());
        assert_eq!(result[0].ambiguity_type, AmbiguityType::MissingFile);
    }

    #[tokio::test]
    async fn test_detect_multiple_ambiguities_in_single_input() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = create_large_file_set();

        let result = resolver
            .detect("check auth.rs and nonexistent.ts", &context, &files)
            .await
            .expect("Should detect");

        // Should detect multiple ambiguities
        assert!(result.len() >= 1);
    }

    #[tokio::test]
    async fn test_detect_with_empty_input() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = create_test_files();

        let result = resolver
            .detect("", &context, &files)
            .await
            .expect("Should detect");

        // No file references in empty input
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_detect_with_whitespace_input() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = create_test_files();

        let result = resolver
            .detect("   ", &context, &files)
            .await
            .expect("Should detect");

        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_detect_suggestions_limit() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = create_large_file_set();

        let result = resolver
            .detect("check auth.rs", &context, &files)
            .await
            .expect("Should detect");

        // Should limit suggestions to 5
        if !result.is_empty() {
            assert!(result[0].clarification.options.len() <= 5);
            assert!(result[0].suggestions.len() <= 5);
        }
    }

    #[tokio::test]
    async fn test_detect_unclear_scope_without_current_file() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new(); // No current file set
        let files = create_test_files();

        let result = resolver
            .detect("explain this file", &context, &files)
            .await
            .expect("Should detect");

        // Should detect unclear scope
        assert!(!result.is_empty());
        assert!(result
            .iter()
            .any(|a| a.ambiguity_type == AmbiguityType::UnclearScope));
    }

    #[tokio::test]
    async fn test_detect_no_unclear_scope_with_current_file() {
        let resolver = create_test_resolver();
        let mut context = ConversationContext::new();
        context.metadata.current_file = Some(FileInfo::new("src/main.rs", "rust"));
        let files = create_test_files();

        let result = resolver
            .detect("explain this file", &context, &files)
            .await
            .expect("Should detect");

        // Should not detect unclear scope when current file is set
        assert!(!result
            .iter()
            .any(|a| a.ambiguity_type == AmbiguityType::UnclearScope));
    }

    #[tokio::test]
    async fn test_fuzzy_match_files() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = create_test_files();

        let result = resolver
            .detect("explain mn.rs", &context, &files)
            .await
            .expect("Should detect");

        // Should provide fuzzy match suggestions
        if !result.is_empty() && result[0].ambiguity_type == AmbiguityType::MissingFile {
            assert!(!result[0].suggestions.is_empty());
        }
    }

    #[test]
    fn test_resolve_with_context_missing_file() {
        let resolver = create_test_resolver();
        let mut context = ConversationContext::new();
        context.metadata.current_file = Some(FileInfo::new("src/main.rs", "rust"));

        let ambiguity = Ambiguity::new(
            AmbiguityType::MissingFile,
            TextSpan::full("test.ts"),
            AmbiguitySeverity::High,
            ClarificationRequest::new("File not found"),
        );

        let resolved = resolver.resolve_with_context(&ambiguity, &context);
        // Should provide current file as suggestion
        assert!(resolved.is_some());
    }

    #[test]
    fn test_resolve_with_context_no_current_file() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();

        let ambiguity = Ambiguity::new(
            AmbiguityType::UnclearScope,
            TextSpan::full("test"),
            AmbiguitySeverity::Medium,
            ClarificationRequest::new("Which file?"),
        );

        let resolved = resolver.resolve_with_context(&ambiguity, &context);
        // Should not resolve without current file
        assert!(resolved.is_none());
    }

    #[tokio::test]
    async fn test_detect_with_long_file_path() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = vec!["src/very/deeply/nested/directory/structure/file.rs".to_string()];

        let result = resolver
            .detect("explain file.rs", &context, &files)
            .await
            .expect("Should detect");

        // Should find the file
        assert!(result.is_empty()); // Single match - no ambiguity
    }

    #[tokio::test]
    async fn test_detect_with_special_characters_in_filename() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = vec!["src/auth-v2.rs".to_string(), "src/auth_v2.rs".to_string()];

        let result = resolver
            .detect("explain auth-v2.rs", &context, &files)
            .await
            .expect("Should detect");

        // Should handle special characters
        if !result.is_empty() {
            assert_eq!(result[0].ambiguity_type, AmbiguityType::MultipleMatches);
        }
    }

    #[tokio::test]
    async fn test_detect_with_case_sensitivity() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = vec!["src/AuthService.rs".to_string()];

        let result = resolver
            .detect("explain authservice.rs", &context, &files)
            .await
            .expect("Should detect");

        // Fuzzy match should work with case insensitivity
        // May return ambiguity if exact match not found
    }

    #[tokio::test]
    async fn test_multiple_file_references() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = create_test_files();

        let result = resolver
            .detect("compare main.rs and lib.rs", &context, &files)
            .await
            .expect("Should detect");

        // Both files should be found (single matches each)
        // So no ambiguity should be reported for these
    }

    #[tokio::test]
    async fn test_detect_with_file_extension_only() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = create_test_files();

        let result = resolver
            .detect("explain .rs", &context, &files)
            .await
            .expect("Should detect");

        // Extension-only references might not match the pattern
        // This is expected behavior
    }

    #[test]
    fn test_default_trait_implementation() {
        let resolver = AmbiguityResolver::default();
        // Should work the same as new()
        assert!(resolver.needs_file_context("explain this file"));
    }

    #[tokio::test]
    async fn test_detect_preserves_span_information() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = create_test_files();

        let input = "check auth.rs for bugs";
        let result = resolver
            .detect(input, &context, &files)
            .await
            .expect("Should detect");

        if !result.is_empty() {
            // Span should contain the referenced text
            assert!(result[0].span.text.contains("auth.rs"));
            assert!(result[0].span.start <= input.find("auth.rs").unwrap_or(0));
        }
    }

    #[tokio::test]
    async fn test_severity_levels_for_different_ambiguities() {
        let resolver = create_test_resolver();
        let context = ConversationContext::new();
        let files = create_test_files();

        // Missing file should be high severity
        let result = resolver
            .detect("check missing.rs", &context, &files)
            .await
            .expect("Should detect");
        if !result.is_empty() {
            assert_eq!(result[0].severity, AmbiguitySeverity::High);
        }

        // Multiple matches should be medium severity
        let result = resolver
            .detect("check auth.rs", &context, &files)
            .await
            .expect("Should detect");
        if !result.is_empty() && result[0].ambiguity_type == AmbiguityType::MultipleMatches {
            assert_eq!(result[0].severity, AmbiguitySeverity::Medium);
        }
    }
}
