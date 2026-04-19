//! Tests for 
//! Extracted into a sibling file during Phase 6 of the permissions-
//! decomposition refactor to keep the host module under the 800-LoC cap.

//! Workstream G.1 — plan-quality validation (overlap + no-op final).
use super::*;

// -------- overlap detection --------------------------------------------

#[test]
fn overlap_rejects_byte_identical_patterns_on_different_subtasks() {
    let subtasks = vec![
        serde_json::json!({
            "title": "Write social copy",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/communications/social/*.md", "kind": "file"}
                ]
            }
        }),
        serde_json::json!({
            "title": "Write launch email",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/communications/social/*.md", "kind": "file"}
                ]
            }
        }),
    ];
    let err = detect_overlapping_contracts(&subtasks)
        .expect_err("identical patterns must be rejected");
    assert!(err.contains("Write social copy"), "message: {err}");
    assert!(err.contains("Write launch email"), "message: {err}");
    assert!(err.contains("overlapping"), "message: {err}");
}

#[test]
fn overlap_rejects_prefix_relationship_patterns_on_different_subtasks() {
    let subtasks = vec![
        serde_json::json!({
            "title": "Write all communications",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/communications/**/*.md", "kind": "file"}
                ]
            }
        }),
        serde_json::json!({
            "title": "Write social copy",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/communications/social/*.md", "kind": "file"}
                ]
            }
        }),
    ];
    let err = detect_overlapping_contracts(&subtasks)
        .expect_err("prefix-related patterns must be rejected");
    assert!(err.contains("overlapping"), "message: {err}");
}

#[test]
fn overlap_accepts_distinct_neighbouring_patterns() {
    let subtasks = vec![
        serde_json::json!({
            "title": "Write tweets",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/communications/social/twitter-*.md", "kind": "file"}
                ]
            }
        }),
        serde_json::json!({
            "title": "Write posts",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/communications/social/linkedin-*.md", "kind": "file"}
                ]
            }
        }),
    ];
    assert!(
        detect_overlapping_contracts(&subtasks).is_ok(),
        "sibling-pattern plan must be accepted"
    );
}

#[test]
fn overlap_accepts_plan_without_contracts() {
    let subtasks = vec![
        serde_json::json!({"title": "Analyze code"}),
        serde_json::json!({"title": "Implement feature"}),
        serde_json::json!({"title": "Verify tests"}),
    ];
    assert!(
        detect_overlapping_contracts(&subtasks).is_ok(),
        "legacy plans must not regress"
    );
}

#[test]
fn overlap_permits_multiple_patterns_on_the_same_subtask() {
    let subtasks = vec![serde_json::json!({
        "title": "Write release bundle",
        "contract": {
            "expected_artifacts": [
                {"path_pattern": "docs/release/notes.md", "kind": "file"},
                {"path_pattern": "docs/release/*.md", "kind": "file"}
            ]
        }
    })];
    assert!(
        detect_overlapping_contracts(&subtasks).is_ok(),
        "intra-subtask patterns must not collide"
    );
}

#[test]
fn overlap_flags_literal_path_matched_by_other_glob() {
    let subtasks = vec![
        serde_json::json!({
            "title": "Write twitter copy",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/social/twitter.md", "kind": "file"}
                ]
            }
        }),
        serde_json::json!({
            "title": "Write social copy",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/social/*.md", "kind": "file"}
                ]
            }
        }),
    ];
    assert!(detect_overlapping_contracts(&subtasks).is_err());
}

#[test]
fn overlap_helper_rejects_proper_prefix_alignment() {
    assert!(patterns_overlap(
        "docs/social/**/*.md",
        "docs/social/twitter/*.md"
    ));
}

#[test]
fn overlap_helper_accepts_sibling_directories() {
    assert!(!patterns_overlap(
        "docs/a/*.md",
        "docs/alpha/*.md"
    ));
}

#[test]
fn overlap_helper_accepts_patterns_with_no_literal_anchor() {
    assert!(!patterns_overlap("*.md", "*.txt"));
}

// -------- no-op final subtask -----------------------------------------

#[test]
fn noop_rejects_documentation_of_prior_work_with_empty_contract() {
    let subtasks = vec![
        serde_json::json!({
            "title": "Draft press release",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/communications/press-release.md", "kind": "file"}
                ]
            }
        }),
        serde_json::json!({
            "title": "Generate documentation of prior subtasks in ./docs/communications/"
        }),
    ];
    let err = detect_noop_final_subtask(&subtasks)
        .expect_err("redundant documentation subtask must be rejected");
    assert!(err.contains("no-op"), "message: {err}");
    assert!(
        err.contains("documentation of"),
        "message must cite the matched trigger: {err}"
    );
}

#[test]
fn noop_rejects_summary_subtask_with_no_contract() {
    let subtasks = vec![
        serde_json::json!({"title": "Draft release notes"}),
        serde_json::json!({"title": "Summarize launch communications"}),
    ];
    assert!(detect_noop_final_subtask(&subtasks).is_err());
}

#[test]
fn noop_accepts_summary_subtask_when_contract_declares_output() {
    let subtasks = vec![
        serde_json::json!({"title": "Draft release notes"}),
        serde_json::json!({
            "title": "Summarize launch communications",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/communications/INDEX.md", "kind": "file"}
                ]
            }
        }),
    ];
    assert!(detect_noop_final_subtask(&subtasks).is_ok());
}

#[test]
fn noop_accepts_single_subtask_plan() {
    let subtasks = vec![serde_json::json!({
        "title": "Summarize findings"
    })];
    assert!(detect_noop_final_subtask(&subtasks).is_ok());
}

#[test]
fn noop_accepts_plan_whose_final_subtask_is_not_a_summary() {
    let subtasks = vec![
        serde_json::json!({"title": "Analyze requirements"}),
        serde_json::json!({"title": "Implement changes"}),
        serde_json::json!({"title": "Verify with tests"}),
    ];
    assert!(detect_noop_final_subtask(&subtasks).is_ok());
}

#[test]
fn noop_trigger_is_case_insensitive() {
    let subtasks = vec![
        serde_json::json!({"title": "Draft press release"}),
        serde_json::json!({
            "title": "LIST OF FILES produced by prior subtasks"
        }),
    ];
    assert!(detect_noop_final_subtask(&subtasks).is_err());
}

#[test]
fn noop_checks_description_field_too() {
    let subtasks = vec![
        serde_json::json!({"title": "Draft press release"}),
        serde_json::json!({
            "title": "Wrap up",
            "description": "Generate a recap of the work done in the prior subtasks."
        }),
    ];
    assert!(detect_noop_final_subtask(&subtasks).is_err());
}

// -------- integration through validate_plan_subtasks ------------------

#[test]
fn validate_plan_subtasks_rejects_overlapping_contracts_with_clear_message() {
    let subtasks = vec![
        serde_json::json!({
            "title": "Implement social copy",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/social/*.md", "kind": "file"}
                ]
            }
        }),
        serde_json::json!({
            "title": "Implement email copy",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/social/*.md", "kind": "file"}
                ]
            }
        }),
        serde_json::json!({"title": "Verify outputs"}),
    ];
    let err = validate_plan_subtasks(&subtasks, false)
        .expect_err("overlapping plans must fail validation");
    assert!(err.contains("overlapping"), "message: {err}");
}

#[test]
fn validate_plan_subtasks_rejects_redundant_final_summary_subtask() {
    let subtasks = vec![
        serde_json::json!({"title": "Analyze existing copy"}),
        serde_json::json!({"title": "Implement new copy"}),
        serde_json::json!({
            "title": "Write documentation of what was done"
        }),
    ];
    let err = validate_plan_subtasks(&subtasks, false)
        .expect_err("redundant final summary must fail validation");
    assert!(err.contains("no-op"), "message: {err}");
}

#[test]
fn validate_plan_subtasks_accepts_clean_contracted_plan() {
    let subtasks = vec![
        serde_json::json!({
            "title": "Implement tweet copy",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/social/twitter-*.md", "kind": "file"}
                ]
            }
        }),
        serde_json::json!({
            "title": "Implement post copy",
            "contract": {
                "expected_artifacts": [
                    {"path_pattern": "docs/social/linkedin-*.md", "kind": "file"}
                ]
            }
        }),
    ];
    assert!(validate_plan_subtasks(&subtasks, false).is_ok());
}

#[test]
fn validate_plan_subtasks_accepts_legacy_contract_free_plan() {
    let subtasks = vec![
        serde_json::json!({"title": "Analyze structure"}),
        serde_json::json!({"title": "Implement update"}),
        serde_json::json!({"title": "Verify behavior"}),
    ];
    assert!(validate_plan_subtasks(&subtasks, false).is_ok());
}
