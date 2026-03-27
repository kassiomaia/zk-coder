//! SQLite schema for NLP context persistence.

use rusqlite::Connection;

use crate::infrastructure::sqlite::repositories::nlp_context::ContextRepositoryError;

pub fn init_schema(conn: &Connection) -> Result<(), ContextRepositoryError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS context_windows (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL UNIQUE,
            thread_id TEXT,
            metadata TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS context_messages (
            id TEXT PRIMARY KEY,
            context_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            tokens INTEGER,
            FOREIGN KEY (context_id) REFERENCES context_windows(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_messages_context
        ON context_messages(context_id);

        CREATE INDEX IF NOT EXISTS idx_conversation_id
        ON context_windows(conversation_id);
        "#,
    )?;
    Ok(())
}
