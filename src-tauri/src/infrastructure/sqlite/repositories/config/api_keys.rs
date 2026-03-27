//! API key operations for the config store.

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};

use super::ConfigRepositoryError;

/// Stores an API key for a provider.
pub fn store_api_key(
    conn: &Connection,
    provider_id: &str,
    api_key: &str,
) -> Result<(), ConfigRepositoryError> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO api_keys (provider_id, api_key, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?3)
         ON CONFLICT(provider_id) DO UPDATE SET
            api_key = excluded.api_key,
            updated_at = excluded.updated_at",
        params![provider_id, api_key, now],
    )?;

    Ok(())
}

/// Retrieves an API key for a provider.
pub fn retrieve_api_key(
    conn: &Connection,
    provider_id: &str,
) -> Result<String, ConfigRepositoryError> {
    let api_key: String = conn
        .query_row(
            "SELECT api_key FROM api_keys WHERE provider_id = ?1",
            params![provider_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| {
            ConfigRepositoryError::NotFound("API key for provider", provider_id.to_string())
        })?;

    Ok(api_key)
}

/// Deletes an API key for a provider.
pub fn delete_api_key(conn: &Connection, provider_id: &str) -> Result<(), ConfigRepositoryError> {
    conn.execute(
        "DELETE FROM api_keys WHERE provider_id = ?1",
        params![provider_id],
    )?;

    Ok(())
}

/// Checks if an API key exists for a provider.
pub fn has_api_key(conn: &Connection, provider_id: &str) -> Result<bool, ConfigRepositoryError> {
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM api_keys WHERE provider_id = ?1)",
        params![provider_id],
        |row| row.get(0),
    )?;

    Ok(exists)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_store() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::config_schema::init_schema(&conn).unwrap();
        super::super::providers::seed_builtin_providers(&conn).unwrap();
        conn
    }

    #[test]
    fn test_store_and_retrieve_api_key() {
        let conn = make_test_store();

        // Persist API key for built-in provider
        store_api_key(&conn, "openai", "sk-test-key-12345").unwrap();

        // Retrieve it
        let key = retrieve_api_key(&conn, "openai").unwrap();
        assert_eq!(key, "sk-test-key-12345");
    }

    #[test]
    fn test_retrieve_nonexistent_api_key() {
        let conn = make_test_store();
        let result = retrieve_api_key(&conn, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_api_key() {
        let conn = make_test_store();

        // Should not exist initially
        let exists = has_api_key(&conn, "openai").unwrap();
        assert!(!exists);

        // Persist the key
        store_api_key(&conn, "openai", "sk-test-key").unwrap();

        // Should exist now
        let exists = has_api_key(&conn, "openai").unwrap();
        assert!(exists);
    }

    #[test]
    fn test_delete_api_key() {
        let conn = make_test_store();

        // Persist and delete
        store_api_key(&conn, "openai", "sk-test-key").unwrap();
        delete_api_key(&conn, "openai").unwrap();

        // Should not exist anymore
        let exists = has_api_key(&conn, "openai").unwrap();
        assert!(!exists);
    }

    #[test]
    fn test_update_api_key() {
        let conn = make_test_store();

        // Persist initial key
        store_api_key(&conn, "openai", "sk-old-key").unwrap();

        // Update with new key
        store_api_key(&conn, "openai", "sk-new-key").unwrap();

        // Should have new key
        let key = retrieve_api_key(&conn, "openai").unwrap();
        assert_eq!(key, "sk-new-key");
    }
}
