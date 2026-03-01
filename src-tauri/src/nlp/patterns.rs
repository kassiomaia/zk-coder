//! Pattern matching for fast-path intent classification.
//!
//! This module provides regex-based pattern matching to quickly classify
//! common intent patterns before falling back to AI classification.

use regex::Regex;

use super::intent::Intent;

/// Pattern matcher for intent recognition.
pub struct IntentPatterns {
    /// List of (intent, patterns) pairs
    patterns: Vec<(Intent, Vec<Regex>)>,
}

impl IntentPatterns {
    /// Creates a new pattern matcher with built-in patterns.
    #[must_use]
    pub fn new() -> Self {
        Self {
            patterns: Self::build_patterns(),
        }
    }

    fn build_patterns() -> Vec<(Intent, Vec<Regex>)> {
        vec![
            // Code generation
            (
                Intent::GenerateCode,
                vec![
                    Regex::new(r"(?i)^(generate|create|write)\s+(code|a\s+function|a\s+class)").unwrap(),
                    Regex::new(r"(?i)^(help\s+me\s+)?(code|implement|build)\s+").unwrap(),
                    Regex::new(r"(?i)^make\s+(a\s+)?(function|class|component)").unwrap(),
                ],
            ),
            (
                Intent::GenerateFunction,
                vec![
                    Regex::new(r"(?i)^(generate|create|write)\s+(a\s+)?function").unwrap(),
                    Regex::new(r"(?i)^function\s+for\s+").unwrap(),
                ],
            ),
            (
                Intent::GenerateClass,
                vec![
                    Regex::new(r"(?i)^(generate|create|write)\s+(a\s+)?class").unwrap(),
                    Regex::new(r"(?i)^class\s+for\s+").unwrap(),
                ],
            ),
            (
                Intent::GenerateTest,
                vec![
                    Regex::new(r"(?i)^(generate|create|write)\s+(tests?|unit\s+tests?)").unwrap(),
                    Regex::new(r"(?i)^test\s+for\s+").unwrap(),
                ],
            ),
            (
                Intent::RefactorCode,
                vec![
                    Regex::new(r"(?i)^(refactor|improve|clean\s+up)\s+").unwrap(),
                    Regex::new(r"(?i)make\s+(this|the)\s+code\s+(better|cleaner)").unwrap(),
                ],
            ),
            (
                Intent::OptimizeCode,
                vec![
                    Regex::new(r"(?i)^(optimize|speed\s+up|make\s+faster)\s+").unwrap(),
                    Regex::new(r"(?i)improve\s+performance\s+of\s+").unwrap(),
                ],
            ),
            // Code understanding
            (
                Intent::ExplainCode,
                vec![
                    Regex::new(r"(?i)^(explain|what\s+does|how\s+does)\s+").unwrap(),
                    Regex::new(r"(?i)^tell\s+me\s+about\s+").unwrap(),
                    Regex::new(r"(?i)^what\s+is\s+").unwrap(),
                ],
            ),
            (
                Intent::AnalyzeCode,
                vec![
                    Regex::new(r"(?i)^(analyze|review|check)\s+(this\s+)?(code|file)").unwrap(),
                    Regex::new(r"(?i)^code\s+review\s+for\s+").unwrap(),
                ],
            ),
            (
                Intent::FindBugs,
                vec![
                    Regex::new(r"(?i)(find|detect|spot)\s+(bugs?|issues?|errors?)").unwrap(),
                    Regex::new(r"(?i)^what'?s\s+wrong\s+(with|in)\s+").unwrap(),
                    Regex::new(r"(?i)^bug\s+in\s+").unwrap(),
                ],
            ),
            (
                Intent::DocumentCode,
                vec![
                    Regex::new(r"(?i)^(document|add\s+docs?|add\s+comments?)\s+").unwrap(),
                    Regex::new(r"(?i)^generate\s+documentation\s+for\s+").unwrap(),
                ],
            ),
            // Navigation
            (
                Intent::FindFile,
                vec![
                    Regex::new(r"(?i)^(find|locate|open)\s+(a\s+)?file").unwrap(),
                    Regex::new(r"(?i)^where\s+is\s+").unwrap(),
                    Regex::new(r"(?i)^file\s+named\s+").unwrap(),
                ],
            ),
            (
                Intent::FindSymbol,
                vec![
                    Regex::new(r"(?i)^(find|locate)\s+(a\s+)?(function|class|method|symbol)").unwrap(),
                    Regex::new(r"(?i)^where\s+(is|are)\s+(the\s+)?(function|class|method)").unwrap(),
                ],
            ),
            (
                Intent::GotoDefinition,
                vec![
                    Regex::new(r"(?i)^go\s*to\s+definition\s+of\s+").unwrap(),
                    Regex::new(r"(?i)^definition\s+of\s+").unwrap(),
                ],
            ),
            (
                Intent::FindReferences,
                vec![
                    Regex::new(r"(?i)^(find|show|list)\s+references\s+(to|of)\s+").unwrap(),
                    Regex::new(r"(?i)^where\s+(is|are)\s+\w+\s+used").unwrap(),
                ],
            ),
            // Project management
            (
                Intent::RunTests,
                vec![
                    Regex::new(r"(?i)^(run|execute)\s+(the\s+)?tests?").unwrap(),
                    Regex::new(r"(?i)^test\s+(the\s+)?(project|code)").unwrap(),
                ],
            ),
            (
                Intent::BuildProject,
                vec![
                    Regex::new(r"(?i)^(build|compile)\s+(the\s+)?project").unwrap(),
                    Regex::new(r"(?i)^cargo\s+build").unwrap(),
                    Regex::new(r"(?i)^npm\s+run\s+build").unwrap(),
                ],
            ),
            (
                Intent::FormatCode,
                vec![
                    Regex::new(r"(?i)^(format|prettify)\s+(this\s+)?(code|file)").unwrap(),
                    Regex::new(r"(?i)^cargo\s+fmt").unwrap(),
                    Regex::new(r"(?i)^npm\s+run\s+format").unwrap(),
                ],
            ),
            (
                Intent::LintCode,
                vec![
                    Regex::new(r"(?i)^(lint|check)\s+(this\s+)?(code|file)").unwrap(),
                    Regex::new(r"(?i)^cargo\s+clippy").unwrap(),
                    Regex::new(r"(?i)^npm\s+run\s+lint").unwrap(),
                ],
            ),
            // Git operations
            (
                Intent::GitStatus,
                vec![
                    Regex::new(r"(?i)^git\s+status").unwrap(),
                    Regex::new(r"(?i)^(show|what\s+is)\s+(the\s+)?git\s+status").unwrap(),
                    Regex::new(r"(?i)^what\s+(files\s+)?changed").unwrap(),
                ],
            ),
            (
                Intent::GitCommit,
                vec![
                    Regex::new(r"(?i)^git\s+commit").unwrap(),
                    Regex::new(r"(?i)^commit\s+(these\s+)?changes").unwrap(),
                    Regex::new(r"(?i)^create\s+a\s+commit").unwrap(),
                ],
            ),
            (
                Intent::GitDiff,
                vec![
                    Regex::new(r"(?i)^git\s+diff").unwrap(),
                    Regex::new(r"(?i)^(show|what\s+is)\s+(the\s+)?diff").unwrap(),
                ],
            ),
            (
                Intent::GitLog,
                vec![
                    Regex::new(r"(?i)^git\s+log").unwrap(),
                    Regex::new(r"(?i)^(show|view)\s+(the\s+)?(git\s+)?history").unwrap(),
                ],
            ),
            // Conversation management
            (
                Intent::StartNewThread,
                vec![
                    Regex::new(r"(?i)^(start|begin|create)\s+(a\s+)?new\s+thread").unwrap(),
                    Regex::new(r"(?i)^new\s+conversation").unwrap(),
                ],
            ),
            (
                Intent::SwitchContext,
                vec![
                    Regex::new(r"(?i)^switch\s+(to\s+)?(context|file|project)").unwrap(),
                    Regex::new(r"(?i)^change\s+context").unwrap(),
                ],
            ),
            (
                Intent::ClearHistory,
                vec![
                    Regex::new(r"(?i)^clear\s+(the\s+)?history").unwrap(),
                    Regex::new(r"(?i)^reset\s+conversation").unwrap(),
                ],
            ),
        ]
    }

    /// Matches input against patterns and returns the first matching intent.
    #[must_use]
    pub fn match_input(&self, input: &str) -> Option<&Intent> {
        for (intent, patterns) in &self.patterns {
            if patterns.iter().any(|p| p.is_match(input)) {
                return Some(intent);
            }
        }
        None
    }

    /// Returns all matching intents with confidence scores.
    #[must_use]
    pub fn match_all(&self, input: &str) -> Vec<(&Intent, f64)> {
        let mut matches = Vec::new();

        for (intent, patterns) in &self.patterns {
            let match_count = patterns.iter().filter(|p| p.is_match(input)).count();
            if match_count > 0 {
                // More pattern matches = higher confidence
                let confidence = (0.9_f64).min(0.8 + (match_count as f64 * 0.05));
                matches.push((intent, confidence));
            }
        }

        // Sort by confidence descending
        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        matches
    }
}

impl Default for IntentPatterns {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_generate_code() {
        let patterns = IntentPatterns::new();

        let result = patterns.match_input("generate a function for user authentication");
        assert_eq!(result, Some(&Intent::GenerateCode));

        let result = patterns.match_input("create code to parse JSON");
        assert_eq!(result, Some(&Intent::GenerateCode));
    }

    #[test]
    fn test_match_explain_code() {
        let patterns = IntentPatterns::new();

        let result = patterns.match_input("explain how this code works");
        assert_eq!(result, Some(&Intent::ExplainCode));

        let result = patterns.match_input("what does this function do?");
        assert_eq!(result, Some(&Intent::ExplainCode));
    }

    #[test]
    fn test_match_find_bugs() {
        let patterns = IntentPatterns::new();

        let result = patterns.match_input("find bugs in this code");
        assert_eq!(result, Some(&Intent::FindBugs));

        let result = patterns.match_input("what's wrong with this function?");
        assert_eq!(result, Some(&Intent::FindBugs));
    }

    #[test]
    fn test_match_git_commands() {
        let patterns = IntentPatterns::new();

        let result = patterns.match_input("git status");
        assert_eq!(result, Some(&Intent::GitStatus));

        let result = patterns.match_input("commit these changes");
        assert_eq!(result, Some(&Intent::GitCommit));
    }

    #[test]
    fn test_no_match() {
        let patterns = IntentPatterns::new();

        let result = patterns.match_input("hello world");
        assert_eq!(result, None);

        let result = patterns.match_input("random text that doesn't match");
        assert_eq!(result, None);
    }

    #[test]
    fn test_match_all() {
        let patterns = IntentPatterns::new();

        let matches = patterns.match_all("generate code to test a function");
        assert!(!matches.is_empty());

        // Should match GenerateCode
        assert!(matches.iter().any(|(i, _)| **i == Intent::GenerateCode));
    }
}
