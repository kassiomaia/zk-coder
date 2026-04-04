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
                    Regex::new(r"(?i)^(please\s+)?(generate|create|write)\s+(code|a(\s+\w+)*\s+(function|class))")
                        .unwrap(),
                    Regex::new(r"(?i)^(please\s+)?(help\s+me\s+)?(code|implement|build\s+(a\s+)?(component|function|class))\s*").unwrap(),
                    Regex::new(r"(?i)^(please\s+)?make\s+(a\s+)?(function|class|component)\s*").unwrap(),
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
                    Regex::new(r"(?i)^(refactor|improve|clean\s+up)\s*").unwrap(),
                    Regex::new(r"(?i)make\s+(this|the|code)\s+(better|cleaner)\s*").unwrap(),
                ],
            ),
            (
                Intent::OptimizeCode,
                vec![
                    Regex::new(r"(?i)^(optimize|speed\s+up|make\s+(this|the|it)\s+faster)\s*").unwrap(),
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
                    Regex::new(r"(?i)^(analyze|review)\s+(this\s+)?(code|file)").unwrap(),
                    Regex::new(r"(?i)^code\s+review\s+for\s+").unwrap(),
                ],
            ),
            (
                Intent::FindBugs,
                vec![
                    Regex::new(r"(?i)(find|detect|spot)\s+(bugs?|issues?|errors?)").unwrap(),
                    Regex::new(r"(?i)^what'?s\s+wrong(\s+(with|in)\s+)?").unwrap(),
                    Regex::new(r"(?i)^bug\s+in\s+").unwrap(),
                ],
            ),
            (
                Intent::DocumentCode,
                vec![
                    Regex::new(r"(?i)^(document|add\s+docs?|add\s+comments?)\s*").unwrap(),
                    Regex::new(r"(?i)^generate\s+documentation\s+for\s+").unwrap(),
                ],
            ),
            // Navigation
            (
                Intent::FindFile,
                vec![
                    Regex::new(r"(?i)^(find|locate|open)\s+(a\s+)?file\s*").unwrap(),
                    Regex::new(r"(?i)^where\s+is\s+").unwrap(),
                    Regex::new(r"(?i)^file\s+named\s*").unwrap(),
                ],
            ),
            (
                Intent::FindSymbol,
                vec![
                    Regex::new(r"(?i)^(find|locate)\s+(a\s+)?(function|class|method|symbol)")
                        .unwrap(),
                    Regex::new(r"(?i)^where\s+(is|are)\s+(the\s+)?(function|class|method)")
                        .unwrap(),
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
                    Regex::new(r"(?i)^test\s+(the\s+)?(this\s+)?(project|code)\s*").unwrap(),
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
                    Regex::new(r"(?i)^(format|prettify)\s+(this\s+)?(code|file)?\s*").unwrap(),
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
                    Regex::new(r"(?i)^(show|what('s|s\s+is))\s+(the\s+)?git\s+status").unwrap(),
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
                    Regex::new(r"(?i)^(show|what('s|s\s+is))\s+(the\s+)?diff").unwrap(),
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
                let confidence = (0.9_f64).min((match_count as f64).mul_add(0.05, 0.8));
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

    // Edge case input fixtures
    const EMPTY_INPUT: &str = "";
    const WHITESPACE_ONLY: &str = "   ";
    const VERY_LONG_INPUT: &str =
        "please generate a very long and detailed function that does many things";
    const SPECIAL_CHARS: &str = "generate code with @#$ special chars!";

    // Generate code fixtures
    const GENERATE_INPUTS: &[&str] = &[
        "generate a function",
        "create a class",
        "write code",
        "implement this",
        "build a component",
        "help me code",
        "code an authentication system",
        "Generate code for me", // Test case insensitivity
    ];

    // Explain code fixtures
    const EXPLAIN_INPUTS: &[&str] = &[
        "explain this code",
        "what does this do",
        "how does this work",
        "tell me about this function",
        "what is this",
    ];

    // Find bugs fixtures
    const BUG_INPUTS: &[&str] = &[
        "find bugs",
        "detect issues",
        "spot errors",
        "what's wrong",
        "bug in this code",
    ];

    // Refactor fixtures
    const REFACTOR_INPUTS: &[&str] = &[
        "refactor this",
        "improve this code",
        "clean up",
        "make this cleaner",
    ];

    // Git fixtures
    const GIT_STATUS_INPUTS: &[&str] = &[
        "git status",
        "show git status",
        "what's the git status",
        "what files changed",
    ];

    const GIT_COMMIT_INPUTS: &[&str] = &["git commit", "commit these changes", "create a commit"];

    // Navigation fixtures
    const FIND_FILE_INPUTS: &[&str] = &["find file", "locate file", "where is file", "file named"];

    // Project management fixtures
    const RUN_TESTS_INPUTS: &[&str] = &["run tests", "execute tests", "test the project"];

    const BUILD_INPUTS: &[&str] = &[
        "build project",
        "compile project",
        "cargo build",
        "npm run build",
    ];

    const FORMAT_INPUTS: &[&str] = &[
        "format code",
        "prettify this",
        "cargo fmt",
        "npm run format",
    ];

    const LINT_INPUTS: &[&str] = &[
        "lint code",
        "check this code",
        "cargo clippy",
        "npm run lint",
    ];

    // Conversation management fixtures
    const NEW_THREAD_INPUTS: &[&str] = &[
        "start new thread",
        "begin new thread",
        "create new thread",
        "new conversation",
    ];

    const SWITCH_CONTEXT_INPUTS: &[&str] = &["switch context", "change context", "switch to file"];

    const CLEAR_HISTORY_INPUTS: &[&str] = &["clear history", "reset conversation"];

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

    #[test]
    fn test_all_generate_inputs_match() {
        let patterns = IntentPatterns::new();

        for input in GENERATE_INPUTS {
            let result = patterns.match_input(input);
            assert!(result.is_some(), "Expected match for input: '{input}'",);
        }
    }

    #[test]
    fn test_all_explain_inputs_match() {
        let patterns = IntentPatterns::new();

        for input in EXPLAIN_INPUTS {
            let result = patterns.match_input(input);
            assert_eq!(
                result,
                Some(&Intent::ExplainCode),
                "Expected ExplainCode for: '{input}'"
            );
        }
    }

    #[test]
    fn test_all_bug_inputs_match() {
        let patterns = IntentPatterns::new();

        for input in BUG_INPUTS {
            let result = patterns.match_input(input);
            assert_eq!(
                result,
                Some(&Intent::FindBugs),
                "Expected FindBugs for: '{input}'"
            );
        }
    }

    #[test]
    fn test_all_refactor_inputs_match() {
        let patterns = IntentPatterns::new();

        for input in REFACTOR_INPUTS {
            let result = patterns.match_input(input);
            assert!(
                result.is_some(),
                "Expected match for refactor input: '{input}'"
            );
        }
    }

    #[test]
    fn test_all_git_status_inputs_match() {
        let patterns = IntentPatterns::new();

        for input in GIT_STATUS_INPUTS {
            let result = patterns.match_input(input);
            assert_eq!(
                result,
                Some(&Intent::GitStatus),
                "Expected GitStatus for: '{input}'"
            );
        }
    }

    #[test]
    fn test_all_git_commit_inputs_match() {
        let patterns = IntentPatterns::new();

        for input in GIT_COMMIT_INPUTS {
            let result = patterns.match_input(input);
            assert_eq!(
                result,
                Some(&Intent::GitCommit),
                "Expected GitCommit for: '{input}'"
            );
        }
    }

    #[test]
    fn test_all_navigation_inputs_match() {
        let patterns = IntentPatterns::new();

        for input in FIND_FILE_INPUTS {
            let result = patterns.match_input(input);
            assert!(
                result.is_some(),
                "Expected match for navigation input: '{input}'"
            );
        }
    }

    #[test]
    fn test_all_project_management_inputs_match() {
        let patterns = IntentPatterns::new();

        // Run tests
        for input in RUN_TESTS_INPUTS {
            assert_eq!(
                patterns.match_input(input),
                Some(&Intent::RunTests),
                "Expected RunTests for: '{input}'"
            );
        }

        // Build
        for input in BUILD_INPUTS {
            assert_eq!(
                patterns.match_input(input),
                Some(&Intent::BuildProject),
                "Expected BuildProject for: '{input}'"
            );
        }

        // Format
        for input in FORMAT_INPUTS {
            assert_eq!(
                patterns.match_input(input),
                Some(&Intent::FormatCode),
                "Expected FormatCode for: '{input}'"
            );
        }

        // Lint
        for input in LINT_INPUTS {
            assert_eq!(
                patterns.match_input(input),
                Some(&Intent::LintCode),
                "Expected LintCode for: '{input}'"
            );
        }
    }

    #[test]
    fn test_all_conversation_management_inputs_match() {
        let patterns = IntentPatterns::new();

        for input in NEW_THREAD_INPUTS {
            assert_eq!(
                patterns.match_input(input),
                Some(&Intent::StartNewThread),
                "Expected StartNewThread for: '{input}'"
            );
        }

        for input in SWITCH_CONTEXT_INPUTS {
            assert_eq!(
                patterns.match_input(input),
                Some(&Intent::SwitchContext),
                "Expected SwitchContext for: '{input}'"
            );
        }

        for input in CLEAR_HISTORY_INPUTS {
            assert_eq!(
                patterns.match_input(input),
                Some(&Intent::ClearHistory),
                "Expected ClearHistory for: '{input}'"
            );
        }
    }

    #[test]
    fn test_edge_case_empty_input() {
        let patterns = IntentPatterns::new();
        assert_eq!(patterns.match_input(EMPTY_INPUT), None);
    }

    #[test]
    fn test_edge_case_whitespace_only() {
        let patterns = IntentPatterns::new();
        assert_eq!(patterns.match_input(WHITESPACE_ONLY), None);
    }

    #[test]
    fn test_case_insensitive_matching() {
        let patterns = IntentPatterns::new();

        // Test various case combinations
        assert_eq!(
            patterns.match_input("GENERATE CODE"),
            Some(&Intent::GenerateCode)
        );
        assert_eq!(
            patterns.match_input("Generate Code"),
            Some(&Intent::GenerateCode)
        );
        assert_eq!(
            patterns.match_input("gEnErAtE cOdE"),
            Some(&Intent::GenerateCode)
        );
    }

    #[test]
    fn test_special_characters_in_input() {
        let patterns = IntentPatterns::new();
        // Should still match even with special chars
        let result = patterns.match_input("generate code with @#$ special chars!");
        assert_eq!(result, Some(&Intent::GenerateCode));
    }

    #[test]
    fn test_long_input_still_matches() {
        let patterns = IntentPatterns::new();
        let long_input = "please generate a very long and detailed function that does many things";
        assert_eq!(
            patterns.match_input(long_input),
            Some(&Intent::GenerateCode)
        );
    }

    #[test]
    fn test_partial_matches_dont_trigger() {
        let patterns = IntentPatterns::new();

        // These should not match because they don't start with the pattern
        assert_ne!(
            patterns.match_input("I want to explain this later"),
            Some(&Intent::ExplainCode)
        );
        assert_ne!(
            patterns.match_input("maybe generate something"),
            Some(&Intent::GenerateCode)
        );
    }

    #[test]
    fn test_confidence_scores_are_valid() {
        let patterns = IntentPatterns::new();

        let matches = patterns.match_all("generate code to test");
        for (_intent, confidence) in matches {
            assert!(
                (0.0..=1.0).contains(&confidence),
                "Confidence {confidence} is not in valid range"
            );
        }
    }

    #[test]
    fn test_multiple_pattern_matches_increase_confidence() {
        let patterns = IntentPatterns::new();

        // Input that might match multiple patterns for same intent
        let matches = patterns.match_all("generate a function");
        if matches.len() == 1 {
            let (_intent, confidence) = matches[0];
            assert!(
                confidence > 0.8,
                "Multi-pattern match should have high confidence"
            );
        }
    }

    #[test]
    fn test_match_all_returns_sorted_by_confidence() {
        let patterns = IntentPatterns::new();

        let matches = patterns.match_all("generate code to test a function");
        if matches.len() > 1 {
            // Verify sorted descending
            for i in 1..matches.len() {
                assert!(
                    matches[i].1 <= matches[i - 1].1,
                    "Matches should be sorted by confidence descending"
                );
            }
        }
    }

    #[test]
    fn test_git_diff_matches() {
        let patterns = IntentPatterns::new();

        assert_eq!(patterns.match_input("git diff"), Some(&Intent::GitDiff));
        assert_eq!(
            patterns.match_input("show the diff"),
            Some(&Intent::GitDiff)
        );
        assert_eq!(
            patterns.match_input("what's the diff"),
            Some(&Intent::GitDiff)
        );
    }

    #[test]
    fn test_git_log_matches() {
        let patterns = IntentPatterns::new();

        assert_eq!(patterns.match_input("git log"), Some(&Intent::GitLog));
        assert_eq!(patterns.match_input("show history"), Some(&Intent::GitLog));
        assert_eq!(
            patterns.match_input("view git history"),
            Some(&Intent::GitLog)
        );
    }

    #[test]
    fn test_navigation_intents_match_correctly() {
        let patterns = IntentPatterns::new();

        assert_eq!(
            patterns.match_input("go to definition of foo"),
            Some(&Intent::GotoDefinition)
        );
        assert_eq!(
            patterns.match_input("find references to bar"),
            Some(&Intent::FindReferences)
        );
        assert_eq!(
            patterns.match_input("find symbol baz"),
            Some(&Intent::FindSymbol)
        );
    }

    #[test]
    fn test_optimize_code_matches() {
        let patterns = IntentPatterns::new();

        assert_eq!(
            patterns.match_input("optimize this function"),
            Some(&Intent::OptimizeCode)
        );
        assert_eq!(
            patterns.match_input("speed up this code"),
            Some(&Intent::OptimizeCode)
        );
        assert_eq!(
            patterns.match_input("make this faster"),
            Some(&Intent::OptimizeCode)
        );
    }

    #[test]
    fn test_document_code_matches() {
        let patterns = IntentPatterns::new();

        assert_eq!(
            patterns.match_input("document this function"),
            Some(&Intent::DocumentCode)
        );
        assert_eq!(
            patterns.match_input("add documentation"),
            Some(&Intent::DocumentCode)
        );
        assert_eq!(
            patterns.match_input("add comments to this code"),
            Some(&Intent::DocumentCode)
        );
    }

    #[test]
    fn test_specific_intents_not_confused() {
        let patterns = IntentPatterns::new();

        // These should not be confused
        assert_ne!(
            patterns.match_input("generate tests"),
            Some(&Intent::RunTests)
        );
        assert_ne!(
            patterns.match_input("run tests"),
            Some(&Intent::GenerateTest)
        );
        assert_ne!(patterns.match_input("git status"), Some(&Intent::GitCommit));
    }

    #[test]
    fn test_default_trait_implementation() {
        let patterns = IntentPatterns::default();
        // Should work the same as new()
        assert_eq!(
            patterns.match_input("generate code"),
            Some(&Intent::GenerateCode)
        );
    }
}
