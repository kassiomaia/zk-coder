//! Session CRUD operations for the agent store.

use rusqlite::{params, Connection, OptionalExtension};

use super::SessionRepositoryError;
use crate::core::agent::session::{AgentSession, SessionId, SessionState};
use crate::core::agent::types::AgentId;

/// Save a new session.
pub fn create_session(
    conn: &Connection,
    session: &AgentSession,
) -> Result<(), SessionRepositoryError> {
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
pub fn get_session(
    conn: &Connection,
    id: &SessionId,
) -> Result<Option<AgentSession>, SessionRepositoryError> {
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
        Some((
            id,
            agent_id,
            agent_name,
            conv_id,
            thread_id,
            state_str,
            workspace,
            parent,
            created,
            updated,
        )) => {
            let state: SessionState = serde_json::from_str(&state_str)?;
            Ok(Some(AgentSession {
                id: SessionId(id),
                agent_id: AgentId(agent_id),
                agent_name,
                conversation_id: conv_id,
                thread_id,
                state,
                workspace_root: workspace.as_deref().map(std::path::PathBuf::from),
                parent_session_id: parent.as_deref().map(|s| SessionId(s.to_string())),
                created_at: chrono::DateTime::parse_from_rfc3339(&created)
                    .unwrap_or_default()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&updated)
                    .unwrap_or_default()
                    .with_timezone(&chrono::Utc),
            }))
        }
        None => Ok(None),
    }
}

/// Update session state.
pub fn update_state(
    conn: &Connection,
    id: &SessionId,
    state: &SessionState,
) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "UPDATE agent_sessions SET state = ?1, updated_at = ?2 WHERE id = ?3",
        params![
            serde_json::to_string(state)?,
            chrono::Utc::now().to_rfc3339(),
            id.0,
        ],
    )?;
    Ok(())
}

/// Get all sessions for a conversation.
pub fn get_sessions_for_conversation(
    conn: &Connection,
    conversation_id: &str,
) -> Result<Vec<AgentSession>, SessionRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_id, agent_name, conversation_id, thread_id, state, workspace_root, parent_session_id, created_at, updated_at
         FROM agent_sessions
         WHERE conversation_id = ?1
         ORDER BY created_at DESC",
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
        .collect::<Result<Vec<_>, _>>()?;

    sessions
        .into_iter()
        .map(
            |(
                id,
                agent_id,
                agent_name,
                conv_id,
                thread_id,
                state_str,
                workspace,
                parent,
                created,
                updated,
            )| {
                let state: SessionState = serde_json::from_str(&state_str)?;
                Ok(AgentSession {
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
                        .with_timezone(&chrono::Utc),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated)
                        .unwrap_or_default()
                        .with_timezone(&chrono::Utc),
                })
            },
        )
        .collect::<Result<Vec<_>, _>>()
}

/// Delete a session.
///
/// Also deletes all associated tasks and subtasks due to foreign key constraints.
pub fn delete_session(conn: &Connection, id: &SessionId) -> Result<(), SessionRepositoryError> {
    // Delete subtasks first (they reference tasks)
    // Need to delete subtasks for all tasks that belong to this session
    conn.execute(
        "DELETE FROM agent_subtasks WHERE task_id IN (SELECT id FROM agent_tasks WHERE session_id = ?1)",
        params![id.0],
    )?;

    // Delete tasks that belong to this session
    conn.execute(
        "DELETE FROM agent_tasks WHERE session_id = ?1",
        params![id.0],
    )?;

    // Finally delete the session
    conn.execute("DELETE FROM agent_sessions WHERE id = ?1", params![id.0])?;
    Ok(())
}

/// Update workspace root for a session.
pub fn update_workspace(
    conn: &Connection,
    id: &SessionId,
    workspace_root: &str,
) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "UPDATE agent_sessions SET workspace_root = ?1, updated_at = ?2 WHERE id = ?3",
        params![workspace_root, chrono::Utc::now().to_rfc3339(), id.0],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::session::AgentSession;

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
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session = make_session();
        create_session(&conn, &session).unwrap();
        let loaded = get_session(&conn, &session.id).unwrap();

        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.agent_id.0, "code-agent");
        assert_eq!(loaded.conversation_id, "conv-1");
    }

    #[test]
    fn test_update_state() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session = make_session();
        create_session(&conn, &session).unwrap();

        let new_state = SessionState::Running {
            current_step: 1,
            context_usage: 50.0,
        };
        update_state(&conn, &session.id, &new_state).unwrap();

        let loaded = get_session(&conn, &session.id).unwrap().unwrap();
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
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session1 = make_session();
        let session2 = AgentSession::new(
            AgentId::new("code-agent"),
            "Code Agent".to_string(),
            "conv-1".to_string(),
            "thread-2".to_string(),
            None,
        );

        create_session(&conn, &session1).unwrap();
        create_session(&conn, &session2).unwrap();

        let sessions = get_sessions_for_conversation(&conn, "conv-1").unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_delete_session() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session = make_session();
        create_session(&conn, &session).unwrap();
        delete_session(&conn, &session.id).unwrap();

        let loaded = get_session(&conn, &session.id).unwrap();
        assert!(loaded.is_none());
    }

    #[test]
    fn test_update_workspace() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session = make_session();
        create_session(&conn, &session).unwrap();

        update_workspace(&conn, &session.id, "/new/workspace").unwrap();

        let loaded = get_session(&conn, &session.id).unwrap().unwrap();
        assert_eq!(
            loaded.workspace_root,
            Some(std::path::PathBuf::from("/new/workspace"))
        );
    }
}
