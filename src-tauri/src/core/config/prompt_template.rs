//! Prompt template domain entity.
//!
//! A prompt template pairs a slash-command `slug` (the token after "/" in
//! chat input, e.g. `refactor`) with a body containing a `{args}` placeholder.
//! The chat pipeline rewrites `/{slug} <rest>` into the body with `{args}`
//! replaced by `<rest>` before the user message reaches the LLM.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The token we substitute in a template body with the user's raw arguments.
pub const ARGS_PLACEHOLDER: &str = "{args}";

/// Regex-ready character class for a valid slug. Kept in one place so the
/// backend validator and any future tooling stay in sync.
pub const SLUG_PATTERN: &str = r"^[a-z0-9][a-z0-9-]*$";

/// A prompt template persisted in the config store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PromptTemplate {
    /// Stable identifier (UUID v4).
    pub id: String,
    /// The token typed after `/` in chat (lowercase, `[a-z0-9-]+`).
    pub slug: String,
    /// Display name in the command palette.
    pub title: String,
    /// Subtitle shown under the slug in the palette.
    pub description: String,
    /// Icon key matching the command-palette icon set.
    #[serde(default)]
    pub icon: Option<String>,
    /// Free-form grouping label (e.g. "CODE", "CUSTOM").
    pub category: String,
    /// Template body. Occurrences of `{args}` are replaced with whatever the
    /// user typed after the slug.
    pub content: String,
    /// Seeded-builtin flag. Builtins are deletable only via the reset action.
    pub is_builtin: bool,
    /// Whether this template participates in slash expansion.
    pub is_enabled: bool,
    /// Sort order in the palette (ascending). Builtins are seeded 0..n; user
    /// templates default to a value past the builtin range.
    pub sort_order: i64,
    /// Row creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last-modification timestamp.
    pub updated_at: DateTime<Utc>,
}

impl PromptTemplate {
    /// Expands the template body using `args` as the `{args}` substitution.
    #[must_use]
    pub fn expand(&self, args: &str) -> String {
        self.content.replace(ARGS_PLACEHOLDER, args)
    }

    /// Returns `true` when `slug` matches the documented pattern.
    #[must_use]
    pub fn is_valid_slug(slug: &str) -> bool {
        if slug.is_empty() {
            return false;
        }
        let mut chars = slug.chars();
        let Some(first) = chars.next() else {
            return false;
        };
        if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
            return false;
        }
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }
}

/// Payload used by Tauri commands when creating or updating a template.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePromptTemplateRequest {
    /// Existing row id. `None` creates a new row.
    pub id: Option<String>,
    /// Slash-command slug.
    pub slug: String,
    /// Display title.
    pub title: String,
    /// Palette description.
    pub description: String,
    /// Optional icon key.
    #[serde(default)]
    pub icon: Option<String>,
    /// Category label.
    pub category: String,
    /// Template body with `{args}` placeholder.
    pub content: String,
    /// Whether the template should be active in slash expansion.
    #[serde(default = "default_true")]
    pub is_enabled: bool,
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> PromptTemplate {
        let now = Utc::now();
        PromptTemplate {
            id: "test-id".to_string(),
            slug: "refactor".to_string(),
            title: "Refactor".to_string(),
            description: "Refactor code".to_string(),
            icon: Some("refactor".to_string()),
            category: "CODE".to_string(),
            content: "Please refactor:\n\n{args}".to_string(),
            is_builtin: true,
            is_enabled: true,
            sort_order: 0,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn expand_substitutes_args() {
        let t = sample();
        assert_eq!(t.expand("foo"), "Please refactor:\n\nfoo");
    }

    #[test]
    fn expand_substitutes_empty_args() {
        let t = sample();
        assert_eq!(t.expand(""), "Please refactor:\n\n");
    }

    #[test]
    fn expand_preserves_multiline_args() {
        let t = sample();
        assert_eq!(t.expand("line1\nline2"), "Please refactor:\n\nline1\nline2");
    }

    #[test]
    fn slug_validation_accepts_valid_forms() {
        assert!(PromptTemplate::is_valid_slug("refactor"));
        assert!(PromptTemplate::is_valid_slug("git-commit"));
        assert!(PromptTemplate::is_valid_slug("a"));
        assert!(PromptTemplate::is_valid_slug("1"));
        assert!(PromptTemplate::is_valid_slug("commit-msg-v2"));
    }

    #[test]
    fn slug_validation_rejects_invalid_forms() {
        assert!(!PromptTemplate::is_valid_slug(""));
        assert!(!PromptTemplate::is_valid_slug("-leading-dash"));
        assert!(!PromptTemplate::is_valid_slug("UPPER"));
        assert!(!PromptTemplate::is_valid_slug("with space"));
        assert!(!PromptTemplate::is_valid_slug("with.dot"));
        assert!(!PromptTemplate::is_valid_slug("/slash"));
    }
}
