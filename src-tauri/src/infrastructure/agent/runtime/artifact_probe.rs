//! Filesystem artifact probing for contract verification.
//!
//! Pure helpers that resolve a subtask contract's `path_pattern`
//! against either the concrete filesystem or the in-session files-
//! touched overlay.
//!
//! Extracted from `permissions.rs` in the permissions-decomposition
//! refactor (Phase 2). See `docs/planning/permissions-decomposition/plan.md`.

/// Returns `true` when the pattern resolves to at least one concrete
/// file beneath `workspace_root`. A plain (non-glob) pattern is
/// short-circuited to a single `exists()` check. For glob patterns
/// this delegates to a bounded walk.
pub(crate) fn pattern_is_satisfied_on_disk(
    pattern: &str,
    workspace_root: Option<&std::path::Path>,
) -> bool {
    use globset::Glob;
    let Some(root) = workspace_root else {
        return false;
    };
    // Plain path (no glob metacharacters): short-circuit to `exists`.
    if !pattern.contains(['*', '?', '[', '{']) {
        return root.join(pattern).exists();
    }

    let glob = match Glob::new(pattern) {
        Ok(g) => g,
        Err(_) => return false,
    };
    let matcher = glob.compile_matcher();

    walk_workspace_for_match(root, &matcher)
}

/// Bounded workspace walk that returns `true` as soon as any file
/// matches the compiled glob. Depth-limited to keep accidental
/// submits against oversized workspaces cheap; the hard bound is
/// enforced by the max-entries cap below.
fn walk_workspace_for_match(
    root: &std::path::Path,
    matcher: &globset::GlobMatcher,
) -> bool {
    const MAX_ENTRIES: usize = 20_000;
    let mut stack: Vec<std::path::PathBuf> = vec![root.to_path_buf()];
    let mut seen = 0usize;
    while let Some(current) = stack.pop() {
        let Ok(read_dir) = std::fs::read_dir(&current) else {
            continue;
        };
        for entry in read_dir.flatten() {
            seen += 1;
            if seen > MAX_ENTRIES {
                return false;
            }
            let path = entry.path();
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let relative_str = relative.to_string_lossy();
            if matcher.is_match(relative_str.as_ref()) {
                return true;
            }
            if path.is_dir() {
                // Skip common heavy directories that are never
                // relevant to a subtask contract output.
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if matches!(
                        name,
                        ".git"
                            | "node_modules"
                            | "target"
                            | "dist"
                            | "build"
                            | ".next"
                            | ".svelte-kit"
                    ) {
                        continue;
                    }
                }
                stack.push(path);
            }
        }
    }
    false
}

/// Returns `true` when at least one recorded touched path matches the
/// compiled glob. Invalid globs silently return `false` — the caller
/// treats an unparseable pattern identically to a no-match.
pub(crate) fn pattern_matches_touched_overlay(
    pattern: &str,
    touched_paths: &[String],
) -> bool {
    use globset::Glob;
    let Ok(glob) = Glob::new(pattern) else {
        return false;
    };
    let matcher = glob.compile_matcher();
    touched_paths.iter().any(|path| matcher.is_match(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------- pattern_matches_touched_overlay ----------------------------

    #[test]
    fn overlay_matches_plain_literal_path() {
        let touched = vec!["docs/social/twitter.md".to_string()];
        assert!(pattern_matches_touched_overlay(
            "docs/social/twitter.md",
            &touched,
        ));
    }

    #[test]
    fn overlay_matches_glob_pattern() {
        let touched = vec!["docs/social/twitter.md".to_string()];
        assert!(pattern_matches_touched_overlay(
            "docs/social/*.md",
            &touched,
        ));
    }

    #[test]
    fn overlay_rejects_when_nothing_matches() {
        let touched = vec!["src/main.rs".to_string()];
        assert!(!pattern_matches_touched_overlay(
            "docs/social/*.md",
            &touched,
        ));
    }

    #[test]
    fn overlay_tolerates_empty_touched_list() {
        let touched: Vec<String> = Vec::new();
        assert!(!pattern_matches_touched_overlay("docs/*.md", &touched));
    }

    #[test]
    fn overlay_returns_false_for_invalid_glob() {
        let touched = vec!["docs/x.md".to_string()];
        assert!(!pattern_matches_touched_overlay("[unclosed", &touched));
    }

    // -------- pattern_is_satisfied_on_disk -------------------------------

    #[test]
    fn on_disk_returns_false_when_no_workspace_root() {
        assert!(!pattern_is_satisfied_on_disk("*.md", None));
    }

    #[test]
    fn on_disk_resolves_literal_path_exists() {
        let temp = tempfile::tempdir().expect("temp");
        std::fs::write(temp.path().join("file.txt"), "x").expect("write");
        assert!(pattern_is_satisfied_on_disk(
            "file.txt",
            Some(temp.path()),
        ));
    }

    #[test]
    fn on_disk_resolves_literal_path_missing() {
        let temp = tempfile::tempdir().expect("temp");
        assert!(!pattern_is_satisfied_on_disk(
            "missing.txt",
            Some(temp.path()),
        ));
    }

    #[test]
    fn on_disk_resolves_glob_against_walk() {
        let temp = tempfile::tempdir().expect("temp");
        let sub = temp.path().join("docs/x");
        std::fs::create_dir_all(&sub).expect("mkdir");
        std::fs::write(sub.join("file.md"), "x").expect("write");
        assert!(pattern_is_satisfied_on_disk(
            "docs/x/*.md",
            Some(temp.path()),
        ));
    }
}
