//! SQLite storage for agent sessions and tasks.
//!
//! Persists agent sessions across app restarts so users can
//! resume incomplete tasks.

use std::sync::Mutex;

use chrono::Utc;
use uuid::Uuid;
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

use super::session::{AgentSession, SessionId, SessionState};
use super::types::AgentId;

/// Errors from the session store.
#[derive(Debug, Error)]
pub enum SessionStoreError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Session not found: {0}")]
    NotFound(String),
    #[error("Lock error: {0}")]
    LockError(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// SQLite-based session store.
pub struct SessionStore {
    pub(crate) conn: Mutex<Connection>,
}

impl SessionStore {
    /// Create a new session store at the given database path.
    pub fn new(db_path: &str) -> Result<Self, SessionStoreError> {
        let conn = Connection::open(db_path)?;
        Self::init_schema(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Initialize the database schema.
    fn init_schema(conn: &Connection) -> Result<(), SessionStoreError> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS agent_sessions (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                agent_name TEXT NOT NULL,
                conversation_id TEXT NOT NULL,
                thread_id TEXT NOT NULL,
                state TEXT NOT NULL,
                workspace_root TEXT,
                parent_session_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS agent_tasks (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                request TEXT NOT NULL,
                status TEXT NOT NULL,
                steps TEXT NOT NULL DEFAULT '[]',
                created_at TEXT NOT NULL,
                completed_at TEXT,
                FOREIGN KEY (session_id) REFERENCES agent_sessions(id)
            );

            CREATE TABLE IF NOT EXISTS agent_subtasks (
                id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                mode TEXT NOT NULL DEFAULT 'full_access',
                status TEXT NOT NULL,
                order_index INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT,
                FOREIGN KEY (task_id) REFERENCES agent_tasks(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_sessions_conversation
                ON agent_sessions(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_thread
                ON agent_sessions(thread_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_session
                ON agent_tasks(session_id);

            CREATE TABLE IF NOT EXISTS agent_files_touched (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                action TEXT NOT NULL,
                diff TEXT,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS agent_terminal_output (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                line_type TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS agent_status_phrases (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                phrase TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS agent_tool_calls (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                arguments TEXT NOT NULL,
                result TEXT,
                is_error INTEGER NOT NULL DEFAULT 0,
                duration_ms INTEGER,
                timestamp TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_files_session
                ON agent_files_touched(session_id);
            CREATE INDEX IF NOT EXISTS idx_terminal_session
                ON agent_terminal_output(session_id);
            CREATE INDEX IF NOT EXISTS idx_phrases_session
                ON agent_status_phrases(session_id);
            CREATE INDEX IF NOT EXISTS idx_tool_calls_session
                ON agent_tool_calls(session_id);

            CREATE TABLE IF NOT EXISTS agent_execution_steps (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                task_id TEXT,
                subtask_id TEXT,
                step_number INTEGER NOT NULL,
                action_type TEXT NOT NULL,
                action_data TEXT NOT NULL,
                result_data TEXT,
                input_tokens INTEGER,
                output_tokens INTEGER,
                total_tokens INTEGER,
                duration_ms INTEGER,
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE,
                FOREIGN KEY (task_id) REFERENCES agent_tasks(id) ON DELETE CASCADE,
                FOREIGN KEY (subtask_id) REFERENCES agent_subtasks(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_execution_steps_session_created
                ON agent_execution_steps(session_id, created_at DESC);
            CREATE INDEX IF NOT EXISTS idx_execution_steps_task
                ON agent_execution_steps(task_id);
            CREATE INDEX IF NOT EXISTS idx_execution_steps_subtask
                ON agent_execution_steps(subtask_id);
            ",
        )?;

        // Run migrations for schema updates
        Self::run_migrations(conn)?;

        Ok(())
    }

    /// Run database migrations for schema updates.
    fn run_migrations(conn: &Connection) -> Result<(), SessionStoreError> {
        // Migration 1: Add mode column to agent_subtasks
        let _ = conn.execute(
            "ALTER TABLE agent_subtasks ADD COLUMN mode TEXT NOT NULL DEFAULT 'full_access'",
            [],
        ); // Ignore error if column exists

        // Migration 2: Add started_at column to agent_subtasks
        let _ = conn.execute(
            "ALTER TABLE agent_subtasks ADD COLUMN started_at TEXT",
            [],
        ); // Ignore error if column exists

        // Migration 3: Add subtask_id column to agent_tool_calls
        let _ = conn.execute(
            "ALTER TABLE agent_tool_calls ADD COLUMN subtask_id TEXT REFERENCES agent_subtasks(id)",
            [],
        ); // Ignore error if column exists

        Ok(())
    }

    /// Save a new session.
    pub fn create_session(&self, session: &AgentSession) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        conn.execute(
            "INSERT INTO agent_sessions (id, agent_id, agent_name, conversation_id, thread_id, state, workspace_root, parent_session_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                session.id.0,
                session.agent_id.0,
                session.agent_name,
                session.conversation_id,
                session.thread_id,
                serde_json::to_string(&session.state)?,
                session.workspace_root.as_ref().map(|p| p.to_string_lossy().to_string()),
                session.parent_session_id.as_ref().map(|id| id.0.clone()),
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Load a session by ID.
    pub fn get_session(&self, id: &SessionId) -> Result<Option<AgentSession>, SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let result = conn
            .query_row(
                "SELECT id, agent_id, agent_name, conversation_id, thread_id, state, workspace_root, parent_session_id, created_at, updated_at
                 FROM agent_sessions WHERE id = ?1",
                params![id.0],
                |row| {
                    let state_str: String = row.get(5)?;
                    let workspace: Option<String> = row.get(6)?;
                    let parent: Option<String> = row.get(7)?;
                    let created: String = row.get(8)?;
                    let updated: String = row.get(9)?;

                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        state_str,
                        workspace,
                        parent,
                        created,
                        updated,
                    ))
                },
            )
            .optional()?;

        match result {
            Some((id, agent_id, agent_name, conv_id, thread_id, state_str, workspace, parent, created, updated)) => {
                let state: SessionState = serde_json::from_str(&state_str)?;
                Ok(Some(AgentSession {
                    id: SessionId(id),
                    agent_id: AgentId(agent_id),
                    agent_name,
                    conversation_id: conv_id,
                    thread_id,
                    state,
                    workspace_root: workspace.map(std::path::PathBuf::from),
                    parent_session_id: parent.map(SessionId),
                    created_at: chrono::DateTime::parse_from_rfc3339(&created)
                        .unwrap_or_default()
                        .with_timezone(&Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated)
                        .unwrap_or_default()
                        .with_timezone(&Utc),
                }))
            }
            None => Ok(None),
        }
    }

    /// Update session state.
    pub fn update_state(
        &self,
        id: &SessionId,
        state: &SessionState,
    ) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let rows = conn.execute(
            "UPDATE agent_sessions SET state = ?1, updated_at = ?2 WHERE id = ?3",
            params![
                serde_json::to_string(state)?,
                Utc::now().to_rfc3339(),
                id.0,
            ],
        )?;

        if rows == 0 {
            return Err(SessionStoreError::NotFound(id.0.clone()));
        }
        Ok(())
    }

    /// Get sessions for a conversation.
    pub fn get_sessions_for_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<AgentSession>, SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, agent_id, agent_name, conversation_id, thread_id, state, workspace_root, parent_session_id, created_at, updated_at
             FROM agent_sessions WHERE conversation_id = ?1 ORDER BY created_at DESC",
        )?;

        let sessions = stmt
            .query_map(params![conversation_id], |row| {
                let state_str: String = row.get(5)?;
                let workspace: Option<String> = row.get(6)?;
                let parent: Option<String> = row.get(7)?;
                let created: String = row.get(8)?;
                let updated: String = row.get(9)?;

                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    state_str,
                    workspace,
                    parent,
                    created,
                    updated,
                ))
            })?
            .filter_map(|r| r.ok())
            .filter_map(|(id, agent_id, agent_name, conv_id, thread_id, state_str, workspace, parent, created, updated)| {
                let state: SessionState = serde_json::from_str(&state_str).ok()?;
                Some(AgentSession {
                    id: SessionId(id),
                    agent_id: AgentId(agent_id),
                    agent_name,
                    conversation_id: conv_id,
                    thread_id,
                    state,
                    workspace_root: workspace.map(std::path::PathBuf::from),
                    parent_session_id: parent.map(SessionId),
                    created_at: chrono::DateTime::parse_from_rfc3339(&created)
                        .unwrap_or_default()
                        .with_timezone(&Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated)
                        .unwrap_or_default()
                        .with_timezone(&Utc),
                })
            })
            .collect();

        Ok(sessions)
    }

    /// Delete a session.
    pub fn delete_session(&self, id: &SessionId) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        // Delete tasks first (foreign key)
        conn.execute("DELETE FROM agent_tasks WHERE session_id = ?1", params![id.0])?;
        conn.execute("DELETE FROM agent_sessions WHERE id = ?1", params![id.0])?;
        Ok(())
    }

    /// Update the workspace root for a session.
    pub fn update_workspace(&self, id: &SessionId, workspace_root: &str) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        conn.execute(
            "UPDATE agent_sessions SET workspace_root = ?1, updated_at = ?2 WHERE id = ?3",
            params![workspace_root, Utc::now().to_rfc3339(), id.0],
        )?;
        Ok(())
    }

    // ========================================================================
    // Task methods
    // ========================================================================

    /// Create a new agent task.
    pub fn create_task(
        &self,
        task: &crate::agents::task::AgentTask,
    ) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        conn.execute(
            "INSERT INTO agent_tasks (id, session_id, request, status, steps, created_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                task.id,
                task.session_id.0,
                serde_json::to_string(&task.request)?,
                serde_json::to_string(&task.status)?,
                serde_json::to_string(&task.steps)?,
                task.created_at.to_rfc3339(),
                task.completed_at.map(|t| t.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    /// Update task status.
    pub fn update_task_status(
        &self,
        task_id: &str,
        status: &crate::agents::task::TaskStatus,
    ) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let completed_at = if matches!(
            status,
            crate::agents::task::TaskStatus::Completed
                | crate::agents::task::TaskStatus::Failed
                | crate::agents::task::TaskStatus::Cancelled
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

    // ========================================================================
    // Execution Step methods
    // ========================================================================

    /// Create a new execution step record.
    pub fn create_execution_step(
        &self,
        step: &crate::agents::task::ExecutionStep,
        session_id: &str,
        task_id: Option<&str>,
        subtask_id: Option<&str>,
    ) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let action_type = match &step.action {
            crate::agents::task::StepAction::LlmRequest { .. } => "llm_request",
            crate::agents::task::StepAction::ToolCall { .. } => "tool_call",
        };

        let action_data = serde_json::to_string(&step.action)?;
        let result_data = step.result.as_ref().map(serde_json::to_string).transpose()?;

        // Extract token statistics and duration
        let (input_tokens, output_tokens, total_tokens, duration_ms) = if let Some(result) = &step.result {
            (
                result.input_tokens.map(|t| t as i64),
                result.output_tokens.map(|t| t as i64),
                result.tokens_used.map(|t| t as i64),
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
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::agents::task::ExecutionStep>, SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

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

                let action: crate::agents::task::StepAction = match serde_json::from_str(&action_data) {
                    Ok(a) => a,
                    Err(e) => {
                        return Err(rusqlite::Error::ToSqlConversionFailure(
                            Box::new(e) as Box<dyn std::error::Error + Send + Sync>,
                        ));
                    }
                };

                let result = match result_data {
                    Some(data) => {
                        Some(serde_json::from_str(&data).map_err(|e| {
                            rusqlite::Error::ToSqlConversionFailure(
                                Box::new(e) as Box<dyn std::error::Error + Send + Sync>,
                            )
                        })?)
                    }
                    None => None,
                };

                Ok(crate::agents::task::ExecutionStep {
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
    ///
    /// This fetches execution steps from the `agent_execution_steps` table,
    /// which is the correct source for execution steps persisted during runtime.
    pub fn get_session_execution_steps(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<crate::agents::task::ExecutionStep>, SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

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

                let action: crate::agents::task::StepAction = match serde_json::from_str(&action_data) {
                    Ok(a) => a,
                    Err(e) => {
                        return Err(rusqlite::Error::ToSqlConversionFailure(
                            Box::new(e) as Box<dyn std::error::Error + Send + Sync>,
                        ));
                    }
                };

                let result = match result_data {
                    Some(data) => {
                        Some(serde_json::from_str(&data).map_err(|e| {
                            rusqlite::Error::ToSqlConversionFailure(
                                Box::new(e) as Box<dyn std::error::Error + Send + Sync>,
                            )
                        })?)
                    }
                    None => None,
                };

                Ok(crate::agents::task::ExecutionStep {
                    step_number,
                    action,
                    result,
                    timestamp: Utc::now(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(steps)
    }

    // ========================================================================
    // Subtask methods
    // ========================================================================

    /// Create a new subtask.
    pub fn create_subtask(
        &self,
        subtask: &crate::agents::task::SubTask,
    ) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        conn.execute(
            "INSERT INTO agent_subtasks (id, task_id, title, description, mode, status, order_index, created_at, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
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
            ],
        )?;
        Ok(())
    }

    /// Update subtask status.
    pub fn update_subtask_status(
        &self,
        subtask_id: &str,
        status: &crate::agents::task::TaskStatus,
    ) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        // Get current status to check if we're transitioning to InProgress
        let current_status: Option<String> = conn
            .query_row("SELECT status FROM agent_subtasks WHERE id = ?1", params![subtask_id], |row| row.get(0))
            .ok();

        // Parse current status to enum for robust comparison (not brittle to serialization changes)
        let is_transitioning_to_in_progress = current_status
            .as_deref()
            .and_then(|s| serde_json::from_str::<crate::agents::task::TaskStatus>(s).ok())
            .map(|current| matches!(current, crate::agents::task::TaskStatus::Pending))
            .unwrap_or(false)
            && matches!(status, crate::agents::task::TaskStatus::InProgress);

        let completed_at = if matches!(
            status,
            crate::agents::task::TaskStatus::Completed
                | crate::agents::task::TaskStatus::Failed
                | crate::agents::task::TaskStatus::Cancelled
        ) {
            Some(Utc::now().to_rfc3339())
        } else {
            None
        };

        if is_transitioning_to_in_progress {
            conn.execute(
                "UPDATE agent_subtasks SET status = ?1, started_at = ?2 WHERE id = ?3",
                params![serde_json::to_string(status)?, Utc::now().to_rfc3339(), subtask_id],
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
    pub fn get_subtasks(&self, task_id: &str) -> Result<Vec<crate::agents::task::SubTask>, SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, task_id, title, description, mode, status, order_index, created_at, started_at, completed_at
             FROM agent_subtasks WHERE task_id = ?1 ORDER BY order_index",
        )?;

        let subtasks = stmt
            .query_map(params![task_id], |row| {
                // Parse mode (default to FullAccess for backward compatibility)
                let mode_str: Option<String> = row.get(4)?;
                let mode = match mode_str.as_deref() {
                    Some(s) => serde_json::from_str::<crate::agents::task::SubTaskMode>(s)
                        .unwrap_or(crate::agents::task::SubTaskMode::FullAccess),
                    None => crate::agents::task::SubTaskMode::FullAccess,
                };

                // Parse status
                let status_str: String = row.get(5)?;
                let status: crate::agents::task::TaskStatus = match serde_json::from_str(&status_str) {
                    Ok(s) => s,
                    Err(e) => {
                        return Err(rusqlite::Error::ToSqlConversionFailure(
                            Box::new(e) as Box<dyn std::error::Error + Send + Sync>,
                        ));
                    }
                };

                // Parse created_at
                let created_at = chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                    .unwrap_or_default()
                    .with_timezone(&Utc);

                // Parse started_at (optional)
                let started_at = row
                    .get::<_, Option<String>>(8)?
                    .map(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .unwrap_or_default()
                            .with_timezone(&Utc)
                    });

                // Parse completed_at (optional)
                let completed_at = row
                    .get::<_, Option<String>>(9)?
                    .map(|s| {
                        chrono::DateTime::parse_from_rfc3339(&s)
                            .unwrap_or_default()
                            .with_timezone(&Utc)
                    });

                Ok(crate::agents::task::SubTask {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    mode,
                    status,
                    order: row.get(6)?,
                    created_at,
                    started_at,
                    completed_at,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(subtasks)
    }

    /// Delete all subtasks for a task.
    pub fn delete_subtasks(&self, task_id: &str) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        conn.execute("DELETE FROM agent_subtasks WHERE task_id = ?1", params![task_id])?;
        Ok(())
    }

    // ========================================================================
    // Task methods
    // ========================================================================

    /// Get all tasks for a session with their execution steps.
    pub fn get_session_tasks(&self, session_id: &SessionId) -> Result<Vec<crate::agents::task::AgentTask>, SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, request, status, steps, created_at, completed_at
             FROM agent_tasks WHERE session_id = ?1 ORDER BY created_at",
        )?;

        let tasks = stmt
            .query_map(params![session_id.0], |row| {
                let task_id: String = row.get(0)?;
                let request_str: String = row.get(2)?;
                let status_str: String = row.get(3)?;
                let steps_str: String = row.get(4)?;

                let request: crate::agents::task::AgentRequest = serde_json::from_str(&request_str)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e) as Box<dyn std::error::Error + Send + Sync>))?;
                let status: crate::agents::task::TaskStatus = serde_json::from_str(&status_str)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e) as Box<dyn std::error::Error + Send + Sync>))?;
                let steps: Vec<crate::agents::task::ExecutionStep> = serde_json::from_str(&steps_str)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e) as Box<dyn std::error::Error + Send + Sync>))?;

                Ok((crate::agents::task::AgentTask {
                    id: task_id.clone(),
                    session_id: SessionId(row.get(1)?),
                    request,
                    status,
                    steps,
                    subtasks: None, // Will be populated below
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .unwrap_or_default()
                        .with_timezone(&Utc),
                    completed_at: row
                        .get::<_, Option<String>>(6)?
                        .map(|s| chrono::DateTime::parse_from_rfc3339(&s)
                        .unwrap_or_default()
                        .with_timezone(&Utc)),
                }, task_id))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Fetch subtasks for each task
        let tasks_with_subtasks = tasks
            .into_iter()
            .map(|(mut task, task_id)| {
                // Fetch subtasks for this task
                match self.get_subtasks(&task_id) {
                    Ok(subtasks) => {
                        if !subtasks.is_empty() {
                            task.subtasks = Some(subtasks);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to fetch subtasks for task {}: {}", task_id, e);
                    }
                }
                task
            })
            .collect();

        Ok(tasks_with_subtasks)
    }

    // ========================================================================
    // Runtime data methods (files, terminal, status phrases)
    // ========================================================================

    /// Add a file operation record.
    pub fn add_file_touched(
        &self,
        session_id: &SessionId,
        file_path: &str,
        action: &str,
        diff: Option<&str>,
    ) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        conn.execute(
            "INSERT INTO agent_files_touched (id, session_id, file_path, action, diff, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                Uuid::new_v4().to_string(),
                session_id.0,
                file_path,
                action,
                diff,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get all files touched for a session.
    pub fn get_session_files(&self, session_id: &SessionId) -> Result<Vec<TouchedFileEntry>, SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, file_path, action, diff, timestamp
             FROM agent_files_touched WHERE session_id = ?1 ORDER BY timestamp",
        )?;

        let files = stmt
            .query_map(params![session_id.0], |row| {
                Ok(TouchedFileEntry {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    file_path: row.get(2)?,
                    action: row.get(3)?,
                    diff: row.get(4)?,
                    timestamp: row.get(5)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(files)
    }

    /// Add a terminal output line.
    pub fn add_terminal_line(
        &self,
        session_id: &SessionId,
        line_type: &str,
        content: &str,
    ) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        conn.execute(
            "INSERT INTO agent_terminal_output (id, session_id, line_type, content, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                Uuid::new_v4().to_string(),
                session_id.0,
                line_type,
                content,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get all terminal output for a session.
    pub fn get_session_terminal_output(&self, session_id: &SessionId) -> Result<Vec<TerminalLineEntry>, SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, line_type, content, timestamp
             FROM agent_terminal_output WHERE session_id = ?1 ORDER BY timestamp",
        )?;

        let lines = stmt
            .query_map(params![session_id.0], |row| {
                Ok(TerminalLineEntry {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    line_type: row.get(2)?,
                    content: row.get(3)?,
                    timestamp: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(lines)
    }

    /// Add a status phrase.
    pub fn add_status_phrase(
        &self,
        session_id: &SessionId,
        phrase: &str,
    ) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        conn.execute(
            "INSERT INTO agent_status_phrases (id, session_id, phrase, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                Uuid::new_v4().to_string(),
                session_id.0,
                phrase,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    /// Get all status phrases for a session.
    pub fn get_session_status_phrases(&self, session_id: &SessionId) -> Result<Vec<String>, SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT phrase FROM agent_status_phrases WHERE session_id = ?1 ORDER BY timestamp",
        )?;

        let phrases = stmt
            .query_map(params![session_id.0], |row| row.get(0))
            .and_then(|mapped| mapped.collect::<Result<Vec<_>, _>>())?;

        Ok(phrases)
    }

    /// Clear all runtime data for a session (useful for cleanup).
    pub fn clear_session_runtime_data(&self, session_id: &SessionId) -> Result<(), SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        conn.execute("DELETE FROM agent_files_touched WHERE session_id = ?1", params![session_id.0])?;
        conn.execute("DELETE FROM agent_terminal_output WHERE session_id = ?1", params![session_id.0])?;
        conn.execute("DELETE FROM agent_status_phrases WHERE session_id = ?1", params![session_id.0])?;
        conn.execute("DELETE FROM agent_tool_calls WHERE session_id = ?1", params![session_id.0])?;
        Ok(())
    }
}

/// Database entry for a file touched during a session.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TouchedFileEntry {
    pub id: String,
    pub session_id: String,
    pub file_path: String,
    pub action: String,
    pub diff: Option<String>,
    pub timestamp: String,
}

/// Database entry for a terminal output line.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TerminalLineEntry {
    pub id: String,
    pub session_id: String,
    pub line_type: String,
    pub content: String,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_session() -> AgentSession {
        AgentSession::new(
            AgentId::new("code-agent"),
            "Code Agent".to_string(),
            "conv-1".to_string(),
            "thread-1".to_string(),
            None,
        )
    }

    #[test]
    fn test_create_and_get_session() {
        let store = SessionStore::new(":memory:").unwrap();
        let session = make_session();

        store.create_session(&session).unwrap();
        let loaded = store.get_session(&session.id).unwrap();

        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.agent_id.0, "code-agent");
        assert_eq!(loaded.conversation_id, "conv-1");
    }

    #[test]
    fn test_update_state() {
        let store = SessionStore::new(":memory:").unwrap();
        let session = make_session();
        store.create_session(&session).unwrap();

        let new_state = SessionState::Running {
            current_step: 1,
            context_usage: 50.0,
        };
        store.update_state(&session.id, &new_state).unwrap();

        let loaded = store.get_session(&session.id).unwrap().unwrap();
        match &loaded.state {
            SessionState::Running {
                current_step,
                context_usage,
            } => {
                assert_eq!(*current_step, 1);
                assert_eq!(*context_usage, 50.0);
            }
            _ => panic!("Expected Running state"),
        }
    }

    #[test]
    fn test_get_sessions_for_conversation() {
        let store = SessionStore::new(":memory:").unwrap();

        let s1 = make_session();
        let mut s2 = make_session();
        s2.conversation_id = "conv-2".to_string();

        store.create_session(&s1).unwrap();
        store.create_session(&s2).unwrap();

        let sessions = store.get_sessions_for_conversation("conv-1").unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].conversation_id, "conv-1");
    }

    #[test]
    fn test_delete_session() {
        let store = SessionStore::new(":memory:").unwrap();
        let session = make_session();
        store.create_session(&session).unwrap();

        store.delete_session(&session.id).unwrap();
        let loaded = store.get_session(&session.id).unwrap();
        assert!(loaded.is_none());
    }

    // ------------------------------------------------------------------------
    // Helper Functions
    // ------------------------------------------------------------------------

    /// Create a test session for tests.
    fn make_test_session(store: &SessionStore) -> SessionId {
        let session = make_session();
        store.create_session(&session).unwrap();
        session.id
    }

    /// Create a test task for subtask tests (also creates session).
    fn make_test_task(store: &SessionStore, task_id: &str, session_id: &SessionId) {
        let request = crate::agents::task::AgentRequest {
            content: "Test request".to_string(),
            attachments: vec![],
            context: None,
        };

        let task = crate::agents::task::AgentTask {
            id: task_id.to_string(),
            session_id: session_id.clone(),
            request,
            status: crate::agents::task::TaskStatus::Pending,
            steps: vec![],
            created_at: chrono::Utc::now(),
            completed_at: None,
        };

        let conn = store.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO agent_tasks (id, session_id, request, status, steps, created_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            [
                task.id,
                task.session_id.0,
                serde_json::to_string(&task.request).unwrap(),
                serde_json::to_string(&task.status).unwrap(),
                serde_json::to_string(&task.steps).unwrap(),
                task.created_at.to_rfc3339(),
                String::new(),
            ],
        ).unwrap();
    }

    // ------------------------------------------------------------------------
    // SubTask Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_create_subtask_with_mode_persists() {
        let store = SessionStore::new(":memory:").unwrap();
        let session_id = make_test_session(&store);

        // Create task first (foreign key requirement)
        make_test_task(&store, "task-1", &session_id);

        let subtask = crate::agents::task::SubTask::new(
            "task-1".to_string(),
            "Test Subtask".to_string(),
            crate::agents::task::SubTaskMode::Read,
            0,
        );

        store.create_subtask(&subtask).unwrap();

        let fetched = store.get_subtasks("task-1").unwrap();
        assert_eq!(fetched.len(), 1);
        assert_eq!(fetched[0].mode, crate::agents::task::SubTaskMode::Read);
        assert_eq!(fetched[0].title, "Test Subtask");
        assert_eq!(fetched[0].status, crate::agents::task::TaskStatus::Pending);
    }

    #[test]
    fn test_update_subtask_status_to_in_progress_sets_started_at() {
        let store = SessionStore::new(":memory:").unwrap();
        let session_id = make_test_session(&store);

        // Create task first (foreign key requirement)
        make_test_task(&store, "task-1", &session_id);

        let mut subtask = crate::agents::task::SubTask::new(
            "task-1".to_string(),
            "Test Subtask".to_string(),
            crate::agents::task::SubTaskMode::Write,
            0,
        );

        store.create_subtask(&subtask).unwrap();

        // Update status to InProgress should set started_at
        store
            .update_subtask_status(&subtask.id, &crate::agents::task::TaskStatus::InProgress)
            .unwrap();

        let fetched = store.get_subtasks("task-1").unwrap();
        assert_eq!(fetched[0].status, crate::agents::task::TaskStatus::InProgress);
        assert!(fetched[0].started_at.is_some(), "started_at should be set when status becomes InProgress");
    }

    #[test]
    fn test_update_subtask_status_to_completed_sets_completed_at() {
        let store = SessionStore::new(":memory:").unwrap();
        let session_id = make_test_session(&store);

        // Create task first (foreign key requirement)
        make_test_task(&store, "task-1", &session_id);

        let subtask = crate::agents::task::SubTask::new(
            "task-1".to_string(),
            "Test Subtask".to_string(),
            crate::agents::task::SubTaskMode::Execute,
            0,
        );

        store.create_subtask(&subtask).unwrap();

        // Update status to Completed should set completed_at
        store
            .update_subtask_status(&subtask.id, &crate::agents::task::TaskStatus::Completed)
            .unwrap();

        let fetched = store.get_subtasks("task-1").unwrap();
        assert_eq!(fetched[0].status, crate::agents::task::TaskStatus::Completed);
        assert!(fetched[0].completed_at.is_some(), "completed_at should be set when status becomes Completed");
    }

    #[test]
    fn test_get_subtasks_returns_in_order() {
        let store = SessionStore::new(":memory:").unwrap();
        let session_id = make_test_session(&store);

        // Create task first (foreign key requirement)
        make_test_task(&store, "task-1", &session_id);

        // Create subtasks in reverse order
        let subtask2 = crate::agents::task::SubTask::new(
            "task-1".to_string(),
            "Second".to_string(),
            crate::agents::task::SubTaskMode::Read,
            1,
        );
        let subtask1 = crate::agents::task::SubTask::new(
            "task-1".to_string(),
            "First".to_string(),
            crate::agents::task::SubTaskMode::Read,
            0,
        );

        store.create_subtask(&subtask2).unwrap();
        store.create_subtask(&subtask1).unwrap();

        let fetched = store.get_subtasks("task-1").unwrap();
        assert_eq!(fetched.len(), 2);
        assert_eq!(fetched[0].order, 0);
        assert_eq!(fetched[0].title, "First");
        assert_eq!(fetched[1].order, 1);
        assert_eq!(fetched[1].title, "Second");
    }

    #[test]
    fn test_get_subtasks_for_different_tasks() {
        let store = SessionStore::new(":memory:").unwrap();
        let session_id = make_test_session(&store);

        // Create tasks first (foreign key requirement)
        make_test_task(&store, "task-1", &session_id);
        make_test_task(&store, "task-2", &session_id);

        let subtask1 = crate::agents::task::SubTask::new(
            "task-1".to_string(),
            "Task 1 Subtask".to_string(),
            crate::agents::task::SubTaskMode::Read,
            0,
        );
        let subtask2 = crate::agents::task::SubTask::new(
            "task-2".to_string(),
            "Task 2 Subtask".to_string(),
            crate::agents::task::SubTaskMode::Write,
            0,
        );

        store.create_subtask(&subtask1).unwrap();
        store.create_subtask(&subtask2).unwrap();

        let task1_subtasks = store.get_subtasks("task-1").unwrap();
        let task2_subtasks = store.get_subtasks("task-2").unwrap();

        assert_eq!(task1_subtasks.len(), 1);
        assert_eq!(task2_subtasks.len(), 1);
        assert_eq!(task1_subtasks[0].task_id, "task-1");
        assert_eq!(task2_subtasks[0].task_id, "task-2");
    }

    // ------------------------------------------------------------------------
    // Tool Call Tests
    // ------------------------------------------------------------------------

    #[test]
    fn test_tool_call_with_subtask_id() {
        let store = SessionStore::new(":memory:").unwrap();
        let session_id = make_test_session(&store);

        // Create task first (foreign key requirement)
        make_test_task(&store, "task-1", &session_id);

        // Create a subtask
        let subtask = crate::agents::task::SubTask::new(
            "task-1".to_string(),
            "Test Subtask".to_string(),
            crate::agents::task::SubTaskMode::Read,
            0,
        );
        store.create_subtask(&subtask).unwrap();

        // Insert tool call with subtask_id
        let tool_call_id = "tool-call-1";
        store
            .insert_tool_call(
                &session_id,
                tool_call_id,
                "fs_read_file",
                &serde_json::json!({"path": "/test/file.txt"}),
                Some(&subtask.id),
            )
            .unwrap();

        // Get tool calls for session
        let tool_calls = store.get_tool_calls_for_session(&session_id).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].id, tool_call_id);
        assert_eq!(tool_calls[0].tool_name, "fs_read_file");
        assert_eq!(tool_calls[0].subtask_id, Some(subtask.id));
    }

    #[test]
    fn test_tool_call_without_subtask_id() {
        let store = SessionStore::new(":memory:").unwrap();
        let session_id = make_test_session(&store);

        // Insert tool call without subtask_id (before subtask creation)
        let tool_call_id = "tool-call-1";
        store
            .insert_tool_call(
                &session_id,
                tool_call_id,
                "agent_plan_subtasks",
                &serde_json::json!({"subtasks": []}),
                None,
            )
            .unwrap();

        // Get tool calls for session
        let tool_calls = store.get_tool_calls_for_session(&session_id).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].subtask_id, None, "subtask_id should be None for tool calls before subtask creation");
    }

    #[test]
    fn test_update_tool_call_result() {
        let store = SessionStore::new(":memory:").unwrap();
        let session_id = make_test_session(&store);

        let tool_call_id = "tool-call-1";
        store
            .insert_tool_call(
                &session_id,
                tool_call_id,
                "fs_read_file",
                &serde_json::json!({"path": "/test/file.txt"}),
                None,
            )
            .unwrap();

        // Update with result
        let result = serde_json::json!({"content": "file content"});
        store
            .update_tool_call_result(tool_call_id, &result, false, Some(100))
            .unwrap();

        // Verify result was stored
        let tool_calls = store.get_tool_calls_for_session(&session_id).unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert!(tool_calls[0].result.is_some());
        assert_eq!(tool_calls[0].result.as_ref().unwrap(), &result);
        assert_eq!(tool_calls[0].is_error, false);
        assert_eq!(tool_calls[0].duration_ms, Some(100));
    }
}
