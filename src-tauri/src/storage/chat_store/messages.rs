//! Message CRUD operations for the chat store.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use super::ChatStoreError;
use super::helpers::{parse_datetime, parse_status, status_to_str};
use crate::core::chat::entities::{
    ChatMessage, MessageFeedback, MessageMetadata, MessageRole, MessageStatus,
};

/// Adds a message to a conversation (optionally to a thread).
pub fn add_message(conn: &Connection, message: &ChatMessage) -> Result<(), ChatStoreError> {
    let metadata_json = message
        .metadata
        .as_ref()
        .map(|m| serde_json::to_string(m))
        .transpose()?;

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
            status_to_str(&message.status),
            metadata_json,
        ],
    )?;

    // Update conversation's updated_at
    let now = Utc::now();
    conn.execute(
        "UPDATE conversations SET updated_at = ?1 WHERE id = ?2",
        params![now.to_rfc3339(), &message.conversation_id],
    )?;

    // Update thread's message count and updated_at if applicable
    if let Some(ref thread_id) = message.thread_id {
        conn.execute(
            "UPDATE threads SET message_count = message_count + 1, updated_at = ?1 WHERE id = ?2",
            params![now.to_rfc3339(), thread_id],
        )?;
    }

    Ok(())
}

/// Loads messages for a conversation.
pub fn load_messages(
    conn: &Connection,
    conversation_id: &str,
) -> Result<Vec<ChatMessage>, ChatStoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, thread_id, role, content, timestamp, status, metadata
         FROM chat_messages WHERE conversation_id = ?1 ORDER BY timestamp ASC",
    )?;

    let messages = stmt
        .query_map(params![conversation_id], |row| {
            let metadata_json: Option<String> = row.get(7)?;
            let metadata = metadata_json.and_then(|j| serde_json::from_str(&j).ok());

            Ok(ChatMessage {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                thread_id: row.get(2)?,
                role: MessageRole::from_str(&row.get::<_, String>(3)?),
                content: row.get(4)?,
                timestamp: parse_datetime(&row.get::<_, String>(5)?),
                status: parse_status(&row.get::<_, String>(6)?),
                metadata,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(messages)
}

/// Loads messages for a specific thread.
pub fn load_thread_messages(
    conn: &Connection,
    thread_id: &str,
) -> Result<Vec<ChatMessage>, ChatStoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, thread_id, role, content, timestamp, status, metadata
         FROM chat_messages WHERE thread_id = ?1 ORDER BY timestamp ASC",
    )?;

    let messages = stmt
        .query_map(params![thread_id], |row| {
            let metadata_json: Option<String> = row.get(7)?;
            let metadata = metadata_json.and_then(|j| serde_json::from_str(&j).ok());

            Ok(ChatMessage {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                thread_id: row.get(2)?,
                role: MessageRole::from_str(&row.get::<_, String>(3)?),
                content: row.get(4)?,
                timestamp: parse_datetime(&row.get::<_, String>(5)?),
                status: parse_status(&row.get::<_, String>(6)?),
                metadata,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(messages)
}

/// Deletes a message.
pub fn delete_message(conn: &Connection, id: &str) -> Result<(), ChatStoreError> {
    conn.execute("DELETE FROM chat_messages WHERE id = ?1", params![id])?;
    Ok(())
}

/// Updates a message's status.
pub fn update_message_status(
    conn: &Connection,
    id: &str,
    status: MessageStatus,
) -> Result<(), ChatStoreError> {
    conn.execute(
        "UPDATE chat_messages SET status = ?1 WHERE id = ?2",
        params![status_to_str(&status), id],
    )?;
    Ok(())
}

/// Updates a message's feedback (like/dislike).
///
/// Pass `None` to clear the feedback. Only one feedback can be active at a time.
pub fn update_message_feedback(
    conn: &Connection,
    id: &str,
    feedback: Option<MessageFeedback>,
) -> Result<(), ChatStoreError> {
    let metadata_json: Option<String> = match conn
        .query_row(
            "SELECT metadata FROM chat_messages WHERE id = ?1",
            params![id],
            |row| row.get::<_, Option<String>>(0),
        )
    {
        Ok(Some(meta)) => Some(meta),
        Ok(None) => None,
        Err(rusqlite::Error::QueryReturnedNoRows) => None,
        Err(e) => return Err(ChatStoreError::Sqlite(e)),
    };

    let mut metadata: MessageMetadata = metadata_json
        .as_deref()
        .and_then(|j| serde_json::from_str(j).ok())
        .unwrap_or_default();

    metadata.feedback = feedback;

    let new_metadata_json = serde_json::to_string(&metadata)?;

    let rows_affected = conn.execute(
        "UPDATE chat_messages SET metadata = ?1 WHERE id = ?2",
        params![new_metadata_json, id],
    )?;

    if rows_affected == 0 {
        return Err(ChatStoreError::NotFound("Message", id.to_string()));
    }

    Ok(())
}

/// Loads messages with feedback for a specific thread.
///
/// Returns messages that have feedback (like or dislike) set, ordered by timestamp descending.
/// Limited to the specified count.
pub fn load_thread_feedback(
    conn: &Connection,
    thread_id: &str,
    limit: usize,
) -> Result<Vec<(ChatMessage, MessageFeedback)>, ChatStoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, conversation_id, thread_id, role, content, timestamp, status, metadata
         FROM chat_messages
         WHERE thread_id = ?1 AND metadata IS NOT NULL
         ORDER BY timestamp DESC
         LIMIT ?2",
    )?;

    let messages_with_feedback = stmt
        .query_map(params![thread_id, limit as i64], |row| {
            let metadata_json: Option<String> = row.get(7)?;
            let metadata: Option<MessageMetadata> =
                metadata_json.and_then(|j| serde_json::from_str(&j).ok());

            let message = ChatMessage {
                id: row.get(0)?,
                conversation_id: row.get(1)?,
                thread_id: row.get(2)?,
                role: MessageRole::from_str(&row.get::<_, String>(3)?),
                content: row.get(4)?,
                timestamp: parse_datetime(&row.get::<_, String>(5)?),
                status: parse_status(&row.get::<_, String>(6)?),
                metadata: metadata.clone(),
            };

            let feedback = metadata.and_then(|m| m.feedback);

            Ok((message, feedback))
        })?
        .filter_map(|result| {
            let (message, feedback) = result.ok()?;
            feedback.map(|f| (message, f))
        })
        .collect::<Vec<_>>();

    Ok(messages_with_feedback)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::chat_store::schema;

    fn make_test_store() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::init_schema(&conn).unwrap();
        conn
    }

    fn make_test_conversation(conn: &Connection) -> String {
        super::super::conversations::create_conversation(conn, "Test Conv")
            .unwrap()
            .id
    }

    #[test]
    fn test_add_and_load_messages() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let msg = ChatMessage::user(&conv_id, "Hello world");
        add_message(&conn, &msg).unwrap();

        let messages = load_messages(&conn, &conv_id).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello world");
        assert_eq!(messages[0].role, MessageRole::User);
    }

    #[test]
    fn test_load_thread_messages() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let thread = super::super::threads::create_thread(&conn, &conv_id, "Test Thread").unwrap();

        let msg = ChatMessage {
            thread_id: Some(thread.id.clone()),
            ..ChatMessage::user(&conv_id, "Thread message")
        };
        add_message(&conn, &msg).unwrap();

        let messages = load_thread_messages(&conn, &thread.id).unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Thread message");
    }

    #[test]
    fn test_delete_message() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let msg = ChatMessage::user(&conv_id, "Test");
        add_message(&conn, &msg).unwrap();
        delete_message(&conn, &msg.id).unwrap();

        let messages = load_messages(&conn, &conv_id).unwrap();
        assert_eq!(messages.len(), 0);
    }

    #[test]
    fn test_update_message_status() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let mut msg = ChatMessage::user(&conv_id, "Test");
        msg.status = MessageStatus::Sending;
        add_message(&conn, &msg).unwrap();

        update_message_status(&conn, &msg.id, MessageStatus::Sent).unwrap();
        let messages = load_messages(&conn, &conv_id).unwrap();
        assert_eq!(messages[0].status, MessageStatus::Sent);
    }

    #[test]
    fn test_update_message_feedback() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let msg = ChatMessage::user(&conv_id, "Test");
        add_message(&conn, &msg).unwrap();

        update_message_feedback(&conn, &msg.id, Some(MessageFeedback::Like)).unwrap();

        let messages = load_messages(&conn, &conv_id).unwrap();
        assert_eq!(
            messages[0].metadata.as_ref().unwrap().feedback,
            Some(MessageFeedback::Like)
        );

        // Clear feedback
        update_message_feedback(&conn, &msg.id, None).unwrap();
        let messages = load_messages(&conn, &conv_id).unwrap();
        assert_eq!(messages[0].metadata.as_ref().unwrap().feedback, None);
    }

    #[test]
    fn test_update_message_feedback_not_found() {
        let conn = make_test_store();
        let result = update_message_feedback(&conn, "nonexistent", Some(MessageFeedback::Like));
        assert!(matches!(result, Err(ChatStoreError::NotFound(_, _))));
    }

    #[test]
    fn test_load_thread_feedback() {
        let conn = make_test_store();
        let conv_id = make_test_conversation(&conn);

        let thread = super::super::threads::create_thread(&conn, &conv_id, "Test Thread").unwrap();

        let mut msg1 = ChatMessage {
            thread_id: Some(thread.id.clone()),
            ..ChatMessage::user(&conv_id, "Message 1")
        };
        msg1.metadata = Some(MessageMetadata {
            feedback: Some(MessageFeedback::Like),
            ..Default::default()
        });
        add_message(&conn, &msg1).unwrap();

        let mut msg2 = ChatMessage {
            thread_id: Some(thread.id.clone()),
            ..ChatMessage::user(&conv_id, "Message 2")
        };
        msg2.metadata = Some(MessageMetadata {
            feedback: Some(MessageFeedback::Dislike),
            ..Default::default()
        });
        add_message(&conn, &msg2).unwrap();

        // Message without feedback should not be included
        let msg3 = ChatMessage {
            thread_id: Some(thread.id.clone()),
            ..ChatMessage::user(&conv_id, "Message 3")
        };
        add_message(&conn, &msg3).unwrap();

        let feedback = load_thread_feedback(&conn, &thread.id, 10).unwrap();
        assert_eq!(feedback.len(), 2);
    }
}
