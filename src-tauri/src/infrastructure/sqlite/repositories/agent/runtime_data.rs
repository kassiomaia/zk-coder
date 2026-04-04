//! Runtime data operations for the agent store.

//! Includes files touched, terminal output, and status phrases.

use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

use super::SessionRepositoryError;
use crate::core::agent::session::SessionId;

/// Database entry for a file touched during a session.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TouchedFileEntry {
    /// Unique identifier for the file touch entry.
    pub id: String,
    /// Session identifier.
    pub session_id: String,
    /// Path to the file that was touched.
    pub file_path: String,
    /// Action performed on the file (e.g., "read", "write", "delete").
    pub action: String,
    /// Optional diff of changes made to the file.
    pub diff: Option<String>,
    /// Timestamp when the file was touched.
    pub timestamp: String,
}

/// Database entry for a terminal output line.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TerminalLineEntry {
    /// Unique identifier for the terminal line entry.
    pub id: String,
    /// Session identifier.
    pub session_id: String,
    /// Type of line (e.g., "stdout", "stderr").
    pub line_type: String,
    /// Content of the terminal line.
    pub content: String,
    /// Timestamp when the line was output.
    pub timestamp: String,
}

/// Add a file operation record.
pub fn add_file_touched(
    conn: &Connection,
    session_id: &SessionId,
    file_path: &str,
    action: &str,
    diff: Option<&str>,
) -> Result<(), SessionRepositoryError> {
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
pub fn get_session_files(
    conn: &Connection,
    session_id: &SessionId,
) -> Result<Vec<TouchedFileEntry>, SessionRepositoryError> {
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
        .filter_map(std::result::Result::ok)
        .collect();

    Ok(files)
}

/// Add a terminal output line.
pub fn add_terminal_line(
    conn: &Connection,
    session_id: &SessionId,
    line_type: &str,
    content: &str,
) -> Result<(), SessionRepositoryError> {
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
pub fn get_session_terminal_output(
    conn: &Connection,
    session_id: &SessionId,
) -> Result<Vec<TerminalLineEntry>, SessionRepositoryError> {
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
        .filter_map(std::result::Result::ok)
        .collect();

    Ok(lines)
}

/// Add a status phrase.
pub fn add_status_phrase(
    conn: &Connection,
    session_id: &SessionId,
    phrase: &str,
) -> Result<(), SessionRepositoryError> {
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
pub fn get_session_status_phrases(
    conn: &Connection,
    session_id: &SessionId,
) -> Result<Vec<String>, SessionRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT phrase FROM agent_status_phrases WHERE session_id = ?1 ORDER BY timestamp",
    )?;

    let phrases = stmt
        .query_map(params![session_id.0], |row| row.get(0))
        .and_then(|mapped| mapped.collect::<Result<Vec<_>, _>>())?;

    Ok(phrases)
}

/// Clear all runtime data for a session.
pub fn clear_session_runtime_data(
    conn: &Connection,
    session_id: &SessionId,
) -> Result<(), SessionRepositoryError> {
    conn.execute(
        "DELETE FROM agent_files_touched WHERE session_id = ?1",
        params![session_id.0],
    )?;
    conn.execute(
        "DELETE FROM agent_terminal_output WHERE session_id = ?1",
        params![session_id.0],
    )?;
    conn.execute(
        "DELETE FROM agent_status_phrases WHERE session_id = ?1",
        params![session_id.0],
    )?;
    conn.execute(
        "DELETE FROM agent_tool_calls WHERE session_id = ?1",
        params![session_id.0],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::agent::session::{AgentSession, SessionId};
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

    #[test]
    fn test_add_and_get_files() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        add_file_touched(
            &conn,
            &session_id,
            "/test/file.txt",
            "write",
            Some("diff content"),
        )
        .unwrap();

        let files = get_session_files(&conn, &session_id).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_path, "/test/file.txt");
        assert_eq!(files[0].action, "write");
    }

    #[test]
    fn test_add_and_get_terminal_output() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        add_terminal_line(&conn, &session_id, "stdout", "output line").unwrap();

        let lines = get_session_terminal_output(&conn, &session_id).unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].content, "output line");
    }

    #[test]
    fn test_add_and_get_status_phrases() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        add_status_phrase(&conn, &session_id, "Working on task...").unwrap();

        let phrases = get_session_status_phrases(&conn, &session_id).unwrap();
        assert_eq!(phrases.len(), 1);
        assert_eq!(phrases[0], "Working on task...");
    }

    #[test]
    fn test_clear_session_runtime_data() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::agent::init_schema(&conn).unwrap();

        let session_id = make_test_session(&conn);
        add_file_touched(&conn, &session_id, "/test/file.txt", "write", None).unwrap();
        add_terminal_line(&conn, &session_id, "stdout", "output").unwrap();
        add_status_phrase(&conn, &session_id, "status").unwrap();

        clear_session_runtime_data(&conn, &session_id).unwrap();

        let files = get_session_files(&conn, &session_id).unwrap();
        let lines = get_session_terminal_output(&conn, &session_id).unwrap();
        let phrases = get_session_status_phrases(&conn, &session_id).unwrap();

        assert_eq!(files.len(), 0);
        assert_eq!(lines.len(), 0);
        assert_eq!(phrases.len(), 0);
    }
}
