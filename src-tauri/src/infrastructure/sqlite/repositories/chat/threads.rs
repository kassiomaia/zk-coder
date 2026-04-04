//! Thread CRUD operations for the chat store.

use chrono::Utc;
use log::warn;
use rusqlite::{params, Connection, OptionalExtension};

use super::helpers::parse_datetime;
use super::ChatRepositoryError;
use crate::core::chat::entities::{MessageRole, Thread, ThreadMetadata};

/// Creates a new thread in a conversation.
pub fn create_thread(
    conn: &Connection,
    conversation_id: &str,
    title: &str,
) -> Result<Thread, ChatRepositoryError> {
    let thread = Thread::new(conversation_id, title);
    let metadata_json = serde_json::to_string(&thread.metadata)?;

    conn.execute(
        "INSERT INTO threads (id, conversation_id, parent_id, title, created_at, updated_at, archived, message_count, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            &thread.id,
            &thread.conversation_id,
            &thread.parent_id,
            &thread.title,
            &thread.created_at.to_rfc3339(),
            &thread.updated_at.to_rfc3339(),
            i64::from(thread.metadata.archived),
            thread.message_count as i64,
            &metadata_json,
        ],
    )?;

    Ok(thread)
}

/// Loads all threads for a conversation.
pub fn load_threads(
    conn: &Connection,
    conversation_id: &str,
) -> Result<Vec<Thread>, ChatRepositoryError> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, parent_id, title, created_at, updated_at, archived, message_count, metadata
         FROM threads WHERE conversation_id = ?1 AND archived = 0 ORDER BY updated_at DESC",
    )?;

    let threads = stmt
        .query_map(params![conversation_id], |row| {
            let thread_id: String = row.get(0)?;
            let metadata_json: String = row.get(8)?;
            let metadata = serde_json::from_str(&metadata_json)
                .unwrap_or_else(|e| {
                    warn!("Failed to parse thread metadata for thread {thread_id} in conversation {conversation_id}: {e}");
                    ThreadMetadata::default()
                });
            let mut metadata = metadata;
            metadata.archived = row.get::<_, i64>(6)? != 0;
            Ok(Thread {
                id: thread_id,
                conversation_id: row.get(1)?,
                parent_id: row.get(2)?,
                title: row.get(3)?,
                created_at: parse_datetime(&row.get::<_, String>(4)?),
                updated_at: parse_datetime(&row.get::<_, String>(5)?),
                message_count: row.get::<_, i64>(7)? as usize,
                metadata,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(threads)
}

/// Loads a single thread by ID.
pub fn load_thread(conn: &Connection, id: &str) -> Result<Thread, ChatRepositoryError> {
    let row = conn
        .query_row(
            "SELECT id, conversation_id, parent_id, title, created_at, updated_at, archived, message_count, metadata
             FROM threads WHERE id = ?1",
            params![id],
            |row| {
                let metadata_json: String = row.get(8)?;
                let metadata = serde_json::from_str(&metadata_json)
                    .unwrap_or_else(|e| {
                        warn!("Failed to parse thread metadata for thread {id}: {e}");
                        ThreadMetadata::default()
                    });
                let mut metadata = metadata;
                metadata.archived = row.get::<_, i64>(6)? != 0;
                Ok(Thread {
                    id: row.get(0)?,
                    conversation_id: row.get(1)?,
                    parent_id: row.get(2)?,
                    title: row.get(3)?,
                    created_at: parse_datetime(&row.get::<_, String>(4)?),
                    updated_at: parse_datetime(&row.get::<_, String>(5)?),
                    message_count: row.get::<_, i64>(7)? as usize,
                    metadata,
                })
            },
        )
        .optional()?;

    row.ok_or(ChatRepositoryError::NotFound("Thread", id.to_string()))
}

/// Updates a thread's resolved status.
pub fn update_thread_resolved(
    conn: &Connection,
    id: &str,
    resolved: bool,
) -> Result<(), ChatRepositoryError> {
    let metadata_json: Option<String> = conn
        .query_row(
            "SELECT metadata FROM threads WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?;

    let mut metadata: ThreadMetadata = metadata_json
        .as_deref()
        .and_then(|j| {
            serde_json::from_str(j)
                .inspect_err(|e| {
                    warn!("Failed to parse thread metadata for thread {id} when updating resolved state, using default: {e}");
                })
                .ok()
        })
        .unwrap_or_default();
    metadata.resolved = resolved;

    let new_metadata_json = serde_json::to_string(&metadata)?;
    let now = Utc::now();

    conn.execute(
        "UPDATE threads SET metadata = ?1, archived = ?2, updated_at = ?3 WHERE id = ?4",
        params![
            new_metadata_json,
            i64::from(metadata.archived),
            now.to_rfc3339(),
            id
        ],
    )?;

    Ok(())
}

/// Updates a thread's archived status.
pub fn update_thread_archived(
    conn: &Connection,
    id: &str,
    archived: bool,
) -> Result<(), ChatRepositoryError> {
    let metadata_json: Option<String> = conn
        .query_row(
            "SELECT metadata FROM threads WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?;

    let mut metadata: ThreadMetadata = metadata_json
        .as_deref()
        .and_then(|j| {
            serde_json::from_str(j)
                .inspect_err(|e| {
                    warn!("Failed to parse thread metadata for thread {id} when updating archived state, using default: {e}");
                })
                .ok()
        })
        .unwrap_or_default();
    metadata.archived = archived;

    let new_metadata_json = serde_json::to_string(&metadata)?;
    let now = Utc::now();
    let rows_affected = conn.execute(
        "UPDATE threads SET metadata = ?1, archived = ?2, updated_at = ?3 WHERE id = ?4",
        params![new_metadata_json, i64::from(archived), now.to_rfc3339(), id],
    )?;

    if rows_affected == 0 {
        return Err(ChatRepositoryError::NotFound("Thread", id.to_string()));
    }

    Ok(())
}

/// Sets the agent session ID for a thread.
pub fn set_thread_agent_session(
    conn: &Connection,
    id: &str,
    agent_session_id: Option<String>,
) -> Result<(), ChatRepositoryError> {
    let metadata_json: Option<String> = conn
        .query_row(
            "SELECT metadata FROM threads WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()?;

    let mut metadata: ThreadMetadata = metadata_json
        .as_deref()
        .and_then(|j| {
            serde_json::from_str(j)
                .inspect_err(|e| {
                    warn!("Failed to parse thread metadata for thread {id} when setting agent session, using default: {e}");
                })
                .ok()
        })
        .unwrap_or_default();
    metadata.agent_session_id = agent_session_id;

    let new_metadata_json = serde_json::to_string(&metadata)?;
    let now = Utc::now();

    conn.execute(
        "UPDATE threads SET metadata = ?1, archived = ?2, updated_at = ?3 WHERE id = ?4",
        params![
            new_metadata_json,
            i64::from(metadata.archived),
            now.to_rfc3339(),
            id
        ],
    )?;

    Ok(())
}

/// Deletes a thread and all its messages.
pub fn delete_thread(conn: &Connection, id: &str) -> Result<(), ChatRepositoryError> {
    let rows_affected = conn.execute("DELETE FROM threads WHERE id = ?1", params![id])?;
    if rows_affected == 0 {
        return Err(ChatRepositoryError::NotFound("Thread", id.to_string()));
    }
    Ok(())
}

/// Renames a thread.
pub fn rename_thread(
    conn: &Connection,
    id: &str,
    title: &str,
) -> Result<Thread, ChatRepositoryError> {
    let now = Utc::now();
    let rows_affected = conn.execute(
        "UPDATE threads SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![title, now.to_rfc3339(), id],
    )?;

    if rows_affected == 0 {
        return Err(ChatRepositoryError::NotFound("Thread", id.to_string()));
    }

    load_thread(conn, id)
}

/// Creates a thread with its first message atomically.
pub fn create_thread_with_message(
    conn: &Connection,
    conversation_id: &str,
    thread_title: &str,
    message_content: &str,
    message_role: MessageRole,
) -> Result<(Thread, crate::core::chat::entities::ChatMessage), ChatRepositoryError> {
    let now = Utc::now();

    // Create thread
    let thread = Thread::new(conversation_id, thread_title);
    let metadata_json = serde_json::to_string(&thread.metadata)?;

    conn.execute(
        "INSERT INTO threads (id, conversation_id, parent_id, title, created_at, updated_at, archived, message_count, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            &thread.id,
            &thread.conversation_id,
            &thread.parent_id,
            &thread.title,
            &thread.created_at.to_rfc3339(),
            &thread.updated_at.to_rfc3339(),
            i64::from(thread.metadata.archived),
            1i64, // Start with 1 message
            &metadata_json,
        ],
    )?;

    // Create message with thread_id
    let message = crate::core::chat::entities::ChatMessage {
        id: uuid::Uuid::new_v4().to_string(),
        conversation_id: conversation_id.to_string(),
        thread_id: Some(thread.id.clone()),
        role: message_role,
        content: message_content.to_string(),
        timestamp: now,
        status: crate::core::chat::entities::MessageStatus::Sent,
        metadata: None,
    };

    conn.execute(
        "INSERT INTO chat_messages (id, conversation_id, thread_id, role, content, timestamp, status, metadata)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            &message.id,
            &message.conversation_id,
            &message.thread_id,
            message.role.as_str(),
            &message.content,
            &message.timestamp.to_rfc3339(),
            super::helpers::status_to_str(&message.status),
            None::<String>, // metadata
        ],
    )?;

    // Update conversation's updated_at
    conn.execute(
        "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
        params![now.to_rfc3339(), conversation_id],
    )?;

    // Update thread's message_count to match database
    let mut thread = thread;
    thread.message_count = 1;

    Ok((thread, message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chat::entities::ChatMessage;
    use crate::infrastructure::sqlite::migrations::chat_schema;

    fn make_test_store() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::chat_schema::init_schema(&conn).unwrap();
        conn
    }

    fn make_test_conversation(conn: &Connection) -> String {
        super::super::conversations::create_conversation(conn, "Test Conv")
            .unwrap()
            .id
    }

    #[test]
    fn test_create_and_load_thread() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let thread = create_thread(&conn, &conv_id, "Test Thread").unwrap();
        assert_eq!(thread.conversation_id, conv_id);
        assert_eq!(thread.title, "Test Thread");

        let threads = load_threads(&conn, &conv_id).unwrap();
        assert_eq!(threads.len(), 1);
    }

    #[test]
    fn test_load_single_thread() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let thread = create_thread(&conn, &conv_id, "Test Thread").unwrap();
        let loaded = load_thread(&conn, &thread.id).expect("Failed to load thread");
        assert_eq!(loaded.id, thread.id);
        assert_eq!(loaded.title, thread.title);
        assert!(!loaded.metadata.resolved);
    }

    #[test]
    fn test_thread_resolved_status() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let thread = create_thread(&conn, &conv_id, "Test Thread").unwrap();
        assert!(!thread.metadata.resolved);

        update_thread_resolved(&conn, &thread.id, true).unwrap();

        let loaded = load_thread(&conn, &thread.id).unwrap();
        assert!(loaded.metadata.resolved);

        update_thread_resolved(&conn, &thread.id, false).unwrap();

        let loaded = load_thread(&conn, &thread.id).unwrap();
        assert!(!loaded.metadata.resolved);
    }

    #[test]
    fn test_archived_thread_is_hidden_from_load_threads_but_loadable_directly() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let thread = create_thread(&conn, &conv_id, "Archived Thread").unwrap();
        update_thread_archived(&conn, &thread.id, true).unwrap();

        let visible_threads = load_threads(&conn, &conv_id).unwrap();
        assert!(visible_threads.is_empty());

        let archived = load_thread(&conn, &thread.id).unwrap();
        assert!(archived.metadata.archived);
        assert_eq!(archived.title, "Archived Thread");
    }

    #[test]
    fn test_delete_thread() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let thread = create_thread(&conn, &conv_id, "Test Thread").unwrap();
        delete_thread(&conn, &thread.id).unwrap();

        let result = load_thread(&conn, &thread.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_thread() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let thread = create_thread(&conn, &conv_id, "Old Title").unwrap();
        let renamed = rename_thread(&conn, &thread.id, "New Title").unwrap();
        assert_eq!(renamed.title, "New Title");
    }

    #[test]
    fn test_create_thread_with_message() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let (thread, message) = create_thread_with_message(
            &conn,
            &conv_id,
            "Thread Title",
            "Hello world",
            MessageRole::User,
        )
        .unwrap();

        assert_eq!(thread.conversation_id, conv_id);
        assert_eq!(thread.title, "Thread Title");
        assert_eq!(thread.message_count, 1);
        assert_eq!(message.content, "Hello world");
        assert_eq!(message.thread_id, Some(thread.id.clone()));
    }

    #[test]
    fn test_set_thread_agent_session() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let thread = create_thread(&conn, &conv_id, "Test Thread").unwrap();
        set_thread_agent_session(&conn, &thread.id, Some("agent-session-123".to_string())).unwrap();

        let loaded = load_thread(&conn, &thread.id).unwrap();
        assert_eq!(
            loaded.metadata.agent_session_id,
            Some("agent-session-123".to_string())
        );
    }

    #[test]
    fn test_load_thread_not_found() {
        let conn = make_test_store();
        let result = load_thread(&conn, "nonexistent");
        assert!(matches!(result, Err(ChatRepositoryError::NotFound(_, _))));
    }

    #[test]
    fn test_load_threads_ordered_by_updated_at() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let thread1 = create_thread(&conn, &conv_id, "First").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let thread2 = create_thread(&conn, &conv_id, "Second").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));

        let thread3 = create_thread(&conn, &conv_id, "Third").unwrap();

        // Update thread1 to make it most recent
        let _ = rename_thread(&conn, &thread1.id, "Updated First");

        let threads = load_threads(&conn, &conv_id).unwrap();
        assert_eq!(threads.len(), 3);
        assert_eq!(threads[0].id, thread1.id); // Most recently updated
        assert_eq!(threads[1].id, thread3.id);
        assert_eq!(threads[2].id, thread2.id);
    }

    #[test]
    fn test_delete_thread_fails_for_nonexistent() {
        let conn = make_test_store();
        // Should return NotFound error when deleting nonexistent thread
        let result = delete_thread(&conn, "nonexistent");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(ChatRepositoryError::NotFound("Thread", _))
        ));
    }

    #[test]
    fn test_rename_nonexistent_thread() {
        let conn = make_test_store();
        let result = rename_thread(&conn, "nonexistent", "New Title");
        assert!(matches!(result, Err(ChatRepositoryError::NotFound(_, _))));
    }

    #[test]
    fn test_thread_metadata_serialization() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let thread = create_thread(&conn, &conv_id, "Test").unwrap();
        assert!(!thread.metadata.resolved);
        assert_eq!(thread.metadata.agent_session_id, None);

        // Set agent session
        set_thread_agent_session(&conn, &thread.id, Some("session-abc".to_string())).unwrap();
        let loaded = load_thread(&conn, &thread.id).unwrap();
        assert_eq!(
            loaded.metadata.agent_session_id,
            Some("session-abc".to_string())
        );

        // Clear agent session
        set_thread_agent_session(&conn, &thread.id, None).unwrap();
        let loaded = load_thread(&conn, &thread.id).unwrap();
        assert_eq!(loaded.metadata.agent_session_id, None);
    }

    #[test]
    fn test_create_thread_with_thread_id_in_message() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let (thread, message) =
            create_thread_with_message(&conn, &conv_id, "Thread Title", "Hello", MessageRole::User)
                .unwrap();

        assert_eq!(thread.message_count, 1);
        assert_eq!(message.thread_id, Some(thread.id.clone()));
        assert_eq!(message.role, MessageRole::User);
        assert_eq!(message.content, "Hello");
    }

    #[test]
    fn test_create_thread_with_different_roles() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let (_thread, message) = create_thread_with_message(
            &conn,
            &conv_id,
            "Assistant Thread",
            "I can help!",
            MessageRole::Assistant,
        )
        .unwrap();

        assert_eq!(message.role, MessageRole::Assistant);
        assert_eq!(message.content, "I can help!");
    }

    #[test]
    fn test_threads_for_conversation() {
        let conn = make_test_store();
        let conv_id1 = make_test_conversation(&conn);
        let conv_id2 = make_test_conversation(&conn);

        create_thread(&conn, &conv_id1, "Thread 1").unwrap();
        create_thread(&conn, &conv_id1, "Thread 2").unwrap();
        create_thread(&conn, &conv_id2, "Thread 3").unwrap();

        let threads1 = load_threads(&conn, &conv_id1).unwrap();
        let threads2 = load_threads(&conn, &conv_id2).unwrap();

        assert_eq!(threads1.len(), 2);
        assert_eq!(threads2.len(), 1);
    }
}
