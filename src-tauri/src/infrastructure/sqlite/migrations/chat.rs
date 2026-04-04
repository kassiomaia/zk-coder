//! Migration operations for the chat store.

use chrono::Utc;
use rusqlite::{params, Connection};
use std::collections::HashMap;

use crate::core::chat::entities::ThreadMetadata;
use crate::infrastructure::sqlite::repositories::chat::ChatRepositoryError;

/// Migrates orphan messages (messages without a thread) to new threads.
///
/// For each conversation that has messages with NULL `thread_id`:
/// 1. Creates a new thread using the first message content as the title
/// 2. Updates all orphan messages in that conversation to reference the new thread
///
/// This migration is idempotent (safe to run multiple times).
pub fn migrate_orphan_messages_to_threads(conn: &Connection) -> Result<(), ChatRepositoryError> {
    // Find conversations with orphan messages
    let orphan_conversations: Vec<(String, String, String)> = conn
        .prepare(
            "SELECT DISTINCT m.conversation_id, m.id, m.content
             FROM chat_messages m
             WHERE m.thread_id IS NULL
             ORDER BY m.timestamp ASC",
        )?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    // Group by conversation_id to get the first message per conversation
    let mut first_messages: HashMap<String, (String, String)> = HashMap::new();
    for (conv_id, msg_id, content) in orphan_conversations {
        if !first_messages.contains_key(&conv_id) {
            first_messages.insert(conv_id.clone(), (msg_id, content));
        }
    }

    // For each conversation with orphan messages, create a thread and update messages
    let now = Utc::now();
    for (conv_id, (_first_msg_id, first_msg_content)) in first_messages {
        // Derive thread title from first message content (truncated to ~50 chars)
        let thread_title = derive_thread_title(&first_msg_content);

        // Create thread
        let thread_id = uuid::Uuid::new_v4().to_string();
        let metadata_json = serde_json::to_string(&ThreadMetadata::default())?;

        conn.execute(
            "INSERT INTO threads (id, conversation_id, parent_id, title, created_at, updated_at, archived, message_count, metadata)
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, 0, 0, ?6)",
            params![
                &thread_id,
                &conv_id,
                &thread_title,
                now.to_rfc3339(),
                now.to_rfc3339(),
                &metadata_json,
            ],
        )?;

        // Update orphan messages to reference the new thread
        conn.execute(
            "UPDATE chat_messages SET thread_id = ?1 WHERE conversation_id = ?2 AND thread_id IS NULL",
            params![&thread_id, &conv_id],
        )?;

        // Update thread message count
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM chat_messages WHERE thread_id = ?1",
            params![&thread_id],
            |row| row.get(0),
        )?;

        conn.execute(
            "UPDATE threads SET message_count = ?1 WHERE id = ?2",
            params![count, &thread_id],
        )?;
    }

    Ok(())
}

/// Derives a thread title from message content.
fn derive_thread_title(content: &str) -> String {
    let cleaned = content
        .replace(['#', '*', '`'], "")
        .replace('\n', " ")
        .trim()
        .to_string();

    if cleaned.is_empty() {
        return "New Thread".to_string();
    }

    if cleaned.len() <= 50 {
        return cleaned;
    }

    let truncated = &cleaned[..50];
    if let Some(last_space) = truncated.rfind(' ') {
        if last_space > 20 {
            format!("{}...", &truncated[..last_space])
        } else {
            format!("{truncated}...")
        }
    } else {
        format!("{truncated}...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::chat::entities::ChatMessage;

    #[test]
    fn test_derive_thread_title_short() {
        assert_eq!(derive_thread_title("Hello"), "Hello");
        assert_eq!(derive_thread_title("A".repeat(50).as_str()), "A".repeat(50));
    }

    #[test]
    fn test_derive_thread_title_long() {
        let long = "A".repeat(100);
        let result = derive_thread_title(&long);
        assert!(result.len() <= 54); // 50 + "..."
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_derive_thread_title_with_space() {
        let text = "This is a moderately long message that needs truncation";
        let result = derive_thread_title(text);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_derive_thread_title_empty() {
        assert_eq!(derive_thread_title(""), "New Thread");
        assert_eq!(derive_thread_title("   \n\n   "), "New Thread");
    }

    #[test]
    fn test_derive_thread_title_strips_markdown() {
        assert_eq!(derive_thread_title("# Header"), "Header");
        assert_eq!(derive_thread_title("**bold**"), "bold");
        assert_eq!(derive_thread_title("`code`"), "code");
        assert_eq!(derive_thread_title("line1\nline2"), "line1 line2");
    }

    #[test]
    fn test_migrate_orphan_messages() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::chat_schema::init_schema(&conn).unwrap();

        // Create a conversation
        let conv =
            crate::infrastructure::sqlite::repositories::chat::conversations::create_conversation(
                &conn, "Test",
            )
            .unwrap();

        // Add messages without thread_id (orphans)
        let msg1 = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conv.id.clone(),
            thread_id: None,
            ..ChatMessage::user(&conv.id, "First message")
        };

        let msg2 = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conv.id.clone(),
            thread_id: None,
            ..ChatMessage::user(&conv.id, "Second message")
        };

        // Insert messages directly
        conn.execute(
            "INSERT INTO chat_messages (id, conversation_id, thread_id, role, content, timestamp, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &msg1.id,
                &msg1.conversation_id,
                &msg1.thread_id,
                msg1.role.as_str(),
                &msg1.content,
                msg1.timestamp.to_rfc3339(),
                "sent",
                None::<String>,
            ],
        ).unwrap();

        conn.execute(
            "INSERT INTO chat_messages (id, conversation_id, thread_id, role, content, timestamp, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &msg2.id,
                &msg2.conversation_id,
                &msg2.thread_id,
                msg2.role.as_str(),
                &msg2.content,
                msg2.timestamp.to_rfc3339(),
                "sent",
                None::<String>,
            ],
        ).unwrap();

        // Migrate
        migrate_orphan_messages_to_threads(&conn).unwrap();

        // Verify thread was created
        let threads = crate::infrastructure::sqlite::repositories::chat::threads::load_threads(
            &conn, &conv.id,
        )
        .unwrap();
        assert_eq!(threads.len(), 1);

        // Verify messages now have thread_id
        let messages = crate::infrastructure::sqlite::repositories::chat::messages::load_messages(
            &conn, &conv.id,
        )
        .unwrap();
        assert!(messages[0].thread_id.is_some());
        assert!(messages[1].thread_id.is_some());
    }

    #[test]
    fn test_migrate_orphan_messages_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::chat_schema::init_schema(&conn).unwrap();

        let conv =
            crate::infrastructure::sqlite::repositories::chat::conversations::create_conversation(
                &conn, "Test",
            )
            .unwrap();

        // Insert orphan message
        let msg = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conv.id.clone(),
            thread_id: None,
            ..ChatMessage::user(&conv.id, "Orphan")
        };

        conn.execute(
            "INSERT INTO chat_messages (id, conversation_id, thread_id, role, content, timestamp, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &msg.id,
                &msg.conversation_id,
                &msg.thread_id,
                msg.role.as_str(),
                &msg.content,
                &msg.timestamp.to_rfc3339(),
                "sent",
                None::<String>,
            ],
        ).unwrap();

        // Run migration twice
        migrate_orphan_messages_to_threads(&conn).unwrap();
        migrate_orphan_messages_to_threads(&conn).unwrap();

        // Should still have exactly one thread
        let threads = crate::infrastructure::sqlite::repositories::chat::threads::load_threads(
            &conn, &conv.id,
        )
        .unwrap();
        assert_eq!(threads.len(), 1);
    }

    #[test]
    fn test_migrate_orphan_messages_multiple_conversations() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::chat_schema::init_schema(&conn).unwrap();

        let conv1 =
            crate::infrastructure::sqlite::repositories::chat::conversations::create_conversation(
                &conn, "Conv1",
            )
            .unwrap();
        let conv2 =
            crate::infrastructure::sqlite::repositories::chat::conversations::create_conversation(
                &conn, "Conv2",
            )
            .unwrap();

        // Add orphans to both conversations
        for conv in [&conv1, &conv2] {
            let msg = ChatMessage {
                id: uuid::Uuid::new_v4().to_string(),
                conversation_id: conv.id.clone(),
                thread_id: None,
                ..ChatMessage::user(&conv.id, "Message")
            };
            conn.execute(
                "INSERT INTO chat_messages (id, conversation_id, thread_id, role, content, timestamp, status, metadata)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    &msg.id,
                    &msg.conversation_id,
                    &msg.thread_id,
                    msg.role.as_str(),
                    &msg.content,
                    &msg.timestamp.to_rfc3339(),
                    "sent",
                    None::<String>,
                ],
            ).unwrap();
        }

        migrate_orphan_messages_to_threads(&conn).unwrap();

        // Each conversation should have its own thread
        let threads1 = crate::infrastructure::sqlite::repositories::chat::threads::load_threads(
            &conn, &conv1.id,
        )
        .unwrap();
        let threads2 = crate::infrastructure::sqlite::repositories::chat::threads::load_threads(
            &conn, &conv2.id,
        )
        .unwrap();

        assert_eq!(threads1.len(), 1);
        assert_eq!(threads2.len(), 1);
        assert_ne!(threads1[0].id, threads2[0].id);
    }

    #[test]
    fn test_migrate_orphan_empty_content() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::chat_schema::init_schema(&conn).unwrap();

        let conv =
            crate::infrastructure::sqlite::repositories::chat::conversations::create_conversation(
                &conn, "Test",
            )
            .unwrap();

        // Insert message with empty content
        let msg = ChatMessage {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conv.id.clone(),
            thread_id: None,
            ..ChatMessage::user(&conv.id, "")
        };

        conn.execute(
            "INSERT INTO chat_messages (id, conversation_id, thread_id, role, content, timestamp, status, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                &msg.id,
                &msg.conversation_id,
                &msg.thread_id,
                msg.role.as_str(),
                &msg.content,
                &msg.timestamp.to_rfc3339(),
                "sent",
                None::<String>,
            ],
        ).unwrap();

        migrate_orphan_messages_to_threads(&conn).unwrap();

        let threads = crate::infrastructure::sqlite::repositories::chat::threads::load_threads(
            &conn, &conv.id,
        )
        .unwrap();
        assert_eq!(threads.len(), 1);
        // Should use default title
        assert_eq!(threads[0].title, "New Thread");
    }
}
