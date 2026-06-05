//! Pure plan-quality validators.
//!
//! Validates the shape and semantics of a model-declared `agent_plan_subtasks`
//! payload before the runtime persists any subtasks. All functions in this
//! module operate on `serde_json::Value` slices and return structured errors
//! so the caller can surface them as tool-level `requires_replan` responses.
//!
//! Callers in `infrastructure/agent/runtime/permissions.rs`:
//! - `handle_plan_subtasks` invokes [`validate_plan_subtasks`] before merging
//!   or creating any subtask row.
//!
//! Split out of the monolithic permissions module during the permissions-
//! decomposition refactor. See `docs/planning/permissions-decomposition/plan.md`.

use std::collections::HashSet;

/// Validates the full plan-subtasks payload: non-empty, distinct titles,
/// optional verification step, no overlapping contracts, and no trailing
/// no-op "documentation of prior work" subtask.
///
/// When `requires_verification` is `true`, the plan must include at least
/// one subtask whose title hints at a verification step (test, verify,
/// build, etc.). Falsy omission preserves legacy behavior.
pub(crate) fn validate_plan_subtasks(
    subtasks: &[serde_json::Value],
    requires_verification: bool,
) -> Result<(), String> {
    if subtasks.is_empty() {
        return Err("Plan must contain at least one subtask".to_string());
    }

    let verification_terms = [
        "test",
        "verify",
        "validation",
        "regression",
        "check",
        "build",
        "compile",
        "lint",
    ];
    let mut seen = HashSet::new();
    let mut has_verification_step = false;

    for item in subtasks {
        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "Each subtask must have a non-empty title".to_string())?;
        let normalized = title.to_lowercase();
        if !seen.insert(normalized.clone()) {
            return Err(format!("Duplicate subtask title found: '{}'", title));
        }

        if verification_terms
            .iter()
            .any(|term| normalized.contains(term))
        {
            has_verification_step = true;
        }
    }

    if requires_verification && !has_verification_step {
        return Err(
            "Code-change intent detected. Add a verification subtask (test/verify/build/check)."
                .to_string(),
        );
    }

    // Workstream G.1 — plan-quality validation. These checks only
    // trigger when contracts are declared (overlap) or when the final
    // subtask's text + empty contract matches the no-op heuristic;
    // legacy plans without contracts fall through unchanged.
    detect_overlapping_contracts(subtasks)?;
    detect_noop_final_subtask(subtasks)?;

    Ok(())
}

/// Workstream G.1 — rejects plans where two subtasks declare
/// overlapping `contract.expected_artifacts.path_pattern` entries.
///
/// The check is intentionally conservative so ordinary plans with
/// neighbouring but distinct contracts are not rejected. Two patterns
/// are treated as overlapping when any of the following hold:
///
/// 1. They are byte-identical after trimming whitespace.
/// 2. Their literal path prefixes (everything before the first glob
///    metacharacter, trimmed of trailing separators and bare `*` or
///    `**` tokens) are in a proper prefix-of relationship aligned on
///    a `/` boundary (e.g. `docs/social/` is a prefix of
///    `docs/social/twitter/`).
/// 3. One pattern is literal (no glob metacharacters) and is matched
///    by the other pattern's compiled `GlobMatcher`.
///
/// Anything beyond these three rules is treated as non-overlapping to
/// avoid false-positive rejections.
pub(crate) fn detect_overlapping_contracts(subtasks: &[serde_json::Value]) -> Result<(), String> {
    // Gather (subtask_index, title, pattern) tuples for every declared
    // path_pattern on every subtask with a contract. Subtasks without
    // contracts or with empty expected_artifacts contribute nothing
    // and therefore cannot trigger overlap rejection.
    let mut patterns: Vec<(usize, String, String)> = Vec::new();
    for (idx, item) in subtasks.iter().enumerate() {
        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let Some(contract) = item.get("contract") else {
            continue;
        };
        if contract.is_null() {
            continue;
        }
        // Accept both the schema-declared snake_case key and the
        // camelCase variant produced by serde on SubTaskContract so
        // we don't regress when either spelling appears.
        let artifacts = contract
            .get("expected_artifacts")
            .or_else(|| contract.get("expectedArtifacts"))
            .and_then(|v| v.as_array());
        let Some(artifacts) = artifacts else {
            continue;
        };
        for spec in artifacts {
            let pattern_raw = spec
                .get("path_pattern")
                .or_else(|| spec.get("pathPattern"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .trim();
            if pattern_raw.is_empty() {
                continue;
            }
            patterns.push((idx, title.clone(), pattern_raw.to_string()));
        }
    }

    if patterns.len() < 2 {
        return Ok(());
    }

    for i in 0..patterns.len() {
        for j in (i + 1)..patterns.len() {
            let (a_idx, a_title, a_pat) = &patterns[i];
            let (b_idx, b_title, b_pat) = &patterns[j];
            if a_idx == b_idx {
                // Two entries on the same subtask's contract are
                // permitted — they describe parallel outputs owned
                // by the same unit of work.
                continue;
            }
            if patterns_overlap(a_pat, b_pat) {
                return Err(format!(
                    "Subtask '{a_title}' (index {a_idx}) and subtask '{b_title}' (index {b_idx}) declare overlapping expected-artifact patterns ('{a_pat}' and '{b_pat}'). Each subtask must own a distinct output path; rewrite the contracts so each pattern covers exactly one subtask's outputs, or merge the subtasks."
                ));
            }
        }
    }

    Ok(())
}

/// Returns `true` when `a` and `b` overlap under the conservative
/// rules documented on [`detect_overlapping_contracts`]. Patterns
/// deemed unrelated return `false`.
pub(crate) fn patterns_overlap(a: &str, b: &str) -> bool {
    let a = a.trim();
    let b = b.trim();
    if a.is_empty() || b.is_empty() {
        return false;
    }
    // Rule 1 — byte identical.
    if a == b {
        return true;
    }

    // Rule 2 — literal-prefix relationship on `/` boundaries.
    let prefix_a = literal_prefix(a);
    let prefix_b = literal_prefix(b);
    if is_path_prefix_of(&prefix_a, &prefix_b) || is_path_prefix_of(&prefix_b, &prefix_a) {
        return true;
    }

    // Rule 3 — one side is literal (no glob metacharacters) and is
    // matched by the other's compiled matcher.
    if let Some(overlap) = literal_matches_glob(a, b) {
        return overlap;
    }
    if let Some(overlap) = literal_matches_glob(b, a) {
        return overlap;
    }

    false
}

/// Returns the literal directory prefix of a glob pattern — everything
/// up to (but not including) the first glob metacharacter, trimmed of
/// trailing separators. A pattern like `docs/social/*.md` yields
/// `docs/social`; a pattern like `*.md` yields an empty string.
pub(crate) fn literal_prefix(pattern: &str) -> String {
    let meta_idx = pattern
        .char_indices()
        .find(|(_, c)| matches!(*c, '*' | '?' | '[' | '{'))
        .map_or(pattern.len(), |(i, _)| i);
    pattern[..meta_idx]
        .trim_end_matches('/')
        .trim_end_matches('\\')
        .to_string()
}

/// Returns `true` when `parent` is a proper path prefix of `child`,
/// aligned on a `/` boundary. Empty `parent` never matches (the
/// check is meaningless for globs with no literal anchor).
pub(crate) fn is_path_prefix_of(parent: &str, child: &str) -> bool {
    if parent.is_empty() || child.is_empty() {
        return false;
    }
    if parent == child {
        // Pure equality is handled by Rule 1 already; return true
        // here so pattern forms like `docs` and `docs` (after
        // stripping trailing glob) are still flagged.
        return true;
    }
    if !child.starts_with(parent) {
        return false;
    }
    // Ensure the boundary is a path separator so `docs/a` isn't
    // flagged as a prefix of `docs/alpha`.
    let remainder = &child[parent.len()..];
    remainder.starts_with('/') || remainder.starts_with('\\')
}

/// Returns `Some(bool)` when `candidate` is a literal path and can be
/// tested against `pattern` as a glob; `None` when `candidate` has
/// glob metacharacters and therefore can't be used as a concrete-path
/// probe.
pub(crate) fn literal_matches_glob(candidate: &str, pattern: &str) -> Option<bool> {
    if candidate
        .chars()
        .any(|c| matches!(c, '*' | '?' | '[' | '{'))
    {
        return None;
    }
    let glob = globset::Glob::new(pattern).ok()?;
    let matcher = glob.compile_matcher();
    Some(matcher.is_match(candidate))
}

/// Workstream G.1 — rejects plans whose final subtask is a no-op
/// "generate documentation of prior subtasks" index with no contract.
///
/// The detector is deliberately narrow: it only fires on the **last**
/// subtask, requires both a lexical trigger *and* an absent/empty
/// contract, and lets any subtask with a declared contract through
/// untouched. This keeps legitimate final-step documentation subtasks
/// (which declare where they write) out of the reject set.
pub(crate) fn detect_noop_final_subtask(subtasks: &[serde_json::Value]) -> Result<(), String> {
    // Need at least two subtasks — a single-subtask plan cannot be a
    // "documentation of prior subtasks" redundancy.
    if subtasks.len() < 2 {
        return Ok(());
    }

    let Some(last) = subtasks.last() else {
        return Ok(());
    };

    // If the final subtask has a non-empty contract, trust the
    // declaration and do not reject — the contract itself justifies
    // the subtask's presence.
    let contract_is_trivial = match last.get("contract") {
        None => true,
        Some(v) if v.is_null() => true,
        Some(v) => {
            let artifacts = v
                .get("expected_artifacts")
                .or_else(|| v.get("expectedArtifacts"))
                .and_then(|a| a.as_array());
            match artifacts {
                Some(list) => list.is_empty(),
                None => true,
            }
        }
    };
    if !contract_is_trivial {
        return Ok(());
    }

    let title = last
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase();
    let description = last
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_lowercase();
    let haystack = format!("{title} {description}");

    // Triggers are fragments so inflected forms ("summarize",
    // "summarizing", "summary") all match a single stem. The
    // phrases are picked to cover the observed regression plus
    // close paraphrases without reaching into words that appear
    // in legitimate action-oriented subtasks.
    const TRIGGERS: &[&str] = &[
        "summariz",
        "index",
        "documentation of",
        "list of files",
        "recap",
        "overview of what",
        "document the above",
        "document what was",
    ];

    let matched = TRIGGERS.iter().find(|t| haystack.contains(**t));
    if let Some(trigger) = matched {
        let original_title = last
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("<untitled>")
            .to_string();
        return Err(format!(
            "Final subtask '{original_title}' looks like a no-op index of prior work (matched heuristic '{trigger}') and declares no output contract. Remove it, or replace it with a concrete artifact contract describing the index file it must produce."
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests migrated from `infrastructure/agent/runtime/permissions.rs` in the
// permissions-decomposition refactor. Coverage tracks the pure validators in
// this module so future edits here stay pinned to behavior.
// ---------------------------------------------------------------------------
#[cfg(test)]
#[path = "plan_validation_tests.rs"]
mod plan_quality_validation_tests;

// ---------------------------------------------------------------------------
// Phase 0 characterization — pure predicate coverage migrated from permissions.rs.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod pure_helper_characterization_tests {
    use super::*;

    // -------- literal_prefix ----------------------------------------------

    #[test]
    fn literal_prefix_returns_whole_string_when_no_glob_metacharacters() {
        assert_eq!(literal_prefix("docs/social/readme"), "docs/social/readme");
    }

    #[test]
    fn literal_prefix_trims_trailing_separator_before_first_glob() {
        assert_eq!(literal_prefix("docs/social/*.md"), "docs/social");
    }

    #[test]
    fn literal_prefix_is_empty_when_first_char_is_glob() {
        assert_eq!(literal_prefix("*.md"), "");
    }

    #[test]
    fn literal_prefix_handles_brace_expansion_start() {
        assert_eq!(literal_prefix("src/{a,b}/file"), "src");
    }

    #[test]
    fn literal_prefix_treats_question_mark_as_meta() {
        assert_eq!(literal_prefix("dir/?ile"), "dir");
    }

    // -------- is_path_prefix_of -------------------------------------------

    #[test]
    fn is_path_prefix_of_rejects_empty_parent() {
        assert!(!is_path_prefix_of("", "docs/a"));
    }

    #[test]
    fn is_path_prefix_of_accepts_equal_strings() {
        assert!(is_path_prefix_of("docs/a", "docs/a"));
    }

    #[test]
    fn is_path_prefix_of_accepts_boundary_aligned_prefix() {
        assert!(is_path_prefix_of("docs/social", "docs/social/twitter"));
    }

    #[test]
    fn is_path_prefix_of_rejects_unaligned_prefix() {
        assert!(!is_path_prefix_of("docs/a", "docs/alpha"));
    }

    #[test]
    fn is_path_prefix_of_rejects_unrelated_paths() {
        assert!(!is_path_prefix_of("docs/a", "src/b"));
    }

    // -------- literal_matches_glob ---------------------------------------

    #[test]
    fn literal_matches_glob_returns_none_for_non_literal_candidate() {
        assert_eq!(literal_matches_glob("foo/*.md", "foo/*.md"), None);
    }

    #[test]
    fn literal_matches_glob_returns_true_for_literal_under_glob() {
        assert_eq!(
            literal_matches_glob("docs/twitter.md", "docs/*.md"),
            Some(true),
        );
    }

    #[test]
    fn literal_matches_glob_returns_false_for_literal_outside_glob() {
        assert_eq!(
            literal_matches_glob("src/twitter.md", "docs/*.md"),
            Some(false),
        );
    }

    // -------- patterns_overlap edge cases --------------------------------

    #[test]
    fn patterns_overlap_rejects_empty_strings() {
        assert!(!patterns_overlap("", ""));
        assert!(!patterns_overlap("docs/*.md", ""));
    }

    // -------- validate_plan_subtasks edge cases --------------------------

    #[test]
    fn validate_plan_subtasks_rejects_empty_plan() {
        let err = validate_plan_subtasks(&[], false).expect_err("empty plan rejected");
        assert!(err.contains("at least one"));
    }

    #[test]
    fn validate_plan_subtasks_requires_verification_when_flag_set() {
        let subtasks = vec![
            serde_json::json!({"title": "Implement feature"}),
            serde_json::json!({"title": "Polish copy"}),
        ];
        let err = validate_plan_subtasks(&subtasks, true)
            .expect_err("code-change intent requires a verification step");
        assert!(err.contains("verification"));
    }

    #[test]
    fn validate_plan_subtasks_accepts_when_verification_step_present() {
        let subtasks = vec![
            serde_json::json!({"title": "Implement feature"}),
            serde_json::json!({"title": "Verify behavior with tests"}),
        ];
        assert!(validate_plan_subtasks(&subtasks, true).is_ok());
    }

    #[test]
    fn validate_plan_subtasks_rejects_duplicate_titles() {
        let subtasks = vec![
            serde_json::json!({"title": "Do the thing"}),
            serde_json::json!({"title": "DO THE THING"}),
        ];
        let err = validate_plan_subtasks(&subtasks, false)
            .expect_err("duplicate titles must be rejected");
        assert!(err.contains("Duplicate"));
    }
}
