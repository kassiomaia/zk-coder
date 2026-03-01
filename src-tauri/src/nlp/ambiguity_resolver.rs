//! Ambiguity resolution engine.
//!
//! This module detects and resolves ambiguities in user input.

use regex::Regex;
use thiserror::Error;

use super::ambiguity::{
    Ambiguity, AmbiguitySeverity, AmbiguitySuggestion, AmbiguityType, ClarificationOption,
    ClarificationRequest,
};
use super::context::ConversationContext;
use super::multi_intent::TextSpan;

/// Ambiguity resolver for user input.
pub struct AmbiguityResolver {
    /// Pattern for matching file references
    file_pattern: Regex,
}

impl AmbiguityResolver {
    /// Creates a new ambiguity resolver.
    #[must_use]
    pub fn new() -> Self {
        Self {
            file_pattern: Regex::new(r"[a-zA-Z0-9_/\-]+\.[a-zA-Z0-9]{1,4}")
                .expect("Invalid file pattern regex"),
        }
    }

    /// Detects ambiguities in the input.
    ///
    /// # Errors
    ///
    /// Currently does not return errors, but reserves the right to in the future.
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
                                .map(|s| {
                                    ClarificationOption::new(&s.value, &s.value)
                                })
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
                        .map(|m| {
                            AmbiguitySuggestion::new(m, 0.7, "matches your reference")
                        })
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

    /// Checks if the input needs file context.
    fn needs_file_context(&self, input: &str) -> bool {
        let keywords = [
            "this file",
            "the file",
            "this code",
            "the code",
            "this function",
            "this class",
        ];
        let lower = input.to_lowercase();
        keywords.iter().any(|k| lower.contains(k))
    }

    /// Performs fuzzy matching on files.
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

    /// Resolves an ambiguity using context.
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
            AmbiguityType::UnclearScope => context.metadata.current_file.as_ref().map(|f| {
                AmbiguitySuggestion::new(&f.path, 0.9, "currently active file")
            }),
            _ => None,
        }
    }
}

impl Default for AmbiguityResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors from ambiguity resolution.
#[derive(Debug, Error)]
pub enum ResolverError {
    /// Pattern error
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
            "tests/auth_test.rs".to_string(),
        ]
    }

    #[test]
    fn test_needs_file_context() {
        let resolver = create_test_resolver();

        assert!(resolver.needs_file_context("explain this file"));
        assert!(resolver.needs_file_context("refactor the code"));
        assert!(!resolver.needs_file_context("create a new function"));
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
        context.metadata.current_file = Some(crate::nlp::context::FileInfo::new("src/main.rs", "rust"));

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
}
