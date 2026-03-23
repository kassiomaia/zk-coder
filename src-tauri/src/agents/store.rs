//! SQLite storage for agent sessions and tasks.
//!
//! Persists agent sessions across app restarts so users can
//! resume incomplete tasks.

use std::sync::Mutex;

use chrono::Utc;
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
    conn: Mutex<Connection>,
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
                status TEXT NOT NULL,
                order_index INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                FOREIGN KEY (task_id) REFERENCES agent_tasks(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_sessions_conversation
                ON agent_sessions(conversation_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_thread
                ON agent_sessions(thread_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_session
                ON agent_tasks(session_id);
            ",
        )?;
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
            "INSERT INTO agent_subtasks (id, task_id, title, description, status, order_index, created_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                subtask.id,
                subtask.task_id,
                subtask.title,
                subtask.description,
                serde_json::to_string(&subtask.status)?,
                subtask.order,
                subtask.created_at.to_rfc3339(),
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
            "UPDATE agent_subtasks SET status = ?1, completed_at = ?2 WHERE id = ?3",
            params![serde_json::to_string(status)?, completed_at, subtask_id],
        )?;
        Ok(())
    }

    /// Get all subtasks for a task.
    pub fn get_subtasks(&self, task_id: &str) -> Result<Vec<crate::agents::task::SubTask>, SessionStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| SessionStoreError::LockError(e.to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, task_id, title, description, status, order_index, created_at, completed_at
             FROM agent_subtasks WHERE task_id = ?1 ORDER BY order_index",
        )?;

        let subtasks = stmt
            .query_map(params![task_id], |row| {
                let status_str: String = row.get(4)?;
                let status: crate::agents::task::TaskStatus = match serde_json::from_str(&status_str) {
                    Ok(s) => s,
                    Err(e) => {
                        return Err(rusqlite::Error::ToSqlConversionFailure(
                            Box::new(e) as Box<dyn std::error::Error + Send + Sync>,
                        ));
                    }
                };
                Ok(crate::agents::task::SubTask {
                    id: row.get(0)?,
                    task_id: row.get(1)?,
                    title: row.get(2)?,
                    description: row.get(3)?,
                    status,
                    order: row.get(5)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .unwrap_or_default()
                        .with_timezone(&Utc),
                    completed_at: row
                        .get::<_, Option<String>>(7)?
                        .map(|s| {
                            chrono::DateTime::parse_from_rfc3339(&s)
                                .unwrap_or_default()
                                .with_timezone(&Utc)
                        }),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::types::TrustMode;

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
            max_steps: 6,
        };
        store.update_state(&session.id, &new_state).unwrap();

        let loaded = store.get_session(&session.id).unwrap().unwrap();
        match loaded.state {
            SessionState::Running {
                current_step,
                max_steps,
            } => {
                assert_eq!(current_step, 1);
                assert_eq!(max_steps, 6);
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
}
