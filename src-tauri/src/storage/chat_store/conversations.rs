//! Conversation CRUD operations for the chat store.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use super::ChatStoreError;
use super::helpers::parse_datetime;
use crate::core::chat::entities::{Conversation, ConversationSettings};

/// Creates a new conversation.
pub fn create_conversation(
    conn: &Connection,
    title: &str,
) -> Result<Conversation, ChatStoreError> {
    let conversation = Conversation::new(title);
    let settings_json = serde_json::to_string(&conversation.settings)?;

    conn.execute(
        "INSERT INTO conversations (id, title, created_at, updated_at, settings)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            &conversation.id,
            &conversation.title,
            &conversation.created_at.to_rfc3339(),
            &conversation.updated_at.to_rfc3339(),
            &settings_json,
        ],
    )?;

    Ok(conversation)
}

/// Loads a conversation by ID.
pub fn load_conversation(conn: &Connection, id: &str) -> Result<Conversation, ChatStoreError> {
    let row = conn
        .query_row(
            "SELECT id, title, created_at, updated_at, settings
             FROM conversations WHERE id = ?1",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;

    let (id, title, created_at, updated_at, settings_json) =
        row.ok_or(ChatStoreError::NotFound("Conversation", id.to_string()))?;

    let settings: ConversationSettings = serde_json::from_str(&settings_json)?;

    Ok(Conversation {
        id,
        title,
        created_at: parse_datetime(&created_at),
        updated_at: parse_datetime(&updated_at),
        settings,
    })
}

/// Loads all conversations, sorted by update time (newest first).
pub fn load_all_conversations(
    conn: &Connection,
) -> Result<Vec<Conversation>, ChatStoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, created_at, updated_at, settings
         FROM conversations ORDER BY updated_at DESC",
    )?;

    let conversations = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .map(|result| {
            let (id, title, created_at, updated_at, settings_json) = result?;
            let settings: ConversationSettings = serde_json::from_str(&settings_json)?;
            Ok(Conversation {
                id,
                title,
                created_at: parse_datetime(&created_at),
                updated_at: parse_datetime(&updated_at),
                settings,
            })
        })
        .collect::<Result<Vec<_>, ChatStoreError>>()?;

    Ok(conversations)
}

/// Updates a conversation's title.
pub fn update_conversation_title(
    conn: &Connection,
    id: &str,
    title: &str,
) -> Result<Conversation, ChatStoreError> {
    let now = Utc::now();
    let rows_affected = conn.execute(
        "UPDATE conversations SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![title, now.to_rfc3339(), id],
    )?;

    if rows_affected == 0 {
        return Err(ChatStoreError::NotFound("Conversation", id.to_string()));
    }

    load_conversation(conn, id)
}

/// Deletes a conversation and all its threads and messages.
pub fn delete_conversation(conn: &Connection, id: &str) -> Result<(), ChatStoreError> {
    conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
    Ok(())
}

/// Updates a conversation's settings.
pub fn update_conversation_settings(
    conn: &Connection,
    id: &str,
    settings: &ConversationSettings,
) -> Result<Conversation, ChatStoreError> {
    let settings_json = serde_json::to_string(settings)?;
    let now = Utc::now();
    let rows_affected = conn.execute(
        "UPDATE conversations SET settings = ?1, updated_at = ?2 WHERE id = ?3",
        params![settings_json, now.to_rfc3339(), id],
    )?;

    if rows_affected == 0 {
        return Err(ChatStoreError::NotFound("Conversation", id.to_string()));
    }

    load_conversation(conn, id)
}

/// Updates a conversation's workspace root.
pub fn update_conversation_workspace(
    conn: &Connection,
    id: &str,
    workspace_root: &str,
) -> Result<Conversation, ChatStoreError> {
    let mut settings: ConversationSettings = conn
        .query_row(
            "SELECT settings FROM conversations WHERE id = ?1",
            params![id],
            |row| {
                let settings_json: String = row.get(0)?;
                serde_json::from_str(&settings_json).map_err(|_| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "Invalid settings JSON",
                        )),
                    )
                })
            },
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => {
                ChatStoreError::NotFound("Conversation", id.to_string())
            }
            _ => ChatStoreError::Sqlite(e),
        })?;

    settings.workspace_root = Some(workspace_root.to_string());
    let settings_json = serde_json::to_string(&settings)?;
    let now = Utc::now();

    conn.execute(
        "UPDATE conversations SET settings = ?1, updated_at = ?2 WHERE id = ?3",
        params![settings_json, now.to_rfc3339(), id],
    )?;

    load_conversation(conn, id)
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

    #[test]
    fn test_create_and_load_conversation() {
        let conn = make_test_store();
        let conv = create_conversation(&conn, "Test Conversation").unwrap();
        assert_eq!(conv.title, "Test Conversation");

        let loaded = load_conversation(&conn, &conv.id).unwrap();
        assert_eq!(loaded.id, conv.id);
        assert_eq!(loaded.title, conv.title);
    }

    #[test]
    fn test_load_all_conversations() {
        let conn = make_test_store();
        create_conversation(&conn, "First").unwrap();
        create_conversation(&conn, "Second").unwrap();

        let conversations = load_all_conversations(&conn).unwrap();
        assert_eq!(conversations.len(), 2);
        // Should be sorted by updated_at DESC
        assert_eq!(conversations[0].title, "Second");
        assert_eq!(conversations[1].title, "First");
    }

    #[test]
    fn test_update_conversation_title() {
        let conn = make_test_store();
        let conv = create_conversation(&conn, "Old Title").unwrap();
        let updated = update_conversation_title(&conn, &conv.id, "New Title").unwrap();
        assert_eq!(updated.title, "New Title");
    }

    #[test]
    fn test_delete_conversation() {
        let conn = make_test_store();
        let conv = create_conversation(&conn, "Test").unwrap();
        delete_conversation(&conn, &conv.id).unwrap();

        let result = load_conversation(&conn, &conv.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_update_conversation_settings() {
        let conn = make_test_store();
        let conv = create_conversation(&conn, "Test").unwrap();

        let new_settings = ConversationSettings {
            model: "custom-model".to_string(),
            temperature: 0.9,
            max_tokens: 8192,
            system_prompt: None,
            workspace_root: None,
        };

        let updated = update_conversation_settings(&conn, &conv.id, &new_settings).unwrap();
        assert_eq!(updated.settings.model, "custom-model");
        assert_eq!(updated.settings.temperature, 0.9);
    }

    #[test]
    fn test_update_conversation_workspace() {
        let conn = make_test_store();
        let conv = create_conversation(&conn, "Test").unwrap();
        let updated = update_conversation_workspace(&conn, &conv.id, "/path/to/workspace").unwrap();
        assert_eq!(updated.settings.workspace_root, Some("/path/to/workspace".to_string()));
    }

    #[test]
    fn test_update_conversation_not_found() {
        let conn = make_test_store();
        let result = update_conversation_title(&conn, "nonexistent", "Title");
        assert!(matches!(result, Err(ChatStoreError::NotFound(_, _))));
    }
}
