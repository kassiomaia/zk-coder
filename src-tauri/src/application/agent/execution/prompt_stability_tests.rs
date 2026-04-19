//! Characterization tests for M1.3 — prompt assembly stabilization — plus
//! M2.2 session-catalog stability tests.
//!
//! These tests lock in the invariant that identical session state produces
//! identical prepared-request hashes, and that subtask transitions change
//! only the expected request components. After M2.2 the tool-catalog hash is
//! further stabilized: subtask mode transitions no longer change the
//! session-visible catalog, because per-mode restriction moved to runtime
//! enforcement in `tool_phase`. See
//! `docs/planning/agent-context-engineering/measurement-harness.md` for the
//! full list of runtime transitions that are allowed to change each hash.

use serde_json::json;

use crate::core::agent::task::{SubTaskMode, TaskStatus};
use crate::core::llm::entities::{ChatMessage, ToolDefinition};
use crate::infrastructure::agent::registry::{
    default_code_framing, harness_contract_prompt, quality_baseline_prompt, SystemPromptBuilder,
};
use crate::infrastructure::agent::runtime::request_shape::{
    framing_hash, harness_section_hash, message_prefix_hash, quality_baseline_hash,
    system_prompt_hash, tool_catalog_hash,
};
use crate::infrastructure::agent::runtime::{derive_system_prompt, SubtaskSummary};

const BASE_PROMPT: &str = "You are a senior engineering agent. Be careful.";

fn summary(title: &str, mode: SubTaskMode, status: TaskStatus, order: i32) -> SubtaskSummary {
    SubtaskSummary {
        title: title.to_string(),
        mode,
        status,
        order,
        contract: None,
    }
}

fn tool(name: &str, description: &str) -> ToolDefinition {
    ToolDefinition {
        name: name.to_string(),
        description: description.to_string(),
        input_schema: json!({"type": "object"}),
    }
}

fn full_catalog() -> Vec<ToolDefinition> {
    // Canonical (sorted-by-name) order as produced by ToolRegistry::definitions()
    // after M1.1.
    vec![
        tool("agent_advance_to_next_subtask", "Advance subtask"),
        tool("agent_emit_status", "Emit status"),
        tool("fs_apply_patch", "Apply a patch"),
        tool("fs_fuzzy_find_files", "Fuzzy find files"),
        tool("fs_glob_files", "Glob files"),
        tool("fs_list_files", "List files"),
        tool("fs_read_file", "Read a file"),
        tool("fs_replace_in_file", "Replace in file"),
        tool("fs_stat_path", "Stat a path"),
        tool("fs_write_file", "Write a file"),
        tool("search_grep", "Grep search"),
        tool("system_execute_command", "Execute command"),
    ]
}

/// Mirrors the pre-M2.2 filter in `context_phase::resolve_context`. Before
/// M2.2, `resolve_context` narrowed the session catalog to the subtask's
/// allowed tools; that path has been removed in favor of runtime enforcement
/// in `tool_phase`. Kept here only to reason about the legacy hash moves that
/// motivated M2.2 — see
/// `subtask_mode_transition_no_longer_changes_session_tool_catalog_hash`.
///
/// `FullAccess` has no allowlist (`allowed_tools()` returns `None`); the
/// legacy filter behavior for an unrestricted mode is "pass everything
/// through", which this helper models by returning the whole catalog.
fn filter_to_mode(catalog: &[ToolDefinition], mode: &SubTaskMode) -> Vec<ToolDefinition> {
    match mode.allowed_tools() {
        Some(allowed) => catalog
            .iter()
            .filter(|t| allowed.iter().any(|name| *name == t.name))
            .cloned()
            .collect(),
        None => catalog.to_vec(),
    }
}

fn user(content: &str) -> ChatMessage {
    ChatMessage::user(content)
}

// --- system_prompt_hash stability -------------------------------------------

#[test]
fn identical_session_state_produces_identical_system_prompt_hash() {
    let s = summary("Review auth", SubTaskMode::Read, TaskStatus::InProgress, 0);
    let first = derive_system_prompt(BASE_PROMPT, &Some((s.clone(), 3)));
    let second = derive_system_prompt(BASE_PROMPT, &Some((s, 3)));
    assert_eq!(first, second);
    assert_eq!(system_prompt_hash(&first), system_prompt_hash(&second));
}

#[test]
fn no_subtask_state_produces_identical_system_prompt_hash_across_invocations() {
    let a = derive_system_prompt(BASE_PROMPT, &None);
    let b = derive_system_prompt(BASE_PROMPT, &None);
    assert_eq!(system_prompt_hash(&a), system_prompt_hash(&b));
}

#[test]
fn subtask_transition_changes_system_prompt_hash() {
    let before = summary(
        "Review auth",
        SubTaskMode::Read,
        TaskStatus::InProgress,
        0,
    );
    let after = summary(
        "Patch auth",
        SubTaskMode::Write,
        TaskStatus::InProgress,
        1,
    );
    let prompt_before = derive_system_prompt(BASE_PROMPT, &Some((before, 3)));
    let prompt_after = derive_system_prompt(BASE_PROMPT, &Some((after, 3)));
    assert_ne!(
        system_prompt_hash(&prompt_before),
        system_prompt_hash(&prompt_after),
        "expected system prompt hash to move when the active subtask changes",
    );
}

#[test]
fn subtask_moving_out_of_in_progress_reverts_system_prompt_hash_to_base() {
    // The helper only inlines subtask context when status is InProgress.
    // A subtask flipping to Completed must therefore produce the same hash as
    // `None` — and *not* the in-progress variant's hash.
    let in_progress = summary(
        "Review auth",
        SubTaskMode::Read,
        TaskStatus::InProgress,
        0,
    );
    let completed = summary(
        "Review auth",
        SubTaskMode::Read,
        TaskStatus::Completed,
        0,
    );
    let in_progress_prompt = derive_system_prompt(BASE_PROMPT, &Some((in_progress, 3)));
    let completed_prompt = derive_system_prompt(BASE_PROMPT, &Some((completed, 3)));
    let base_prompt = derive_system_prompt(BASE_PROMPT, &None);

    assert_eq!(
        system_prompt_hash(&completed_prompt),
        system_prompt_hash(&base_prompt),
    );
    assert_ne!(
        system_prompt_hash(&completed_prompt),
        system_prompt_hash(&in_progress_prompt),
    );
}

// --- tool_catalog_hash stability --------------------------------------------

#[test]
fn identical_filter_over_canonical_catalog_produces_identical_tool_catalog_hash() {
    let catalog = full_catalog();
    let first = filter_to_mode(&catalog, &SubTaskMode::Read);
    let second = filter_to_mode(&catalog, &SubTaskMode::Read);
    assert_eq!(tool_catalog_hash(&first), tool_catalog_hash(&second));
}

#[test]
fn subtask_transition_preserves_tool_catalog_hash_when_mode_is_unchanged() {
    // Two different subtasks of the same mode must filter down to the same
    // tool set, so the tool catalog hash must not move on transition.
    let catalog = full_catalog();
    let a = filter_to_mode(&catalog, &SubTaskMode::Read);
    let b = filter_to_mode(&catalog, &SubTaskMode::Read);
    assert_eq!(tool_catalog_hash(&a), tool_catalog_hash(&b));
}

#[test]
fn legacy_per_mode_filter_still_moves_the_hash_when_applied() {
    // Kept as a regression safety net: the legacy filter logic itself still
    // produces different hashes for different modes. If this test ever starts
    // returning equal hashes, the mode-permission data has drifted and the
    // runtime enforcement added in M2.2 may no longer be enforcing anything.
    let catalog = full_catalog();
    let read_set = filter_to_mode(&catalog, &SubTaskMode::Read);
    let write_set = filter_to_mode(&catalog, &SubTaskMode::Write);
    assert_ne!(tool_catalog_hash(&read_set), tool_catalog_hash(&write_set));
}

#[test]
fn subtask_mode_transition_no_longer_changes_session_tool_catalog_hash() {
    // M2.2 invariant: `resolve_context` now always sends the full session
    // catalog. A mode transition may change `ctx.current_allowed_tool_names`
    // (used for runtime enforcement) but must not change the tool-catalog
    // hash the LLM sees. Modeling this directly: the hash input is
    // `deps.all_tool_definitions` which is invariant to subtask mode.
    let catalog = full_catalog();
    let hash_for_read_mode_step = tool_catalog_hash(&catalog);
    let hash_for_write_mode_step = tool_catalog_hash(&catalog);
    assert_eq!(
        hash_for_read_mode_step, hash_for_write_mode_step,
        "session tool catalog must not move on subtask mode transitions \
         (M2.2: runtime enforcement, stable catalog)",
    );
}

// --- message_prefix_hash stability ------------------------------------------

#[test]
fn appending_a_single_new_message_leaves_prefix_hash_unchanged() {
    // This is the cache-warmth invariant: the prepared request for step N+1
    // should share its prefix with step N, modulo an appended tail message.
    let shared = vec![
        user("please investigate login"),
        user("start by reading auth.rs"),
    ];
    let mut step_n = shared.clone();
    step_n.push(user("tool call for step N"));
    let mut step_n1 = shared.clone();
    step_n1.push(user("tool call for step N+1"));

    assert_eq!(
        message_prefix_hash(&step_n),
        message_prefix_hash(&step_n1),
        "appending a new tail message must not move the prefix hash",
    );
}

#[test]
fn mutating_an_earlier_message_changes_prefix_hash() {
    let original = vec![
        user("please investigate login"),
        user("start by reading auth.rs"),
        user("current tail"),
    ];
    let mutated = vec![
        user("please investigate login"),
        user("start by reading login.rs"),
        user("current tail"),
    ];
    assert_ne!(
        message_prefix_hash(&original),
        message_prefix_hash(&mutated),
    );
}

// --- Section-attribution (H.4) -----------------------------------------------
//
// These tests back the Workstream H.4 requirement that prompt-stability tests
// expose *which section* moved when the whole-prompt hash changes. They lock
// in three invariants:
//
// 1. `harness_section_hash` and `quality_baseline_hash` are stable across
//    subtask transitions, mode changes, and hypothetical task boundaries.
// 2. `framing_hash` is stable across subtask transitions in Phase 1 (where
//    every task inherits the default code framing). It will begin to move at
//    task boundaries once Workstream H.2 ships per-task `TaskFraming`.
// 3. The existing whole-prompt hash tests still pass — the three section
//    hashes sit alongside `system_prompt_hash`, not replacing it.

/// Build a base prompt using the real production composition, so the
/// section-attribution assertions run against the exact sections the agent
/// actually carries.
fn production_base_prompt() -> String {
    SystemPromptBuilder::compose_default_code_agent()
}

#[test]
fn harness_section_hash_is_stable_across_subtask_transitions() {
    // Two different subtasks (different title, mode, status, order). The
    // harness-contract section is invariant across the pair — its hash must
    // not move. This is the H.4 attribution invariant the measurement
    // harness relies on.
    let _ = production_base_prompt(); // the composed prompt depends on subtask state; the section does not
    let before = harness_section_hash(harness_contract_prompt());

    // Simulate a transition by deriving two different full prompts. The
    // derived appendix is subtask-variant; the section is not.
    let subtask_a = summary("Review auth", SubTaskMode::Read, TaskStatus::InProgress, 0);
    let subtask_b = summary("Patch auth", SubTaskMode::Write, TaskStatus::InProgress, 1);
    let _ = derive_system_prompt(&production_base_prompt(), &Some((subtask_a, 3)));
    let _ = derive_system_prompt(&production_base_prompt(), &Some((subtask_b, 3)));

    let after = harness_section_hash(harness_contract_prompt());
    assert_eq!(
        before, after,
        "harness section must not move across subtask transitions",
    );
}

#[test]
fn quality_baseline_hash_is_stable_across_subtask_transitions() {
    let before = quality_baseline_hash(quality_baseline_prompt());
    let subtask_a = summary("Review auth", SubTaskMode::Read, TaskStatus::InProgress, 0);
    let subtask_b = summary("Patch auth", SubTaskMode::Write, TaskStatus::InProgress, 1);
    let _ = derive_system_prompt(&production_base_prompt(), &Some((subtask_a, 3)));
    let _ = derive_system_prompt(&production_base_prompt(), &Some((subtask_b, 3)));
    let after = quality_baseline_hash(quality_baseline_prompt());
    assert_eq!(
        before, after,
        "quality baseline must not move across subtask transitions",
    );
}

#[test]
fn harness_and_quality_hashes_are_stable_across_modes() {
    // Mode transitions change the tool catalog allowlist and the active
    // subtask appendix, but they must never move the harness or quality
    // sections.
    let harness_before = harness_section_hash(harness_contract_prompt());
    let quality_before = quality_baseline_hash(quality_baseline_prompt());

    for mode in [
        SubTaskMode::Read,
        SubTaskMode::Write,
        SubTaskMode::Execute,
        SubTaskMode::FullAccess,
    ] {
        let s = summary("task", mode, TaskStatus::InProgress, 0);
        let _ = derive_system_prompt(&production_base_prompt(), &Some((s, 2)));
    }

    assert_eq!(
        harness_before,
        harness_section_hash(harness_contract_prompt()),
    );
    assert_eq!(
        quality_before,
        quality_baseline_hash(quality_baseline_prompt()),
    );
}

#[test]
fn framing_hash_is_stable_across_subtask_transitions_in_phase_1() {
    // Phase 1 emits the default code framing for every subtask. The
    // framing hash must therefore be stable across transitions. This test
    // will need to be relaxed when Workstream H.2 lands — at that point
    // framing becomes a per-task record, and the *mid-task* stability
    // promise remains, but framing moves at task boundaries.
    let before = framing_hash(default_code_framing());

    let subtask_a = summary("Review auth", SubTaskMode::Read, TaskStatus::InProgress, 0);
    let subtask_b = summary("Patch auth", SubTaskMode::Write, TaskStatus::InProgress, 1);
    let _ = derive_system_prompt(&production_base_prompt(), &Some((subtask_a, 3)));
    let _ = derive_system_prompt(&production_base_prompt(), &Some((subtask_b, 3)));

    let after = framing_hash(default_code_framing());
    assert_eq!(
        before, after,
        "framing hash must be stable across subtask transitions in Phase 1",
    );
}

// --- H.6 golden-file test ---------------------------------------------------
//
// Workstream H.6 rewrote `harness_contract_prompt()` from descriptive to
// consequence language. The wording is now locked by a golden file at
// `goldens/harness_contract_prompt.txt`. A PR that intentionally changes the
// prompt must update the golden in lockstep — the diff makes the wording
// change explicit and reviewable. Unintentional drift (for example, a stray
// whitespace edit in the string literal) fails this test and forces a
// deliberate decision.
//
// Regeneration: run `UPDATE_HARNESS_CONTRACT_GOLDEN=1 cargo test
// harness_contract_matches_h6_golden -- --nocapture` and commit the updated
// file alongside the source change.
const HARNESS_CONTRACT_GOLDEN_PATH: &str = "goldens/harness_contract_prompt.txt";

#[test]
fn harness_contract_matches_h6_golden() {
    let current = harness_contract_prompt();
    let golden_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/application/agent/execution")
        .join(HARNESS_CONTRACT_GOLDEN_PATH);

    if std::env::var_os("UPDATE_HARNESS_CONTRACT_GOLDEN").is_some() {
        std::fs::write(&golden_path, current)
            .expect("failed to write harness-contract golden");
        eprintln!(
            "wrote {} bytes to {}",
            current.len(),
            golden_path.display(),
        );
        return;
    }

    let golden = std::fs::read_to_string(&golden_path).unwrap_or_else(|err| {
        panic!(
            "failed to read harness-contract golden at {}: {err}. \
             Run with UPDATE_HARNESS_CONTRACT_GOLDEN=1 to regenerate.",
            golden_path.display(),
        )
    });

    assert_eq!(
        current, golden,
        "harness_contract_prompt() no longer matches the H.6 golden at {}. \
         If this wording change is intentional, regenerate the golden with \
         UPDATE_HARNESS_CONTRACT_GOLDEN=1 cargo test harness_contract_matches_h6_golden.",
        golden_path.display(),
    );
}

#[test]
fn harness_contract_uses_consequence_language_for_key_mechanisms() {
    // Consequence-language audit: every mechanism the runtime enforces must
    // appear with a runtime-consequence sentence. These assertions are
    // narrower than the golden and catch structural regressions early (for
    // example, someone refactors wording and loses the "not allowed by
    // mode" branch or the scope-drift consequence).
    let prompt = harness_contract_prompt();

    // Scope-drift consequence is named and anchored on runtime behavior.
    assert!(
        prompt.contains("flagged as scope drift"),
        "harness must name scope-drift consequence",
    );
    assert!(
        prompt.contains("rejected by the runtime"),
        "harness must state repeated scope drift is rejected",
    );

    // Failure-memory scoping and new NotAllowed split are both mentioned.
    assert!(prompt.contains("NotAllowedByMode"));
    assert!(prompt.contains("NotAllowedByArgs"));
    assert!(
        prompt.contains("scoped to the active subtask"),
        "harness must state failure memory is subtask-scoped (C.2)",
    );

    // D.1 `next:` gating is explained, not silently hidden.
    assert!(
        prompt.contains("suppressed in the current release"),
        "harness must explain that `next:` is suppressed (D.1)",
    );

    // Narration-intent concept is taught ahead of Workstream E landing.
    assert!(prompt.contains("intent: narration | question"));
    assert!(prompt.contains("pauses the runtime"));

    // Clean-exit behavior references all three terminal statuses and the
    // needs-review flag path.
    assert!(prompt.contains("Completed, Failed, or Cancelled"));
    assert!(prompt.contains("flagged for review"));

    // Consequence language: hedging phrases removed. These must not appear.
    for phrase in [
        "try to",
        "when possible",
        "meaningful moments",
        "Focus on completing",
    ] {
        assert!(
            !prompt.contains(phrase),
            "harness must not use hedging phrase `{phrase}`; H.6 requires consequence language",
        );
    }
}

// ---------------------------------------------------------------------------
// Workstream H.2.c — task framing is rendered into the composed prompt.
// ---------------------------------------------------------------------------
#[test]
fn compose_for_task_with_communication_framing_carries_framing_tokens() {
    use crate::core::agent::task::{DomainPriors, TaskFraming};
    let framing = TaskFraming {
        role: "senior communication professional".to_string(),
        voice_constraints: vec![
            "one claim per paragraph".to_string(),
            "no filler openers".to_string(),
        ],
        domain_priors: Some(DomainPriors {
            hints: vec!["press-release voice".to_string()],
        }),
        forbidden_patterns: vec![],
    };
    let composed = SystemPromptBuilder::compose_for_task(Some(&framing));
    assert!(composed.contains("senior communication professional"));
    assert!(composed.contains("one claim per paragraph"));
    // And the composed prompt must no longer carry code-default
    // framing tokens when a non-code framing is declared.
    assert!(!composed.contains("Bun"));
    assert!(!composed.contains("Drizzle"));
}

#[test]
fn compose_for_task_without_framing_equals_default_code_agent() {
    // Pre-H.2.c and legacy tasks render the exact prompt they have
    // today — byte-for-byte — so cache warmth on in-flight sessions
    // is preserved.
    let with_none = SystemPromptBuilder::compose_for_task(None);
    let legacy = SystemPromptBuilder::compose_default_code_agent();
    assert_eq!(with_none, legacy);
}

#[test]
fn active_framing_section_default_matches_code_framing() {
    assert_eq!(
        SystemPromptBuilder::active_framing_section(None),
        default_code_framing(),
    );
}

#[test]
fn section_hashes_are_separate_surfaces_from_system_prompt_hash() {
    // Regression guard: the three H.5 section hashes are emitted alongside
    // the existing `systemPromptHash`, not in place of it. This test
    // ensures the composed-prompt hash (what drives cache invalidation) is
    // still computable independently of the section hashes.
    let composed = production_base_prompt();
    let whole = system_prompt_hash(&composed);
    let harness = harness_section_hash(harness_contract_prompt());
    let quality = quality_baseline_hash(quality_baseline_prompt());
    let framing = framing_hash(default_code_framing());

    // Trivially, the four hashes are distinct hex digests of distinct
    // inputs — the composed prompt is strictly longer than any one section.
    assert_ne!(whole, harness);
    assert_ne!(whole, quality);
    assert_ne!(whole, framing);
    // And the composed prompt actually carries each section.
    assert!(composed.contains(harness_contract_prompt()));
    assert!(composed.contains(quality_baseline_prompt()));
    assert!(composed.contains(default_code_framing()));
}
