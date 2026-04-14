//! Database schema initialization for the config store.

use rusqlite::Connection;

use crate::infrastructure::sqlite::repositories::config::ConfigRepositoryError;

/// Initializes the database schema for the config store.
pub fn init_schema(conn: &Connection) -> Result<(), ConfigRepositoryError> {
    conn.execute_batch(
        r"
        -- Providers table
        CREATE TABLE IF NOT EXISTS providers (
            id             TEXT PRIMARY KEY,
            name           TEXT NOT NULL,
            icon           TEXT NOT NULL,
            is_custom      BOOLEAN NOT NULL DEFAULT FALSE,
            api_type       TEXT NOT NULL CHECK(api_type IN ('openai', 'anthropic', 'google')),
            base_url       TEXT,
            is_configured  BOOLEAN NOT NULL DEFAULT FALSE,
            models_source  TEXT NOT NULL DEFAULT 'auto' CHECK(models_source IN ('auto', 'manual', 'hybrid')),
            created_at     TEXT NOT NULL,
            updated_at     TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_providers_name
        ON providers(name COLLATE NOCASE);

        -- Models table
        CREATE TABLE IF NOT EXISTS models (
            id                  TEXT PRIMARY KEY,
            provider_id         TEXT NOT NULL,
            name                TEXT NOT NULL,
            supports_analysis   BOOLEAN NOT NULL DEFAULT TRUE,
            supports_code       BOOLEAN NOT NULL DEFAULT TRUE,
            supports_vision     BOOLEAN NOT NULL DEFAULT FALSE,
            supports_audio      BOOLEAN NOT NULL DEFAULT FALSE,
            supports_tips       BOOLEAN NOT NULL DEFAULT TRUE,
            supports_review     BOOLEAN NOT NULL DEFAULT TRUE,
            last_fetched        TEXT NOT NULL,
            FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_models_provider
        ON models(provider_id);

        -- Operation defaults table
        CREATE TABLE IF NOT EXISTS operation_defaults (
            operation   TEXT PRIMARY KEY CHECK(operation IN ('analysis', 'code', 'vision', 'audio', 'tips', 'review')),
            model_id    TEXT REFERENCES models(id) ON DELETE SET NULL
        );

        -- API Keys table
        CREATE TABLE IF NOT EXISTS api_keys (
            provider_id TEXT PRIMARY KEY,
            api_key     TEXT NOT NULL,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL,
            FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
        );

        -- Manual models table (user-configured models for providers)
        CREATE TABLE IF NOT EXISTS manual_models (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            provider_id     TEXT NOT NULL,
            name            TEXT NOT NULL,
            supports_analysis   BOOLEAN NOT NULL DEFAULT TRUE,
            supports_code       BOOLEAN NOT NULL DEFAULT TRUE,
            supports_vision     BOOLEAN NOT NULL DEFAULT FALSE,
            supports_audio      BOOLEAN NOT NULL DEFAULT FALSE,
            supports_tips       BOOLEAN NOT NULL DEFAULT TRUE,
            supports_review     BOOLEAN NOT NULL DEFAULT TRUE,
            created_at      TEXT NOT NULL,
            FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE,
            UNIQUE(provider_id, name)
        );

        CREATE INDEX IF NOT EXISTS idx_manual_models_provider
        ON manual_models(provider_id);

        -- Tool credentials table. Each row stores the opaque JSON credential
        -- blob owned by one tool_id. Shape is tool-specific (see the
        -- per-tool resolver for the format).
        CREATE TABLE IF NOT EXISTS tool_credentials (
            tool_id     TEXT PRIMARY KEY,
            credentials TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        );
        ",
    )?;
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
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"api_keys".to_string()));
        assert!(tables.contains(&"manual_models".to_string()));
        assert!(tables.contains(&"models".to_string()));
        assert!(tables.contains(&"operation_defaults".to_string()));
        assert!(tables.contains(&"providers".to_string()));
        assert!(tables.contains(&"tool_credentials".to_string()));
    }

    #[test]
    fn test_init_schema_is_idempotent_on_existing_db() {
        // Simulate a fresh install followed by an app restart: running
        // `init_schema` twice on the same connection must be safe, since
        // existing users will re-run it every launch.
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        init_schema(&conn).unwrap();
    }

    #[test]
    fn test_tool_credentials_table_has_expected_columns() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(tool_credentials)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(columns.contains(&"tool_id".to_string()));
        assert!(columns.contains(&"credentials".to_string()));
        assert!(columns.contains(&"updated_at".to_string()));
    }
}
