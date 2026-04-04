//! Database schema initialization for chat store.

use rusqlite::Connection;

use crate::infrastructure::sqlite::repositories::chat::ChatRepositoryError;

/// Initializes the database schema for the chat store.
///
/// Creates tables for conversations, threads, and chat messages with
/// appropriate indexes for efficient querying.
pub fn init_schema(conn: &Connection) -> Result<(), ChatRepositoryError> {
    conn.execute_batch(
        r"
        -- Conversations table
        CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            archived INTEGER NOT NULL DEFAULT 0,
            settings TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_conversations_updated
        ON conversations(updated_at DESC);

        -- Threads table
        CREATE TABLE IF NOT EXISTS threads (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            parent_id TEXT,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            archived INTEGER NOT NULL DEFAULT 0,
            message_count INTEGER NOT NULL DEFAULT 0,
            metadata TEXT NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (parent_id) REFERENCES threads(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_threads_conversation
        ON threads(conversation_id);

        CREATE INDEX IF NOT EXISTS idx_threads_updated
        ON threads(updated_at DESC);

        -- Messages table
        CREATE TABLE IF NOT EXISTS chat_messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            thread_id TEXT,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'sent',
            metadata TEXT,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE,
            FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE SET NULL
        );

        CREATE INDEX IF NOT EXISTS idx_messages_conversation
        ON chat_messages(conversation_id, timestamp ASC);

        CREATE INDEX IF NOT EXISTS idx_messages_thread
        ON chat_messages(thread_id, timestamp ASC);
        ",
    )?;
    ensure_column_exists(
        conn,
        "conversations",
        "archived",
        "ALTER TABLE conversations ADD COLUMN archived INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column_exists(
        conn,
        "threads",
        "archived",
        "ALTER TABLE threads ADD COLUMN archived INTEGER NOT NULL DEFAULT 0",
    )?;
    Ok(())
}

fn ensure_column_exists(
    conn: &Connection,
    table_name: &str,
    column_name: &str,
    alter_sql: &str,
) -> Result<(), ChatRepositoryError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table_name})"))?;
    let has_column = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .any(|name| name == column_name);

    if !has_column {
        conn.execute(alter_sql, [])?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_schema() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();

        assert!(tables.contains(&"conversations".to_string()));
        assert!(tables.contains(&"threads".to_string()));
        assert!(tables.contains(&"chat_messages".to_string()));
    }
}
