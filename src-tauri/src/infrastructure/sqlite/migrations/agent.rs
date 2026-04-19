//! Database schema and migrations for agent session storage.

use rusqlite::Connection;

use crate::infrastructure::sqlite::repositories::agent::SessionRepositoryError;

/// Initialize the database schema.
pub fn init_schema(conn: &Connection) -> Result<(), SessionRepositoryError> {
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
            model_id TEXT,
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
            contract TEXT,
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
            subtask_id TEXT REFERENCES agent_subtasks(id),
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

        CREATE TABLE IF NOT EXISTS agent_execution_snapshots (
            session_id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            step_number INTEGER NOT NULL,
            phase TEXT NOT NULL,
            current_subtask_id TEXT,
            context_usage_percent REAL NOT NULL DEFAULT 0,
            total_input_tokens INTEGER NOT NULL DEFAULT 0,
            total_output_tokens INTEGER NOT NULL DEFAULT 0,
            last_model TEXT,
            last_error TEXT,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE,
            FOREIGN KEY (task_id) REFERENCES agent_tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (current_subtask_id) REFERENCES agent_subtasks(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS agent_runtime_traces (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            task_id TEXT NOT NULL,
            subtask_id TEXT,
            step_number INTEGER NOT NULL,
            trace_type TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE,
            FOREIGN KEY (task_id) REFERENCES agent_tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (subtask_id) REFERENCES agent_subtasks(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_execution_snapshots_updated
            ON agent_execution_snapshots(updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_runtime_traces_session_created
            ON agent_runtime_traces(session_id, created_at ASC);
        CREATE INDEX IF NOT EXISTS idx_runtime_traces_type
            ON agent_runtime_traces(trace_type);

        CREATE TABLE IF NOT EXISTS agent_recovery_events (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            reason TEXT NOT NULL,
            recovered_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_recovery_session_created
            ON agent_recovery_events(session_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS agent_session_memory (
            session_id TEXT PRIMARY KEY,
            task_id TEXT NOT NULL,
            task_goal TEXT NOT NULL,
            active_plan TEXT NOT NULL,
            completed_findings_json TEXT NOT NULL DEFAULT '[]',
            open_questions_json TEXT NOT NULL DEFAULT '[]',
            extracted_facts_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
        );
        ",
    )?;

    // Run migrations for schema updates
    run_migrations(conn)?;

    Ok(())
}

/// Run database migrations for schema updates.
pub fn run_migrations(conn: &Connection) -> Result<(), SessionRepositoryError> {
    // Migration 1: Add mode column to agent_subtasks
    let _ = conn.execute(
        "ALTER TABLE agent_subtasks ADD COLUMN mode TEXT NOT NULL DEFAULT 'full_access'",
        [],
    ); // Ignore error if column exists

    // Migration 2: Add started_at column to agent_subtasks
    let _ = conn.execute("ALTER TABLE agent_subtasks ADD COLUMN started_at TEXT", []); // Ignore error if column exists

    // Migration 3: Add subtask_id column to agent_tool_calls
    let _ = conn.execute(
        "ALTER TABLE agent_tool_calls ADD COLUMN subtask_id TEXT REFERENCES agent_subtasks(id)",
        [],
    ); // Ignore error if column exists

    // Migration 4: Add model_id column to agent_sessions
    let _ = conn.execute("ALTER TABLE agent_sessions ADD COLUMN model_id TEXT", []); // Ignore error if column exists

    // Migration 5: Add title and description columns to agent_tasks
    let _ = conn.execute("ALTER TABLE agent_tasks ADD COLUMN title TEXT", []); // Ignore error if column exists
    let _ = conn.execute("ALTER TABLE agent_tasks ADD COLUMN description TEXT", []); // Ignore error if column exists

    // Migration 6: Add contract column to agent_subtasks (Workstream B.1).
    // Stores the serialized `SubTaskContract` JSON; `NULL` means "no
    // contract declared" and preserves legacy behavior (scope-drift
    // detection short-circuits for that subtask).
    let _ = conn.execute("ALTER TABLE agent_subtasks ADD COLUMN contract TEXT", []); // Ignore error if column exists

    // Migration 7: Add framing column to agent_tasks (Workstream H.2.a).
    // Stores the serialized `TaskFraming` JSON; `NULL` means "no framing
    // declared" and the system-prompt composer falls back to the
    // `default_code_framing()` section.
    let _ = conn.execute("ALTER TABLE agent_tasks ADD COLUMN framing TEXT", []); // Ignore error if column exists

    // Migration 8: Add needs_review flag to agent_sessions (Workstream F.2).
    // Stored as INTEGER (0/1) with a SQL-level default of 0 so pre-F.2
    // rows load as `needs_review = false`. Flipped to 1 when a subtask
    // is explicitly marked Failed through `agent_mark_subtask_status`.
    let _ = conn.execute(
        "ALTER TABLE agent_sessions ADD COLUMN needs_review INTEGER NOT NULL DEFAULT 0",
        [],
    ); // Ignore error if column exists

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_schema_creates_tables() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        // Check that tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"agent_sessions".to_string()));
        assert!(tables.contains(&"agent_tasks".to_string()));
        assert!(tables.contains(&"agent_subtasks".to_string()));
        assert!(tables.contains(&"agent_files_touched".to_string()));
        assert!(tables.contains(&"agent_terminal_output".to_string()));
        assert!(tables.contains(&"agent_status_phrases".to_string()));
        assert!(tables.contains(&"agent_tool_calls".to_string()));
        assert!(tables.contains(&"agent_execution_steps".to_string()));
        assert!(tables.contains(&"agent_execution_snapshots".to_string()));
        assert!(tables.contains(&"agent_runtime_traces".to_string()));
        assert!(tables.contains(&"agent_recovery_events".to_string()));
    }

    #[test]
    fn test_init_schema_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        // Running again should not fail
        init_schema(&conn).unwrap();
    }

    #[test]
    fn test_migrations_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        // Run migrations again - should not fail
        run_migrations(&conn).unwrap();
    }
}
