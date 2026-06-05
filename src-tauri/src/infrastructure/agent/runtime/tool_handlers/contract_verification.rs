//! Workstream F.1 — contract artifact verification and supporting
//! helpers for the agent permissions pipeline.
//!
//! Gates a subtask's transition to `Completed` on the presence of the
//! declared `expected_artifacts`. Ground truth is the workspace
//! filesystem; when no `workspace_root` is available, the overlay of
//! recorded touched files fills in. Returns a structured failure that
//! the caller surfaces as a `contract_unsatisfied` tool-level error.
//!
//! Extracted from `permissions.rs` in Phase 4 of the permissions-
//! decomposition refactor.

use crate::core::agent::session::AgentSession;
use crate::core::agent::task::SubTaskContract;
use crate::core::llm::entities::ToolCall;
use crate::infrastructure::agent::runtime::artifact_probe::{
    pattern_is_satisfied_on_disk, pattern_matches_touched_overlay,
};
use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
use crate::infrastructure::tools::OrchestratorToolResult;

/// Workstream F.1 — verify that a subtask's contract artifacts exist
/// before the handler allows a transition to `Completed`.
///
/// Ground truth is the filesystem: for each `ArtifactSpec` in the
/// contract, every file the glob resolves to must exist on disk. When
/// the session has no `workspace_root`, glob evaluation would be
/// ambiguous, so the verification falls back to the
/// `agent_files_touched` overlay and treats a pattern as satisfied when
/// any recorded file path matches it.
///
/// Returns `Ok(())` when every `path_pattern` resolves; otherwise
/// returns the list of unsatisfied patterns plus a de-duplicated list
/// of concrete missing paths (empty when the pattern had no match at
/// all).
pub(crate) fn verify_contract_artifacts(
    contract: &SubTaskContract,
    session: &AgentSession,
    telemetry_repo: &dyn AgentTelemetryRepository,
) -> Result<(), ContractVerificationFailure> {
    use globset::{Glob, GlobSetBuilder};

    let workspace_root: Option<&std::path::Path> = session.workspace_root.as_deref();

    // Build the overlay matcher once; used as fallback when the
    // filesystem check cannot evaluate (no workspace root) and as an
    // auxiliary signal when on-disk globbing fails.
    let touched_paths: Vec<String> = telemetry_repo
        .get_session_files(&session.id)
        .ok()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            let action = entry.action.to_ascii_lowercase();
            if matches!(action.as_str(), "write" | "delete") {
                Some(entry.file_path)
            } else {
                None
            }
        })
        .collect();

    let mut unsatisfied_patterns: Vec<String> = Vec::new();
    for spec in &contract.expected_artifacts {
        let pattern = spec.path_pattern.trim();
        if pattern.is_empty() {
            unsatisfied_patterns.push(spec.path_pattern.clone());
            continue;
        }
        if pattern_is_satisfied_on_disk(pattern, workspace_root) {
            continue;
        }
        if pattern_matches_touched_overlay(pattern, &touched_paths) {
            continue;
        }
        unsatisfied_patterns.push(spec.path_pattern.clone());
    }

    if unsatisfied_patterns.is_empty() {
        return Ok(());
    }

    // Provide the model with a concrete path list where possible so
    // the rejection message is actionable. Empty when patterns
    // genuinely produced zero matches.
    let mut missing_paths: Vec<String> = Vec::new();
    if let Some(root) = workspace_root {
        let mut builder = GlobSetBuilder::new();
        for pattern in &unsatisfied_patterns {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }
        if let Ok(matcher) = builder.build() {
            if matcher.len() > 0 {
                // Walk each pattern's literal-prefix parent when
                // evaluable. The globset crate does not provide a
                // walk-and-match primitive, so we surface pattern-level
                // info rather than false concrete paths; the model is
                // expected to produce the declared artifacts, not
                // inspect the workspace.
                let _ = root; // keep `root` binding intentional
                let _ = matcher;
            }
        }
    }

    for path in &touched_paths {
        // Surface touched paths that *almost* satisfy the contract — a
        // path recorded in `agent_files_touched` but that no
        // `expected_artifacts` glob accepted. This is the concrete
        // signal the model can act on.
        let mut builder = GlobSetBuilder::new();
        for pattern in &contract.expected_artifacts {
            if let Ok(glob) = Glob::new(&pattern.path_pattern) {
                builder.add(glob);
            }
        }
        if let Ok(matcher) = builder.build() {
            if !matcher.is_match(path) && !missing_paths.contains(path) {
                missing_paths.push(path.clone());
            }
        }
    }

    Err(ContractVerificationFailure {
        missing_patterns: unsatisfied_patterns,
        concrete_missing_paths: missing_paths,
    })
}

/// Payload carried by `contract_unsatisfied` errors so the model sees
/// both the patterns that did not resolve and any concrete paths that
/// were written outside the contract (helpful when scope drift landed).
#[derive(Debug, Clone)]
pub(crate) struct ContractVerificationFailure {
    pub(crate) missing_patterns: Vec<String>,
    pub(crate) concrete_missing_paths: Vec<String>,
}

impl ContractVerificationFailure {
    pub(crate) fn into_error_result(self, call: &ToolCall) -> OrchestratorToolResult {
        let missing = self
            .missing_patterns
            .iter()
            .map(|p| serde_json::Value::String(p.clone()))
            .collect::<Vec<_>>();
        let concrete = self
            .concrete_missing_paths
            .iter()
            .map(|p| serde_json::Value::String(p.clone()))
            .collect::<Vec<_>>();
        OrchestratorToolResult {
            call_id: call.id.clone(),
            name: call.name.clone(),
            output: serde_json::json!({
                "error": "Cannot advance subtask: contract_unsatisfied. The subtask's expected outputs do not exist on disk or in the session's file-touch overlay.",
                "reason": "contract_unsatisfied",
                "missingPatterns": missing,
                "concreteMissingPaths": concrete,
            }),
            is_error: true,
        }
    }
}

/// Returns `true` when the session's telemetry has at least one
/// successful file-or-command tool call linked to the given subtask.
pub(crate) fn has_subtask_evidence(
    session: &AgentSession,
    subtask_id: &str,
    telemetry_repo: &dyn AgentTelemetryRepository,
) -> bool {
    telemetry_repo
        .get_tool_calls_for_session(&session.id)
        .ok()
        .map(|calls| {
            calls.into_iter().any(|c| {
                c.subtask_id.as_deref() == Some(subtask_id)
                    && !c.is_error
                    && matches!(
                        c.tool_name.as_str(),
                        "fs_write_file" | "fs_delete_file" | "system_execute_command"
                    )
            })
        })
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Workstream F.1 — contract verification tests migrated from permissions.rs.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod contract_verification_tests {
    use super::*;
    use crate::core::agent::session::{AgentSession, SessionId};
    use crate::core::agent::task::{ArtifactKind, ArtifactSpec, SubTaskContract};
    use crate::core::agent::types::AgentId;
    use crate::infrastructure::agent::session_telemetry::AgentTelemetryRepository;
    use crate::infrastructure::sqlite::repositories::agent::{
        SessionRepositoryError, TerminalLineEntry, ToolCallEntry, TouchedFileEntry,
    };
    use serde_json::Value;
    use std::path::PathBuf;
    use std::sync::Mutex;

    /// Minimal telemetry double that only needs to answer
    /// `get_session_files`. Everything else is a no-op so the test
    /// focuses on the overlay-check path.
    struct OverlayTelemetry {
        files: Mutex<Vec<TouchedFileEntry>>,
    }

    impl OverlayTelemetry {
        fn new(files: Vec<(&str, &str)>) -> Self {
            let files = files
                .into_iter()
                .enumerate()
                .map(|(i, (path, action))| TouchedFileEntry {
                    id: format!("id-{i}"),
                    session_id: "session".to_string(),
                    file_path: path.to_string(),
                    action: action.to_string(),
                    diff: None,
                    timestamp: "2026-04-18T00:00:00Z".to_string(),
                })
                .collect();
            Self {
                files: Mutex::new(files),
            }
        }
    }

    impl AgentTelemetryRepository for OverlayTelemetry {
        fn create_execution_step(
            &self,
            _step: &crate::core::agent::task::ExecutionStep,
            _session_id: &str,
            _task_id: Option<&str>,
            _subtask_id: Option<&str>,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn get_session(
            &self,
            _session_id: &SessionId,
        ) -> Result<Option<AgentSession>, SessionRepositoryError> {
            Ok(None)
        }
        fn get_session_execution_steps(
            &self,
            _session_id: &SessionId,
        ) -> Result<Vec<crate::core::agent::task::ExecutionStep>, SessionRepositoryError> {
            Ok(vec![])
        }
        fn add_file_touched(
            &self,
            _session_id: &SessionId,
            _file_path: &str,
            _action: &str,
            _diff: Option<&str>,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn get_session_files(
            &self,
            _session_id: &SessionId,
        ) -> Result<Vec<TouchedFileEntry>, SessionRepositoryError> {
            Ok(self.files.lock().unwrap().clone())
        }
        fn add_terminal_line(
            &self,
            _session_id: &SessionId,
            _line_type: &str,
            _content: &str,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn get_session_terminal_output(
            &self,
            _session_id: &SessionId,
        ) -> Result<Vec<TerminalLineEntry>, SessionRepositoryError> {
            Ok(vec![])
        }
        fn add_status_phrase(
            &self,
            _session_id: &SessionId,
            _phrase: &str,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn get_session_status_phrases(
            &self,
            _session_id: &SessionId,
        ) -> Result<Vec<String>, SessionRepositoryError> {
            Ok(vec![])
        }
        fn clear_session_runtime_data(
            &self,
            _session_id: &SessionId,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn insert_tool_call(
            &self,
            _session_id: &SessionId,
            _tool_call_id: &str,
            _tool_name: &str,
            _arguments: &Value,
            _subtask_id: Option<&str>,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn update_tool_call_result(
            &self,
            _tool_call_id: &str,
            _result: &Value,
            _is_error: bool,
            _duration_ms: Option<i64>,
        ) -> Result<(), SessionRepositoryError> {
            Ok(())
        }
        fn get_tool_calls_for_session(
            &self,
            _session_id: &SessionId,
        ) -> Result<Vec<ToolCallEntry>, SessionRepositoryError> {
            Ok(vec![])
        }
    }

    fn session_with_workspace(root: Option<PathBuf>) -> AgentSession {
        let mut s = AgentSession::new(
            AgentId::new("code-agent"),
            "Code".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            root,
            None,
        );
        s.id = SessionId("session".to_string());
        s
    }

    fn contract(pattern: &str, kind: ArtifactKind) -> SubTaskContract {
        SubTaskContract {
            expected_artifacts: vec![ArtifactSpec {
                path_pattern: pattern.to_string(),
                kind,
            }],
        }
    }

    #[test]
    fn contract_fails_when_neither_disk_nor_overlay_have_the_artifact() {
        let temp = tempfile::tempdir().expect("temp");
        let session = session_with_workspace(Some(temp.path().to_path_buf()));
        let telemetry = OverlayTelemetry::new(vec![]);
        let c = contract("docs/communications/social/*.md", ArtifactKind::File);
        let err = verify_contract_artifacts(&c, &session, &telemetry)
            .expect_err("missing artifacts must fail verification");
        assert_eq!(err.missing_patterns.len(), 1);
        assert!(err.missing_patterns[0].contains("social"));
    }

    #[test]
    fn contract_passes_when_file_exists_on_disk() {
        let temp = tempfile::tempdir().expect("temp");
        let social = temp.path().join("docs/communications/social");
        std::fs::create_dir_all(&social).expect("mkdir");
        std::fs::write(social.join("twitter-launch.md"), "x").expect("write");

        let session = session_with_workspace(Some(temp.path().to_path_buf()));
        let telemetry = OverlayTelemetry::new(vec![]);
        let c = contract("docs/communications/social/*.md", ArtifactKind::File);
        verify_contract_artifacts(&c, &session, &telemetry)
            .expect("existing file on disk satisfies the pattern");
    }

    #[test]
    fn contract_passes_when_overlay_records_a_matching_write() {
        let session = session_with_workspace(None);
        let telemetry =
            OverlayTelemetry::new(vec![("docs/communications/social/twitter.md", "write")]);
        let c = contract("docs/communications/social/*.md", ArtifactKind::File);
        verify_contract_artifacts(&c, &session, &telemetry)
            .expect("overlay write matching the glob satisfies the pattern");
    }

    #[test]
    fn contract_rejection_includes_out_of_contract_touched_paths() {
        let session = session_with_workspace(None);
        let telemetry = OverlayTelemetry::new(vec![
            ("docs/communications/email/outreach.md", "write"),
            ("docs/communications/email/launch.md", "write"),
        ]);
        let c = contract("docs/communications/social/*.md", ArtifactKind::File);
        let err = verify_contract_artifacts(&c, &session, &telemetry)
            .expect_err("drifted writes do not satisfy the contract");
        assert_eq!(err.missing_patterns.len(), 1);
        assert!(err
            .concrete_missing_paths
            .iter()
            .any(|p| p.contains("email/outreach.md")));
        assert!(err
            .concrete_missing_paths
            .iter()
            .any(|p| p.contains("email/launch.md")));
    }

    #[test]
    fn verification_failure_serializes_with_contract_unsatisfied_reason() {
        let call = ToolCall {
            id: "call-1".to_string(),
            name: "agent_advance_to_next_subtask".to_string(),
            arguments: serde_json::json!({}),
        };
        let failure = ContractVerificationFailure {
            missing_patterns: vec!["docs/social/*.md".to_string()],
            concrete_missing_paths: vec!["docs/email/x.md".to_string()],
        };
        let result = failure.into_error_result(&call);
        assert!(result.is_error);
        assert_eq!(
            result.output.get("reason").and_then(|v| v.as_str()),
            Some("contract_unsatisfied"),
        );
        let patterns = result
            .output
            .get("missingPatterns")
            .and_then(|v| v.as_array())
            .expect("missingPatterns");
        assert_eq!(patterns.len(), 1);
        let concrete = result
            .output
            .get("concreteMissingPaths")
            .and_then(|v| v.as_array())
            .expect("concreteMissingPaths");
        assert_eq!(concrete.len(), 1);
    }
}
