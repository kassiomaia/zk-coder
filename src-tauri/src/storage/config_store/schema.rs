//! Database schema initialization for the config store.

use rusqlite::Connection;

use super::ConfigStoreError;

/// Initializes the database schema for the config store.
pub fn init_schema(conn: &Connection) -> Result<(), ConfigStoreError> {
    conn.execute_batch(
        r#"
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
        "#,
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
    }
}
