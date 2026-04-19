//! Tests for 
//! Extracted into a sibling file during Phase 6 of the permissions-
//! decomposition refactor to keep the host module under the 800-LoC cap.

use super::*;
use crate::core::agent::repositories::TaskRepository as _;
use crate::core::agent::session::{AgentSession, SessionId};
use crate::core::agent::task::{
    AgentRequest, AgentTask, ArtifactKind, ArtifactSpec, SubTask, SubTaskContract, SubTaskMode,
    TaskStatus,
};
use crate::core::agent::types::AgentId;
use crate::infrastructure::sqlite::repositories::agent::SqliteSessionRepository;
use serde_json::{json, Value};
use std::sync::Arc;

fn make_repo_with_task() -> (Arc<SqliteSessionRepository>, String) {
    let repo = Arc::new(
        SqliteSessionRepository::new(":memory:").expect("in-memory repo"),
    );
    let session = AgentSession::new(
        AgentId::new("agent"),
        "Agent".to_string(),
        "conv".to_string(),
        "thread".to_string(),
        None,
        None,
    );
    repo.as_ref()
        .create_session(&session)
        .expect("session create");
    let task = AgentTask::new(
        session.id.clone(),
        AgentRequest {
            content: "test request".to_string(),
            attachments: vec![],
            context: None,
        },
    );
    let task_id = task.id.clone();
    <SqliteSessionRepository as TaskRepository>::create_task(repo.as_ref(), &task)
        .expect("task create");
    (repo, task_id)
}

fn persist_skeleton(
    repo: &SqliteSessionRepository,
    task_id: &str,
    titles: &[&str],
) -> Vec<SubTask> {
    let mut created = Vec::new();
    for (idx, title) in titles.iter().enumerate() {
        let subtask = SubTask::new(
            task_id.to_string(),
            (*title).to_string(),
            SubTaskMode::FullAccess,
            idx as i32,
        );
        <SqliteSessionRepository as TaskRepository>::create_subtask(repo, &subtask)
            .expect("subtask create");
        created.push(subtask);
    }
    created
}

fn make_session() -> AgentSession {
    AgentSession::new(
        AgentId::new("agent"),
        "Agent".to_string(),
        "conv".to_string(),
        "thread".to_string(),
        None,
        None,
    )
}

fn make_call(args: Value) -> ToolCall {
    ToolCall {
        id: "call-1".to_string(),
        name: "agent_plan_subtasks".to_string(),
        arguments: args,
    }
}

fn contract_for_path(path: &str) -> Value {
    json!({
        "expected_artifacts": [
            {"path_pattern": path, "kind": "file"}
        ]
    })
}

#[test]
fn merge_attaches_contract_and_refines_fields_on_pending_skeleton() {
    let (repo, task_id) = make_repo_with_task();
    persist_skeleton(repo.as_ref(), &task_id, &[
        "Investigate auth module",
        "Implement fix",
        "Verify behavior",
    ]);

    let existing =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    let session = make_session();
    let app = tauri::test::mock_app();
    let app_handle = app.handle().clone();
    let call = make_call(json!({}));

    let model_plan = vec![
        json!({
            "title": "Investigate auth module",
            "description": "Read auth/*.rs and map the current flow",
            "contract": contract_for_path("docs/auth/*.md"),
            "order": 0
        }),
        json!({
            "title": "Implement session-handoff fix",
            "description": "Update auth/session.rs and add regression tests",
            "contract": contract_for_path("src-tauri/src/core/auth/**/*.rs"),
            "mode": "write",
            "order": 1
        }),
        json!({
            "title": "Verify behavior",
            "contract": contract_for_path("tests/auth/*.rs"),
            "order": 2
        }),
    ];

    let result = merge_model_plan_into_existing(
        &call,
        &session,
        &task_id,
        &model_plan,
        &existing,
        &app_handle,
        repo.as_ref(),
    );

    assert!(!result.is_error, "merge should succeed: {:?}", result.output);
    let refined = result.output.get("refined").and_then(Value::as_u64).unwrap();
    let appended = result.output.get("appended").and_then(Value::as_u64).unwrap();
    assert_eq!(refined, 3);
    assert_eq!(appended, 0);

    let after =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    assert_eq!(after.len(), 3);
    for st in &after {
        assert!(
            st.contract.is_some(),
            "order {} should have a contract after merge",
            st.order,
        );
    }
    let by_order: std::collections::HashMap<i32, &SubTask> =
        after.iter().map(|st| (st.order, st)).collect();
    assert_eq!(
        by_order[&1].title,
        "Implement session-handoff fix",
        "title should be updated on the pending slot",
    );
    assert_eq!(by_order[&1].mode, SubTaskMode::Write);
    assert_eq!(
        by_order[&0].description.as_deref(),
        Some("Read auth/*.rs and map the current flow"),
    );
}

#[test]
fn merge_appends_when_model_declares_more_subtasks() {
    let (repo, task_id) = make_repo_with_task();
    persist_skeleton(repo.as_ref(), &task_id, &["Plan", "Execute"]);

    let existing =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    let session = make_session();
    let app = tauri::test::mock_app();
    let app_handle = app.handle().clone();
    let call = make_call(json!({}));

    let model_plan = vec![
        json!({
            "title": "Plan",
            "contract": contract_for_path("docs/plan.md"),
            "order": 0
        }),
        json!({
            "title": "Execute",
            "contract": contract_for_path("src/*.rs"),
            "order": 1
        }),
        json!({
            "title": "Verify outputs",
            "contract": contract_for_path("tests/*.rs"),
            "order": 2
        }),
    ];

    let result = merge_model_plan_into_existing(
        &call,
        &session,
        &task_id,
        &model_plan,
        &existing,
        &app_handle,
        repo.as_ref(),
    );
    assert!(!result.is_error, "{:?}", result.output);
    assert_eq!(result.output["appended"], 1);
    assert_eq!(result.output["refined"], 2);

    let after =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    assert_eq!(after.len(), 3);
    assert_eq!(after[2].title, "Verify outputs");
    assert!(after[2].contract.is_some());
}

#[test]
fn merge_ignores_extras_when_model_declares_fewer_subtasks() {
    let (repo, task_id) = make_repo_with_task();
    persist_skeleton(repo.as_ref(), &task_id, &["A", "B", "C"]);

    let existing =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    let session = make_session();
    let app = tauri::test::mock_app();
    let app_handle = app.handle().clone();
    let call = make_call(json!({}));

    let model_plan = vec![json!({
        "title": "A",
        "contract": contract_for_path("a/*.md"),
        "order": 0
    })];

    let result = merge_model_plan_into_existing(
        &call,
        &session,
        &task_id,
        &model_plan,
        &existing,
        &app_handle,
        repo.as_ref(),
    );
    assert!(!result.is_error);

    let after =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    assert_eq!(after.len(), 3, "extra skeleton slots must be preserved");
    assert!(
        after[0].contract.is_some(),
        "the refined slot still receives its contract",
    );
    assert!(after[1].contract.is_none());
    assert!(after[2].contract.is_none());
}

#[test]
fn merge_refuses_to_redefine_completed_subtask_with_requires_replan() {
    let (repo, task_id) = make_repo_with_task();
    let created = persist_skeleton(repo.as_ref(), &task_id, &["Investigate", "Implement"]);
    <SqliteSessionRepository as TaskRepository>::update_subtask_status(
        repo.as_ref(),
        &created[0].id,
        TaskStatus::Completed,
    )
    .unwrap();

    let existing =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    let session = make_session();
    let app = tauri::test::mock_app();
    let app_handle = app.handle().clone();
    let call = make_call(json!({}));

    let model_plan = vec![
        json!({
            "title": "Investigate DIFFERENT scope",
            "description": "A materially different description",
            "contract": contract_for_path("x/*.md"),
            "order": 0
        }),
        json!({"title": "Implement", "order": 1}),
    ];

    let result = merge_model_plan_into_existing(
        &call,
        &session,
        &task_id,
        &model_plan,
        &existing,
        &app_handle,
        repo.as_ref(),
    );
    assert!(result.is_error);
    assert_eq!(result.output["requires_replan"], true);
    assert!(result
        .output
        .get("feedback")
        .and_then(Value::as_str)
        .unwrap()
        .contains("Completed"));
}

#[test]
fn merge_refuses_to_append_when_a_completed_subtask_exists() {
    let (repo, task_id) = make_repo_with_task();
    let created = persist_skeleton(repo.as_ref(), &task_id, &["First", "Second"]);
    <SqliteSessionRepository as TaskRepository>::update_subtask_status(
        repo.as_ref(),
        &created[0].id,
        TaskStatus::Completed,
    )
    .unwrap();

    let existing =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    let session = make_session();
    let app = tauri::test::mock_app();
    let app_handle = app.handle().clone();
    let call = make_call(json!({}));

    let model_plan = vec![
        json!({"title": "First", "order": 0}),
        json!({"title": "Second", "order": 1}),
        json!({"title": "Added later", "order": 2}),
    ];

    let result = merge_model_plan_into_existing(
        &call,
        &session,
        &task_id,
        &model_plan,
        &existing,
        &app_handle,
        repo.as_ref(),
    );
    assert!(result.is_error);
    assert_eq!(result.output["requires_replan"], true);
}

#[test]
fn merge_does_not_mutate_completed_subtask_when_shape_matches() {
    let (repo, task_id) = make_repo_with_task();
    let created = persist_skeleton(repo.as_ref(), &task_id, &["First", "Second"]);
    <SqliteSessionRepository as TaskRepository>::update_subtask_status(
        repo.as_ref(),
        &created[0].id,
        TaskStatus::Completed,
    )
    .unwrap();

    let existing =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    let session = make_session();
    let app = tauri::test::mock_app();
    let app_handle = app.handle().clone();
    let call = make_call(json!({}));

    let model_plan = vec![
        json!({"title": "First", "order": 0}),
        json!({
            "title": "Second",
            "contract": contract_for_path("src/*.rs"),
            "order": 1
        }),
    ];

    let result = merge_model_plan_into_existing(
        &call,
        &session,
        &task_id,
        &model_plan,
        &existing,
        &app_handle,
        repo.as_ref(),
    );
    assert!(!result.is_error, "{:?}", result.output);

    let after =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    assert_eq!(after[0].status, TaskStatus::Completed);
    assert!(
        after[0].contract.is_none(),
        "completed slot must not gain a contract it did not already have",
    );
    assert!(after[1].contract.is_some());
}

#[test]
fn merge_rejects_malformed_contract_with_structured_error() {
    let (repo, task_id) = make_repo_with_task();
    persist_skeleton(repo.as_ref(), &task_id, &["One"]);

    let existing =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    let session = make_session();
    let app = tauri::test::mock_app();
    let app_handle = app.handle().clone();
    let call = make_call(json!({}));

    let model_plan = vec![json!({
        "title": "One",
        "contract": { "expected_artifacts": "not-an-array" },
        "order": 0
    })];

    let result = merge_model_plan_into_existing(
        &call,
        &session,
        &task_id,
        &model_plan,
        &existing,
        &app_handle,
        repo.as_ref(),
    );
    assert!(result.is_error);
    let err = result.output.get("error").and_then(Value::as_str).unwrap();
    assert!(err.contains("malformed"), "got: {err}");
}

#[test]
fn merge_characterization_7_subtasks_all_gain_contract() {
    let (repo, task_id) = make_repo_with_task();
    let skeleton_titles = [
        "Plan architecture and tech stack",
        "Design database schema",
        "Scaffold project structure",
        "Implement auth endpoints",
        "Implement session persistence",
        "Wire up error middleware",
        "Run smoke tests",
    ];
    persist_skeleton(repo.as_ref(), &task_id, &skeleton_titles);

    let existing =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    let session = make_session();
    let app = tauri::test::mock_app();
    let app_handle = app.handle().clone();
    let call = make_call(json!({}));

    let model_plan: Vec<Value> = skeleton_titles
        .iter()
        .enumerate()
        .map(|(idx, title)| {
            json!({
                "title": title,
                "contract": contract_for_path(&format!("docs/phase-{idx}/*.md")),
                "order": idx
            })
        })
        .collect();

    let result = merge_model_plan_into_existing(
        &call,
        &session,
        &task_id,
        &model_plan,
        &existing,
        &app_handle,
        repo.as_ref(),
    );
    assert!(!result.is_error, "{:?}", result.output);
    assert_eq!(result.output["appended"], 0);
    assert_eq!(result.output["refined"], 7);

    let after =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    assert_eq!(after.len(), 7);
    for st in &after {
        let contract = st
            .contract
            .as_ref()
            .unwrap_or_else(|| panic!("order {} must gain a contract", st.order));
        assert!(
            !contract.expected_artifacts.is_empty(),
            "contract on order {} must not be empty",
            st.order,
        );
    }
}

#[test]
fn merge_not_invoked_leaves_skeleton_contracts_none() {
    let (repo, task_id) = make_repo_with_task();
    persist_skeleton(repo.as_ref(), &task_id, &["Only"]);

    let subtasks =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    assert_eq!(subtasks.len(), 1);
    assert!(
        subtasks[0].contract.is_none(),
        "skeleton subtask starts without contract",
    );
    assert_eq!(subtasks[0].mode, SubTaskMode::FullAccess);
}

#[test]
fn refine_subtask_round_trip_updates_named_fields_only() {
    use crate::core::agent::repositories::SubTaskRefinement;

    let (repo, task_id) = make_repo_with_task();
    let created = persist_skeleton(repo.as_ref(), &task_id, &["slot"]);
    let slot = &created[0];

    let contract = SubTaskContract {
        expected_artifacts: vec![ArtifactSpec {
            path_pattern: "docs/x.md".to_string(),
            kind: ArtifactKind::File,
        }],
    };
    let refinement = SubTaskRefinement {
        title: Some("refined title".to_string()),
        description: Some("refined description".to_string()),
        mode: Some(SubTaskMode::Write),
        contract: Some(contract.clone()),
    };
    <SqliteSessionRepository as TaskRepository>::refine_subtask(
        repo.as_ref(),
        &slot.id,
        refinement,
    )
    .unwrap();

    let after =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].title, "refined title");
    assert_eq!(after[0].description.as_deref(), Some("refined description"));
    assert_eq!(after[0].mode, SubTaskMode::Write);
    assert_eq!(after[0].contract.as_ref(), Some(&contract));

    let partial = SubTaskRefinement {
        description: Some("newer desc".to_string()),
        ..SubTaskRefinement::default()
    };
    <SqliteSessionRepository as TaskRepository>::refine_subtask(
        repo.as_ref(),
        &after[0].id,
        partial,
    )
    .unwrap();

    let after2 =
        <SqliteSessionRepository as TaskRepository>::get_subtasks(repo.as_ref(), &task_id)
            .unwrap();
    assert_eq!(after2[0].title, "refined title");
    assert_eq!(after2[0].description.as_deref(), Some("newer desc"));
    assert_eq!(after2[0].mode, SubTaskMode::Write);
    assert_eq!(after2[0].contract.as_ref(), Some(&contract));

    <SqliteSessionRepository as TaskRepository>::refine_subtask(
        repo.as_ref(),
        &after[0].id,
        SubTaskRefinement::default(),
    )
    .unwrap();
}
