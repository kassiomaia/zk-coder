//! Task and subtask operations for the agent store.

use chrono::Utc;
use log::warn;
use rusqlite::{params, Connection};

use super::SessionRepositoryError;

/// Create a new agent task.
///
/// Workstream H.2.a: the optional `framing` field is serialized to the
/// `framing` column as JSON. `None` maps to SQL `NULL` and round-trips
/// as `None` on read, preserving legacy (pre-H.2) behavior.
pub fn create_task(
    conn: &Connection,
    task: &crate::core::agent::task::AgentTask,
    title: Option<&str>,
    description: Option<&str>,
) -> Result<(), SessionRepositoryError> {
    let framing_json = match &task.framing {
        Some(framing) => Some(serde_json::to_string(framing)?),
        None => None,
    };
    conn.execute(
        "INSERT INTO agent_tasks (id, session_id, request, status, steps, created_at, completed_at, title, description, framing)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            task.id,
            task.session_id.0,
            serde_json::to_string(&task.request)?,
            serde_json::to_string(&task.status)?,
            serde_json::to_string(&task.steps)?,
            task.created_at.to_rfc3339(),
            task.completed_at.map(|t| t.to_rfc3339()),
            title,
            description,
            framing_json,
        ],
    )?;
    Ok(())
}

/// Update the `framing` column for a task (Workstream H.2.b).
///
/// `Some(framing)` overwrites the column with the serialized JSON;
/// `None` clears the column to SQL NULL so the system-prompt composer
/// falls back to the genre default on the next step.
pub fn update_task_framing(
    conn: &Connection,
    task_id: &str,
    framing: Option<&crate::core::agent::task::TaskFraming>,
) -> Result<(), SessionRepositoryError> {
    let framing_json = match framing {
        Some(framing) => Some(serde_json::to_string(framing)?),
        None => None,
    };
    conn.execute(
        "UPDATE agent_tasks SET framing = ?1 WHERE id = ?2",
        params![framing_json, task_id],
    )?;
    Ok(())
}

/// Update task status.
pub fn update_task_status(
    conn: &Connection,
    task_id: &str,
    status: &crate::core::agent::task::TaskStatus,
) -> Result<(), SessionRepositoryError> {
    let completed_at = if matches!(
        status,
        crate::core::agent::task::TaskStatus::Completed
            | crate::core::agent::task::TaskStatus::Failed
            | crate::core::agent::task::TaskStatus::Cancelled
    ) {
        Some(Utc::now().to_rfc3339())
    } else {
        None
    };

    conn.execute(
        "UPDATE agent_tasks SET status = ?1, completed_at = ?2 WHERE id = ?3",
        params![serde_json::to_string(status)?, completed_at, task_id],
    )?;
    Ok(())
}

/// Get all tasks for a session.
pub fn get_session_tasks(
    conn: &Connection,
    session_id: &crate::core::agent::session::SessionId,
) -> Result<Vec<crate::core::agent::task::AgentTask>, SessionRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT id, session_id, request, status, steps, created_at, completed_at, title, description, framing
         FROM agent_tasks
         WHERE session_id = ?1
         ORDER BY created_at DESC",
    )?;

    let tasks = stmt
        .query_map(params![session_id.0], |row| {
            let task_id: String = row.get(0)?;
            let request_str: String = row.get(2)?;
            let status_str: String = row.get(3)?;
            let steps_str: String = row.get(4)?;
            let created: String = row.get(5)?;
            let completed: Option<String> = row.get(6)?;
            let title: Option<String> = row.get(7)?;
            let description: Option<String> = row.get(8)?;
            let framing_json: Option<String> = row.get(9)?;
            // Workstream H.2.a: framing is an opaque JSON blob in SQL.
            // A row written before the column existed reads as NULL →
            // `None`. A row with malformed JSON also degrades to `None`
            // with a warning; we do not want a single bad row to poison
            // the whole session's task list.
            let framing = framing_json.and_then(|raw| {
                match serde_json::from_str::<crate::core::agent::task::TaskFraming>(&raw) {
                    Ok(parsed) => Some(parsed),
                    Err(e) => {
                        warn!("Failed to parse framing JSON for task {task_id}: {e}");
                        None
                    }
                }
            });

            let request: crate::core::agent::task::AgentRequest =
                serde_json::from_str(&request_str).map_err(|e| {
                    rusqlite::Error::ToSqlConversionFailure(
                        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                    )
                })?;
            let status: crate::core::agent::task::TaskStatus = serde_json::from_str(&status_str)
                .map_err(|e| {
                    rusqlite::Error::ToSqlConversionFailure(
                        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                    )
                })?;
            let steps: Vec<crate::core::agent::task::ExecutionStep> =
                serde_json::from_str(&steps_str).map_err(|e| {
                    rusqlite::Error::ToSqlConversionFailure(
                        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                    )
                })?;

            let created_at = chrono::DateTime::parse_from_rfc3339(&created)
                .inspect_err(|e| {
                    warn!("Failed to parse created_at timestamp for task {task_id}: {e}");
                })
                .unwrap_or_default()
                .with_timezone(&Utc);

            let completed_at = completed.and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .inspect_err(|e| {
                        warn!("Failed to parse completed_at timestamp for task {task_id}: {e}");
                    })
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            });

            Ok(crate::core::agent::task::AgentTask {
                id: task_id,
                session_id: crate::core::agent::session::SessionId(row.get(1)?),
                request,
                title,
                description,
                status,
                steps,
                subtasks: None, // Loaded separately via get_subtasks
                framing,
                created_at,
                completed_at,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(tasks)
}

/// Create a new subtask.
pub fn create_subtask(
    conn: &Connection,
    subtask: &crate::core::agent::task::SubTask,
) -> Result<(), SessionRepositoryError> {
    // Workstream B.1 — persist the contract JSON when present. `None`
    // maps to SQL NULL and round-trips as `None` on read, which the
    // runtime treats as "no scope-drift expectations".
    let contract_json = match &subtask.contract {
        Some(contract) => Some(serde_json::to_string(contract)?),
        None => None,
    };
    conn.execute(
        "INSERT INTO agent_subtasks (id, task_id, title, description, mode, status, order_index, created_at, started_at, completed_at, contract)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            subtask.id,
            subtask.task_id,
            subtask.title,
            subtask.description,
            serde_json::to_string(&subtask.mode)?,
            serde_json::to_string(&subtask.status)?,
            subtask.order,
            subtask.created_at.to_rfc3339(),
            subtask.started_at.map(|t| t.to_rfc3339()),
            subtask.completed_at.map(|t| t.to_rfc3339()),
            contract_json,
        ],
    )?;
    Ok(())
}

/// Update subtask status.
pub fn update_subtask_status(
    conn: &Connection,
    subtask_id: &str,
    status: &crate::core::agent::task::TaskStatus,
) -> Result<(), SessionRepositoryError> {
    // Get current status to check if we're transitioning to InProgress
    let current_status: Option<String> = conn
        .query_row(
            "SELECT status FROM agent_subtasks WHERE id = ?1",
            params![subtask_id],
            |row| row.get(0),
        )
        .inspect_err(|e| {
            warn!("Failed to query current status for subtask {subtask_id}: {e}");
        })
        .ok();

    // Parse current status to enum for robust comparison
    let is_transitioning_to_in_progress = current_status
        .as_deref()
        .and_then(|s| {
            serde_json::from_str::<crate::core::agent::task::TaskStatus>(s)
                .inspect_err(|e| {
                    warn!("Failed to parse current status JSON for subtask {subtask_id}: {e}");
                })
                .ok()
        })
        .is_some_and(|current| matches!(current, crate::core::agent::task::TaskStatus::Pending))
        && matches!(status, crate::core::agent::task::TaskStatus::InProgress);

    let completed_at = if matches!(
        status,
        crate::core::agent::task::TaskStatus::Completed
            | crate::core::agent::task::TaskStatus::Failed
            | crate::core::agent::task::TaskStatus::Cancelled
    ) {
        Some(Utc::now().to_rfc3339())
    } else {
        None
    };

    if is_transitioning_to_in_progress {
        conn.execute(
            "UPDATE agent_subtasks SET status = ?1, started_at = ?2 WHERE id = ?3",
            params![
                serde_json::to_string(status)?,
                Utc::now().to_rfc3339(),
                subtask_id
            ],
        )?;
    } else {
        conn.execute(
            "UPDATE agent_subtasks SET status = ?1, completed_at = ?2 WHERE id = ?3",
            params![serde_json::to_string(status)?, completed_at, subtask_id],
        )?;
    }
    Ok(())
}

/// Get all subtasks for a task.
pub fn get_subtasks(
    conn: &Connection,
    task_id: &str,
) -> Result<Vec<crate::core::agent::task::SubTask>, SessionRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT id, task_id, title, description, mode, status, order_index, created_at, started_at, completed_at, contract
         FROM agent_subtasks WHERE task_id = ?1 ORDER BY order_index",
    )?;

    let subtasks = stmt.query_map(params![task_id], |row| {
        let subtask_id: String = row.get(0)?;
        // Parse mode (default to FullAccess for backward compatibility)
        let mode_str: Option<String> = row.get(4)?;
        let mode = match mode_str.as_deref() {
            Some(s) => serde_json::from_str::<crate::core::agent::task::SubTaskMode>(s)
                .inspect_err(|e| {
                    warn!("Failed to parse mode for subtask {subtask_id} in task {task_id}: {e}");
                })
                .unwrap_or(crate::core::agent::task::SubTaskMode::FullAccess),
            None => crate::core::agent::task::SubTaskMode::FullAccess,
        };

        // Parse status
        let status_str: String = row.get(5)?;
        let status: crate::core::agent::task::TaskStatus = serde_json::from_str(&status_str).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        })?;

        // Parse created_at
        let created_at = chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
            .inspect_err(|e| {
                warn!("Failed to parse created_at timestamp for subtask {subtask_id} in task {task_id}: {e}");
            })
            .unwrap_or_default()
            .with_timezone(&Utc);

        // Parse started_at (optional)
        let started_at = row
            .get::<_, Option<String>>(8)?
            .map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .inspect_err(|e| {
                        warn!("Failed to parse started_at timestamp for subtask {subtask_id} in task {task_id}: {e}");
                    })
                    .unwrap_or_default()
                    .with_timezone(&Utc)
            });

        // Parse completed_at (optional)
        let completed_at = row
            .get::<_, Option<String>>(9)?
            .map(|s| {
                chrono::DateTime::parse_from_rfc3339(&s)
                    .inspect_err(|e| {
                        warn!("Failed to parse completed_at timestamp for subtask {subtask_id} in task {task_id}: {e}");
                    })
                    .unwrap_or_default()
                    .with_timezone(&Utc)
            });

        // Workstream B.1 — contract is optional; absent rows map to
        // `None`, preserving legacy behavior. Malformed JSON is logged
        // and falls back to `None` rather than failing the entire
        // subtask load.
        let contract_str: Option<String> = row.get(10)?;
        let contract = contract_str.and_then(|s| {
            match serde_json::from_str::<crate::core::agent::task::SubTaskContract>(&s) {
                Ok(c) => Some(c),
                Err(e) => {
                    warn!(
                        "Failed to parse contract JSON for subtask {subtask_id} in task {task_id}: {e}"
                    );
                    None
                }
            }
        });

        Ok(crate::core::agent::task::SubTask {
            id: subtask_id,
            task_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            mode,
            status,
            order: row.get(6)?,
            contract,
            created_at,
            started_at,
            completed_at,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(subtasks)
}

/// Delete all subtasks for a task.
pub fn delete_subtasks(conn: &Connection, task_id: &str) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "DELETE FROM agent_subtasks WHERE task_id = ?1",
        params![task_id],
    )?;
    Ok(())
}

/// Create a new execution step record.
pub fn create_execution_step(
    conn: &Connection,
    step: &crate::core::agent::task::ExecutionStep,
    session_id: &str,
    task_id: Option<&str>,
    subtask_id: Option<&str>,
) -> Result<(), SessionRepositoryError> {
    use uuid::Uuid;

    let action_type = match &step.action {
        crate::core::agent::task::StepAction::LlmRequest { .. } => "llm_request",
        crate::core::agent::task::StepAction::ToolCall { .. } => "tool_call",
    };

    let action_data = serde_json::to_string(&step.action)?;
    let result_data = step
        .result
        .as_ref()
        .map(serde_json::to_string)
        .transpose()?;

    // Extract token statistics and duration
    let (input_tokens, output_tokens, total_tokens, duration_ms) =
        if let Some(result) = &step.result {
            (
                result.input_tokens.map(i64::from),
                result.output_tokens.map(i64::from),
                result.tokens_used.map(i64::from),
                Some(result.duration_ms as i64),
            )
        } else {
            (None, None, None, None)
        };

    conn.execute(
        "INSERT INTO agent_execution_steps (id, session_id, task_id, subtask_id, step_number, action_type, action_data, result_data, input_tokens, output_tokens, total_tokens, duration_ms, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            Uuid::new_v4().to_string(),
            session_id,
            task_id,
            subtask_id,
            step.step_number,
            action_type,
            action_data,
            result_data,
            input_tokens,
            output_tokens,
            total_tokens,
            duration_ms,
            Utc::now().to_rfc3339(),
        ],
    )?;

    Ok(())
}

/// Get recent execution steps for a session.
pub fn get_recent_execution_steps(
    conn: &Connection,
    session_id: &str,
    limit: usize,
) -> Result<Vec<crate::core::agent::task::ExecutionStep>, SessionRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT action_data, result_data, step_number
         FROM agent_execution_steps
         WHERE session_id = ?1
         ORDER BY created_at DESC
         LIMIT ?2",
    )?;

    let steps = stmt
        .query_map(params![session_id, limit], |row| {
            let action_data: String = row.get(0)?;
            let result_data: Option<String> = row.get(1)?;
            let step_number: u32 = row.get(2)?;

            let action: crate::core::agent::task::StepAction = serde_json::from_str(&action_data)
                .map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(
                    Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                )
            })?;

            let result = match result_data {
                Some(data) => Some(serde_json::from_str(&data).map_err(|e| {
                    rusqlite::Error::ToSqlConversionFailure(
                        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                    )
                })?),
                None => None,
            };

            Ok(crate::core::agent::task::ExecutionStep {
                step_number,
                action,
                result,
                timestamp: Utc::now(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(steps)
}

/// Get all execution steps for a session in chronological order.
pub fn get_session_execution_steps(
    conn: &Connection,
    session_id: &crate::core::agent::session::SessionId,
) -> Result<Vec<crate::core::agent::task::ExecutionStep>, SessionRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT action_data, result_data, step_number, input_tokens, output_tokens, total_tokens, duration_ms
         FROM agent_execution_steps
         WHERE session_id = ?1
         ORDER BY created_at ASC",
    )?;

    let steps = stmt
        .query_map(params![session_id.0], |row| {
            let action_data: String = row.get(0)?;
            let result_data: Option<String> = row.get(1)?;
            let step_number: u32 = row.get(2)?;
            let input_tokens: Option<i64> = row.get(3)?;
            let output_tokens: Option<i64> = row.get(4)?;
            let total_tokens: Option<i64> = row.get(5)?;
            let _duration_ms: Option<i64> = row.get(6)?;

            let action: crate::core::agent::task::StepAction = serde_json::from_str(&action_data)
                .map_err(|e| {
                rusqlite::Error::ToSqlConversionFailure(
                    Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                )
            })?;

            let result = match result_data {
                Some(data) => Some(serde_json::from_str(&data).map_err(|e| {
                    rusqlite::Error::ToSqlConversionFailure(
                        Box::new(e) as Box<dyn std::error::Error + Send + Sync>
                    )
                })?),
                None => None,
            };

            // Ensure StepResult has token information if available from the database
            let result = result.map(|mut r: crate::core::agent::task::StepResult| {
                if r.input_tokens.is_none() {
                    r.input_tokens = input_tokens.map(|t| t as u32);
                }
                if r.output_tokens.is_none() {
                    r.output_tokens = output_tokens.map(|t| t as u32);
                }
                if r.tokens_used.is_none() {
                    r.tokens_used = total_tokens.map(|t| t as u32);
                }
                r
            });

            Ok(crate::core::agent::task::ExecutionStep {
                step_number,
                action,
                result,
                timestamp: Utc::now(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(steps)
}

/// Clear all execution steps for a session.
pub fn clear_session_execution_steps(
    conn: &Connection,
    session_id: &crate::core::agent::session::SessionId,
) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "DELETE FROM agent_execution_steps WHERE session_id = ?1",
        params![session_id.0],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::session::{AgentSession, SessionId};
    use crate::core::agent::task::{AgentRequest, AgentTask, SubTask, SubTaskMode, TaskStatus};
    use crate::core::agent::types::AgentId;

    fn make_test_session(conn: &Connection) -> SessionId {
        let session = AgentSession::new(
            AgentId::new("test-agent"),
            "Test Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
            None,
        );
        super::super::sessions::create_session(conn, &session).unwrap();
        session.id
    }

    fn make_test_task(conn: &Connection, session_id: &SessionId) -> String {
        let task = AgentTask::new(
            session_id.clone(),
            crate::core::agent::task::AgentRequest {
                content: "Test request".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        let task_id = task.id.clone();
        create_task(conn, &task, None, None).unwrap();
        task_id
    }

    #[test]
    fn test_create_and_get_tasks() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        make_test_task(&conn, &session_id);

        let tasks = get_session_tasks(&conn, &session_id).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].session_id.0, session_id.0);
    }

    /// Round-trip regression test (Workstream A.3).
    ///
    /// Previously `get_session_tasks` omitted the `title` and `description`
    /// columns from its SELECT and hardcoded `title: None` on load, so the
    /// LLM-generated title authored by the planning phase was dropped after
    /// the first reload. The frontend then fell back to truncating the raw
    /// request content at 80 characters, producing the "double-render" bug
    /// reported on session `a597debb-c5b7-49eb-913e-7b11c7e4ac84` (2026-04-18).
    ///
    /// This test creates a task with both `title` and `description`, reloads
    /// it via `get_session_tasks`, and asserts the columns survived the
    /// round-trip. Would have caught the original regression.
    #[test]
    fn test_task_title_and_description_survive_round_trip() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "long user request that should not be used as the display title"
                    .to_string(),
                attachments: vec![],
                context: None,
            },
        );
        let expected_title = "Create release communication files and copy";
        let expected_description =
            "Generate launch communication assets (email, social, blog) for the release.";
        create_task(&conn, &task, Some(expected_title), Some(expected_description)).unwrap();

        let tasks = get_session_tasks(&conn, &session_id).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].title.as_deref(), Some(expected_title));
        assert_eq!(tasks[0].description.as_deref(), Some(expected_description));
    }

    /// A task persisted without title/description must load as `None` for
    /// both fields. Protects against a future regression that would default
    /// to empty strings or to the request content.
    #[test]
    fn test_task_without_title_and_description_loads_as_none() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        make_test_task(&conn, &session_id);

        let tasks = get_session_tasks(&conn, &session_id).unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].title.is_none());
        assert!(tasks[0].description.is_none());
    }

    #[test]
    fn test_update_task_status() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "Test request".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        create_task(&conn, &task, None, None).unwrap();

        update_task_status(&conn, &task.id, &TaskStatus::Completed).unwrap();

        let tasks = get_session_tasks(&conn, &session_id).unwrap();
        assert!(matches!(tasks[0].status, TaskStatus::Completed));
        assert!(tasks[0].completed_at.is_some());
    }

    /// Workstream H.2.a — DB round-trip for `TaskFraming`.
    ///
    /// A task persisted with a `framing` record must reload with every
    /// field preserved: `role`, `voice_constraints`, `domain_priors`,
    /// and `forbidden_patterns`. Pre-H.2 rows (where the `framing`
    /// column is NULL) must still load as `framing: None`.
    #[test]
    fn test_task_framing_survives_round_trip() {
        use crate::core::agent::task::{DomainPriors, TaskFraming};

        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let mut task = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "write a launch announcement".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        let framing = TaskFraming {
            role: "senior communication professional".to_string(),
            voice_constraints: vec![
                "one claim per paragraph".to_string(),
                "no filler openers".to_string(),
            ],
            domain_priors: Some(DomainPriors {
                hints: vec!["press-release voice".to_string()],
            }),
            forbidden_patterns: vec![r"(?i)game[- ]changing".to_string()],
        };
        task.framing = Some(framing.clone());
        create_task(&conn, &task, Some("Launch announcement"), None).unwrap();

        let tasks = get_session_tasks(&conn, &session_id).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].framing, Some(framing));
    }

    #[test]
    fn test_task_without_framing_round_trips_as_none() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        make_test_task(&conn, &session_id);

        let tasks = get_session_tasks(&conn, &session_id).unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(
            tasks[0].framing.is_none(),
            "legacy tasks without framing must load as None"
        );
    }

    #[test]
    fn test_create_and_get_subtasks() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task_id = make_test_task(&conn, &session_id);

        let subtask = SubTask::new(
            task_id.clone(),
            "Test Subtask".to_string(),
            SubTaskMode::Read,
            0,
        );
        create_subtask(&conn, &subtask).unwrap();

        let subtasks = get_subtasks(&conn, &task_id).unwrap();
        assert_eq!(subtasks.len(), 1);
        assert_eq!(subtasks[0].title, "Test Subtask");
    }

    /// Workstream B.1 — DB round-trip for `SubTaskContract`.
    ///
    /// A subtask persisted with an `expected_artifacts` contract must
    /// reload with the same contract, preserving every field on each
    /// `ArtifactSpec` (both `path_pattern` and `kind`). This is the
    /// characterization test that pins the column semantics before
    /// runtime enforcement (B.3) reads them.
    #[test]
    fn test_subtask_contract_survives_db_round_trip() {
        use crate::core::agent::task::{ArtifactKind, ArtifactSpec, SubTaskContract};

        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task_id = make_test_task(&conn, &session_id);

        let contract = SubTaskContract {
            expected_artifacts: vec![
                ArtifactSpec {
                    path_pattern: "docs/communications/social/*.md".to_string(),
                    kind: ArtifactKind::File,
                },
                ArtifactSpec {
                    path_pattern: "reports/launch/**/*.pdf".to_string(),
                    kind: ArtifactKind::Document,
                },
            ],
        };

        let subtask = SubTask::new(
            task_id.clone(),
            "publish launch copy".to_string(),
            SubTaskMode::Write,
            0,
        )
        .with_contract(contract.clone());
        create_subtask(&conn, &subtask).unwrap();

        let fetched = get_subtasks(&conn, &task_id).unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(
            fetched[0].contract.as_ref(),
            Some(&contract),
            "contract must round-trip through the DB unchanged"
        );
    }

    /// A subtask persisted without a contract (the legacy path) must
    /// reload with `contract == None`. Guards against an accidental
    /// default that would turn every legacy subtask into a scope-drift
    /// trigger.
    #[test]
    fn test_subtask_without_contract_loads_as_none() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task_id = make_test_task(&conn, &session_id);

        let subtask = SubTask::new(
            task_id.clone(),
            "no contract".to_string(),
            SubTaskMode::Read,
            0,
        );
        create_subtask(&conn, &subtask).unwrap();

        let fetched = get_subtasks(&conn, &task_id).unwrap();
        assert_eq!(fetched.len(), 1);
        assert!(
            fetched[0].contract.is_none(),
            "absent contract must remain absent after reload"
        );
    }

    #[test]
    fn test_update_subtask_status() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task_id = make_test_task(&conn, &session_id);

        let subtask = SubTask::new(
            task_id.clone(),
            "Test Subtask".to_string(),
            SubTaskMode::Read,
            0,
        );
        create_subtask(&conn, &subtask).unwrap();

        update_subtask_status(&conn, &subtask.id, &TaskStatus::Completed).unwrap();

        let subtasks = get_subtasks(&conn, &task_id).unwrap();
        assert!(matches!(subtasks[0].status, TaskStatus::Completed));
        assert!(subtasks[0].completed_at.is_some());
    }

    #[test]
    fn test_delete_subtasks() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task_id = make_test_task(&conn, &session_id);

        let subtask = SubTask::new(
            task_id.clone(),
            "Test Subtask".to_string(),
            SubTaskMode::Read,
            0,
        );
        create_subtask(&conn, &subtask).unwrap();

        delete_subtasks(&conn, &task_id).unwrap();

        let subtasks = get_subtasks(&conn, &task_id).unwrap();
        assert_eq!(subtasks.len(), 0);
    }

    #[test]
    fn test_get_subtasks_ordered() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task_id = make_test_task(&conn, &session_id);

        let subtask1 = SubTask::new(task_id.clone(), "First".to_string(), SubTaskMode::Read, 1);
        let subtask2 = SubTask::new(task_id.clone(), "Second".to_string(), SubTaskMode::Write, 0);
        create_subtask(&conn, &subtask1).unwrap();
        create_subtask(&conn, &subtask2).unwrap();

        let fetched = get_subtasks(&conn, &task_id).unwrap();
        assert_eq!(fetched.len(), 2);
        assert_eq!(fetched[0].order, 0);
        assert_eq!(fetched[0].title, "Second");
        assert_eq!(fetched[1].order, 1);
        assert_eq!(fetched[1].title, "First");
    }

    #[test]
    fn test_get_subtasks_for_different_tasks() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task1_id = make_test_task(&conn, &session_id);
        let task2_id = make_test_task(&conn, &session_id);

        let subtask1 = SubTask::new(
            task1_id.clone(),
            "Task 1 Subtask".to_string(),
            SubTaskMode::Read,
            0,
        );
        let subtask2 = SubTask::new(
            task2_id.clone(),
            "Task 2 Subtask".to_string(),
            SubTaskMode::Write,
            0,
        );
        create_subtask(&conn, &subtask1).unwrap();
        create_subtask(&conn, &subtask2).unwrap();

        let task1_subtasks = get_subtasks(&conn, &task1_id).unwrap();
        let task2_subtasks = get_subtasks(&conn, &task2_id).unwrap();

        assert_eq!(task1_subtasks.len(), 1);
        assert_eq!(task2_subtasks.len(), 1);
        assert_eq!(task1_subtasks[0].task_id, task1_id);
        assert_eq!(task2_subtasks[0].task_id, task2_id);
    }

    // ========================================================================
    // Incomplete Task Detection Tests
    // ========================================================================

    #[test]
    fn test_find_incomplete_task_returns_pending_root_task() {
        // Test that Pending root tasks are found (tasks that haven't been completed)
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "Test request".to_string(),
                attachments: vec![],
                context: None,
            },
        );

        create_task(&conn, &task, None, None).unwrap();

        // Create subtasks - some completed, some pending
        let subtask1 = SubTask::new(task.id.clone(), "Step 1".to_string(), SubTaskMode::Read, 0);
        let subtask2 = SubTask::new(task.id.clone(), "Step 2".to_string(), SubTaskMode::Read, 1);
        create_subtask(&conn, &subtask1).unwrap();
        create_subtask(&conn, &subtask2).unwrap();

        // Mark first subtask as completed, second as pending
        update_subtask_status(&conn, &subtask1.id, &TaskStatus::Completed).unwrap();

        // Query for incomplete tasks
        let tasks = get_session_tasks(&conn, &session_id).unwrap();
        let incomplete: Vec<_> = tasks
            .iter()
            .filter(|t| !matches!(t.status, TaskStatus::Completed | TaskStatus::Cancelled))
            .collect();

        // Root task should still be Pending because it hasn't been completed
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].id, task.id);
    }

    #[test]
    fn test_find_incomplete_task_excludes_completed_tasks() {
        // Test that Completed tasks are not returned
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "Test request".to_string(),
                attachments: vec![],
                context: None,
            },
        );

        create_task(&conn, &task, None, None).unwrap();
        update_task_status(&conn, &task.id, &TaskStatus::Completed).unwrap();

        // Query for incomplete tasks
        let tasks = get_session_tasks(&conn, &session_id).unwrap();
        let incomplete: Vec<_> = tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::InProgress | TaskStatus::Failed))
            .collect();

        // Should be empty - task is completed
        assert!(incomplete.is_empty());
    }

    #[test]
    fn test_find_incomplete_task_with_failed_status() {
        // Test that Failed tasks are also considered incomplete
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "Test request".to_string(),
                attachments: vec![],
                context: None,
            },
        );

        create_task(&conn, &task, None, None).unwrap();
        update_task_status(&conn, &task.id, &TaskStatus::Failed).unwrap();

        // Query for incomplete tasks
        let tasks = get_session_tasks(&conn, &session_id).unwrap();
        let incomplete: Vec<_> = tasks
            .iter()
            .filter(|t| matches!(t.status, TaskStatus::InProgress | TaskStatus::Failed))
            .collect();

        // Failed tasks should be considered incomplete
        assert_eq!(incomplete.len(), 1);
        assert!(matches!(incomplete[0].status, TaskStatus::Failed));
    }

    #[test]
    fn test_multiple_tasks_finds_most_recent_incomplete() {
        // Test that when there are multiple tasks, we can identify incomplete ones
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);

        // Create first task - completed
        let task1 = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "First task".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        create_task(&conn, &task1, None, None).unwrap();
        update_task_status(&conn, &task1.id, &TaskStatus::Completed).unwrap();

        // Create second task - pending (incomplete)
        let task2 = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "Second task".to_string(),
                attachments: vec![],
                context: None,
            },
        );
        create_task(&conn, &task2, None, None).unwrap();

        // Query for incomplete tasks
        let tasks = get_session_tasks(&conn, &session_id).unwrap();
        let incomplete: Vec<_> = tasks
            .iter()
            .filter(|t| !matches!(t.status, TaskStatus::Completed | TaskStatus::Cancelled))
            .collect();

        // Only task2 should be incomplete
        assert_eq!(incomplete.len(), 1);
        assert_eq!(incomplete[0].id, task2.id);
    }

    #[test]
    fn test_subtask_completion_affects_task_completion() {
        // Test that a task with pending subtasks is considered incomplete
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        let task = AgentTask::new(
            session_id.clone(),
            AgentRequest {
                content: "Multi-step task".to_string(),
                attachments: vec![],
                context: None,
            },
        );

        create_task(&conn, &task, None, None).unwrap();

        // Create multiple subtasks
        let subtask1 = SubTask::new(task.id.clone(), "Step 1".to_string(), SubTaskMode::Read, 0);
        let subtask2 = SubTask::new(task.id.clone(), "Step 2".to_string(), SubTaskMode::Read, 1);
        let subtask3 = SubTask::new(task.id.clone(), "Step 3".to_string(), SubTaskMode::Read, 2);
        create_subtask(&conn, &subtask1).unwrap();
        create_subtask(&conn, &subtask2).unwrap();
        create_subtask(&conn, &subtask3).unwrap();

        // Complete first subtask only
        update_subtask_status(&conn, &subtask1.id, &TaskStatus::Completed).unwrap();

        // Check subtask status
        let subtasks = get_subtasks(&conn, &task.id).unwrap();
        let completed = subtasks
            .iter()
            .filter(|st| matches!(st.status, TaskStatus::Completed))
            .count();
        let pending = subtasks
            .iter()
            .filter(|st| matches!(st.status, TaskStatus::Pending | TaskStatus::InProgress))
            .count();

        assert_eq!(completed, 1);
        assert_eq!(pending, 2);

        // Task should still be incomplete (has pending subtasks)
        let tasks = get_session_tasks(&conn, &session_id).unwrap();
        let root_task = &tasks[0];
        assert!(matches!(
            root_task.status,
            TaskStatus::Pending | TaskStatus::InProgress
        ));
    }
}
