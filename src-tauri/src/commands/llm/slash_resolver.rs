//! Expands `/slug <args>` at the top of a chat message into the body of a
//! user-editable prompt template, with `{args}` substituted.
//!
//! The resolver runs exactly once per user turn — in `send_chat_message` and
//! `send_chat_message_streaming`, before the orchestrator loop — so later
//! agent turns see the already-expanded content.

use std::sync::Arc;

use crate::core::config::ConfigRepository;

/// Splits `content` into a `(slug, args)` pair when it opens with a slash
/// command. Returns `None` for anything that isn't a slash command.
///
/// An inline sequence of `a-z 0-9 -` starting with `a-z` or `0-9` is treated
/// as the slug. Whitespace (including newlines) after the slug is consumed
/// once; everything else is returned verbatim as the argument tail.
fn split_command(content: &str) -> Option<(&str, &str)> {
    let rest = content.strip_prefix('/')?;

    let mut slug_len = 0;
    for (idx, ch) in rest.char_indices() {
        let valid = if idx == 0 {
            ch.is_ascii_lowercase() || ch.is_ascii_digit()
        } else {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'
        };
        if valid {
            slug_len = idx + ch.len_utf8();
        } else {
            break;
        }
    }
    if slug_len == 0 {
        return None;
    }

    let (slug, tail) = rest.split_at(slug_len);

    // After the slug we require either end-of-input or a whitespace boundary
    // so `/refactorX` does not match slug `refactor`.
    let args = if tail.is_empty() {
        ""
    } else {
        let first = tail.chars().next().unwrap();
        if !first.is_whitespace() {
            return None;
        }
        // Consume exactly one whitespace separator; keep the rest verbatim
        // (newlines included).
        &tail[first.len_utf8()..]
    };

    Some((slug, args))
}

/// Expands `content` using the configured prompt templates. Returns the
/// original content when:
///
/// - the text is not a slash command;
/// - the slug is unknown;
/// - the matching template is disabled.
pub fn resolve(content: &str, repo: &Arc<dyn ConfigRepository>) -> String {
    let Some((slug, args)) = split_command(content) else {
        return content.to_string();
    };

    match repo.get_prompt_template_by_slug(slug) {
        Ok(Some(template)) if template.is_enabled => template.expand(args),
        Ok(_) => content.to_string(),
        Err(err) => {
            log::warn!(
                "slash_resolver: lookup for '/{slug}' failed, passing through unchanged: {err}"
            );
            content.to_string()
        }
    }
}

/// Applies `resolve` if `content` plausibly looks like a slash command,
/// otherwise returns the input unchanged. Exposed for callers that want to
/// skip the repository hit for obviously-non-command inputs.
pub fn resolve_if_command(content: &str, repo: &Arc<dyn ConfigRepository>) -> String {
    if content.starts_with('/') {
        resolve(content, repo)
    } else {
        content.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;

    use super::*;
    use crate::core::config::PromptTemplate;
    use crate::infrastructure::sqlite::repositories::config::SqliteConfigRepository;

    fn make_repo() -> Arc<dyn ConfigRepository> {
        Arc::new(SqliteConfigRepository::in_memory().expect("in-memory repo"))
            as Arc<dyn ConfigRepository>
    }

    fn disable_refactor(repo: &Arc<dyn ConfigRepository>) {
        let mut t = repo
            .get_prompt_template_by_slug("refactor")
            .unwrap()
            .unwrap();
        t.is_enabled = false;
        t.updated_at = Utc::now();
        repo.upsert_prompt_template(&t).unwrap();
    }

    #[test]
    fn split_command_detects_slug_only() {
        assert_eq!(split_command("/refactor"), Some(("refactor", "")));
    }

    #[test]
    fn split_command_detects_slug_with_args() {
        assert_eq!(
            split_command("/refactor tighten this loop"),
            Some(("refactor", "tighten this loop"))
        );
    }

    #[test]
    fn split_command_preserves_multiline_args() {
        assert_eq!(
            split_command("/refactor\nfn main() {\n    println!(\"hi\");\n}"),
            Some(("refactor", "fn main() {\n    println!(\"hi\");\n}"))
        );
    }

    #[test]
    fn split_command_rejects_non_command_text() {
        assert_eq!(split_command("hello world"), None);
        assert_eq!(split_command(""), None);
        assert_eq!(split_command("/"), None);
        assert_eq!(split_command("/-bad"), None);
        assert_eq!(split_command("/UPPER"), None);
    }

    #[test]
    fn split_command_rejects_slug_without_boundary() {
        // `/refactorX` must not match `refactor` — the boundary check keeps
        // partial-prefix commands from hijacking unrelated slugs.
        assert_eq!(split_command("/refactorX"), None);
    }

    #[test]
    fn resolve_passes_through_non_commands() {
        let repo = make_repo();
        assert_eq!(resolve("hello", &repo), "hello");
        assert_eq!(resolve("", &repo), "");
    }

    #[test]
    fn resolve_passes_through_unknown_slug() {
        let repo = make_repo();
        assert_eq!(resolve("/no-such-slug foo", &repo), "/no-such-slug foo");
    }

    #[test]
    fn resolve_expands_builtin_template() {
        let repo = make_repo();
        let expanded = resolve("/refactor tighten this loop", &repo);
        assert!(expanded.contains("tighten this loop"));
        assert!(expanded.starts_with("Refactor the following code"));
        assert!(!expanded.contains("{args}"));
    }

    #[test]
    fn resolve_expands_with_empty_args() {
        let repo = make_repo();
        let expanded = resolve("/refactor", &repo);
        assert!(expanded.starts_with("Refactor the following code"));
        assert!(!expanded.contains("{args}"));
    }

    #[test]
    fn resolve_preserves_multiline_args() {
        let repo = make_repo();
        let input = "/refactor\nfn main() {\n    42\n}";
        let expanded = resolve(input, &repo);
        assert!(expanded.contains("fn main() {\n    42\n}"));
    }

    #[test]
    fn resolve_passes_through_disabled_template() {
        let repo = make_repo();
        disable_refactor(&repo);
        assert_eq!(resolve("/refactor foo", &repo), "/refactor foo");
    }

    #[test]
    fn resolve_respects_custom_template() {
        let repo = make_repo();
        let now = Utc::now();
        let custom = PromptTemplate {
            id: uuid::Uuid::new_v4().to_string(),
            slug: "commit-msg".to_string(),
            title: "Commit message".to_string(),
            description: "".to_string(),
            icon: None,
            category: "CUSTOM".to_string(),
            content: "Write a commit message for:\n\n{args}".to_string(),
            is_builtin: false,
            is_enabled: true,
            sort_order: 50,
            created_at: now,
            updated_at: now,
        };
        repo.upsert_prompt_template(&custom).unwrap();
        let expanded = resolve("/commit-msg fix flaky test", &repo);
        assert_eq!(expanded, "Write a commit message for:\n\nfix flaky test");
    }

    #[test]
    fn resolve_if_command_skips_non_command_fast_path() {
        let repo = make_repo();
        // Only checks that the non-command branch returns identity — the
        // early-return is an optimization, not a behavioral change.
        assert_eq!(resolve_if_command("hello", &repo), "hello");
        assert_eq!(
            resolve_if_command("/refactor foo", &repo).contains("foo"),
            true
        );
    }
}
