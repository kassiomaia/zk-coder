//! Database migrations for the config store.

use rusqlite::Connection;

use crate::infrastructure::sqlite::repositories::config::ConfigRepositoryError;

/// Migrates the api_type constraint to include 'google' for existing databases
/// and fixes is_custom for built-in providers.
pub fn migrate_api_type_constraint(conn: &Connection) -> Result<(), ConfigRepositoryError> {
    // First, fix built-in providers that may have incorrect is_custom or api_type values
    // This handles cases where providers were created through the UI before proper seeding
    conn.execute_batch(
        r#"
        -- Fix Anthropic provider
        UPDATE providers
        SET is_custom = FALSE, api_type = 'anthropic'
        WHERE id = 'anthropic';

        -- Fix OpenAI provider
        UPDATE providers
        SET is_custom = FALSE, api_type = 'openai'
        WHERE id = 'openai';

        -- Fix Google provider
        UPDATE providers
        SET is_custom = FALSE, api_type = 'google'
        WHERE id = 'google';
        "#,
    )?;

    // Check if migration is needed by trying to insert a google provider
    // If the constraint doesn't allow 'google', we need to recreate the table
    let needs_migration: bool = conn
        .query_row(
            "SELECT 0 FROM providers WHERE api_type = 'google' LIMIT 1",
            [],
            |_| Ok(false),
        )
        .unwrap_or(true);

    if needs_migration {
        // Check if the constraint actually prevents google (table exists but constraint is old)
        let constraint_allows_google: bool = conn
            .execute(
                "INSERT OR ROLLBACK INTO providers (id, name, icon, is_custom, api_type, is_configured, created_at, updated_at)
                 VALUES ('__migration_test__', 'Test', 'T', FALSE, 'google', FALSE, datetime('now'), datetime('now'))",
                [],
            )
            .is_ok();

        if constraint_allows_google {
            // Clean up test row
            let _ = conn.execute("DELETE FROM providers WHERE id = '__migration_test__'", []);

            // Fix any Google provider that was inserted with wrong api_type
            // This can happen if the constraint was updated but data wasn't
            conn.execute(
                "UPDATE providers SET api_type = 'google', is_custom = FALSE WHERE id = 'google' AND (api_type != 'google' OR is_custom != FALSE)",
                [],
            )?;
        } else {
            // Need to migrate - recreate the table with the new constraint
            // Note: We must drop dependent tables first to avoid breaking foreign keys
            conn.execute_batch(
                r#"
                -- Save models data before dropping
                CREATE TABLE IF NOT EXISTS models_backup AS SELECT * FROM models;

                -- Drop dependent tables (they will be recreated with correct foreign keys)
                DROP TABLE IF EXISTS operation_defaults;
                DROP TABLE IF EXISTS models;

                -- Start migration: recreate providers table with updated constraint
                ALTER TABLE providers RENAME TO providers_old;

                -- Create new table with correct constraint
                CREATE TABLE providers (
                    id            TEXT PRIMARY KEY,
                    name          TEXT NOT NULL,
                    icon          TEXT NOT NULL,
                    is_custom     BOOLEAN NOT NULL DEFAULT FALSE,
                    api_type      TEXT NOT NULL CHECK(api_type IN ('openai', 'anthropic', 'google')),
                    base_url      TEXT,
                    is_configured BOOLEAN NOT NULL DEFAULT FALSE,
                    created_at    TEXT NOT NULL,
                    updated_at    TEXT NOT NULL
                );

                -- Copy data from old table, fixing built-in providers' api_type, is_custom, and is_configured
                INSERT INTO providers
                SELECT id, name, icon,
                       CASE WHEN id IN ('anthropic', 'openai', 'google') THEN FALSE ELSE is_custom END,
                       CASE
                           WHEN id = 'anthropic' THEN 'anthropic'
                           WHEN id = 'google' THEN 'google'
                           ELSE api_type
                       END,
                       base_url,
                       CASE WHEN id IN ('anthropic', 'openai', 'google') THEN FALSE ELSE is_configured END,
                       created_at, updated_at
                FROM providers_old;

                -- Drop old table
                DROP TABLE providers_old;

                -- Recreate providers index
                CREATE INDEX IF NOT EXISTS idx_providers_name
                ON providers(name COLLATE NOCASE);

                -- Recreate models table with correct foreign key
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

                -- Restore models data if it existed
                INSERT OR IGNORE INTO models SELECT * FROM models_backup;

                -- Recreate models index
                CREATE INDEX IF NOT EXISTS idx_models_provider
                ON models(provider_id);

                -- Recreate operation_defaults table
                CREATE TABLE IF NOT EXISTS operation_defaults (
                    operation   TEXT PRIMARY KEY CHECK(operation IN ('analysis', 'code', 'vision', 'audio', 'tips', 'review')),
                    model_id    TEXT REFERENCES models(id) ON DELETE SET NULL
                );

                -- Cleanup backup table
                DROP TABLE IF EXISTS models_backup;
                "#,
            )?;
        }
    }

    Ok(())
}

/// Migrates the providers table to add the models_source column for existing databases.
pub fn migrate_models_source(conn: &Connection) -> Result<(), ConfigRepositoryError> {
    // Check if migration is needed by checking if the column exists
    let has_column: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('providers') WHERE name = 'models_source'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if has_column == 0 {
        // Column doesn't exist, add it
        conn.execute(
            "ALTER TABLE providers ADD COLUMN models_source TEXT NOT NULL DEFAULT 'auto' CHECK(models_source IN ('auto', 'manual', 'hybrid'))",
            [],
        )?;
        log::info!("Added models_source column to providers table");
    }

    // Check if manual_models table exists, create if not
    let has_table: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='manual_models'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if has_table == 0 {
        conn.execute_batch(
            r#"
            CREATE TABLE manual_models (
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
        log::info!("Created manual_models table");
    }

    Ok(())
}

/// Normalizes base URLs in the providers table to remove endpoint paths.
///
/// This fixes a bug where base URLs were incorrectly stored with endpoint paths
/// like `/chat/completions` included.
pub fn normalize_base_urls(conn: &Connection) -> Result<(), ConfigRepositoryError> {
    // Fix OpenAI base URLs
    conn.execute(
        r#"UPDATE providers
           SET base_url = REPLACE(REPLACE(REPLACE(base_url,
               '/chat/completions', ''),
               '/completions', ''),
               '/models', '')
           WHERE base_url IS NOT NULL
           AND base_url LIKE '%/chat/completions%'
           OR base_url LIKE '%/completions%'
           OR base_url LIKE '%/models'"#,
        [],
    )?;

    // For built-in providers, set base_url to NULL (use defaults)
    conn.execute(
        "UPDATE providers SET base_url = NULL WHERE id IN ('openai', 'anthropic', 'google') AND is_custom = FALSE",
        [],
    )?;

    log::info!("Normalized base URLs in providers table");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrate_models_source_adds_column() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::config_schema::init_schema(&conn).unwrap();

        // Remove the models_source column to simulate old database
        conn.execute_batch(
            r#"
            DROP INDEX IF EXISTS idx_providers_name;
            DROP TABLE IF EXISTS providers;
            CREATE TABLE providers (
                id             TEXT PRIMARY KEY,
                name           TEXT NOT NULL,
                icon           TEXT NOT NULL,
                is_custom      BOOLEAN NOT NULL DEFAULT FALSE,
                api_type       TEXT NOT NULL CHECK(api_type IN ('openai', 'anthropic', 'google')),
                base_url       TEXT,
                is_configured  BOOLEAN NOT NULL DEFAULT FALSE,
                created_at     TEXT NOT NULL,
                updated_at     TEXT NOT NULL
            );
            CREATE INDEX idx_providers_name ON providers(name COLLATE NOCASE);
            "#,
        )
        .unwrap();

        // Run migration
        migrate_models_source(&conn).unwrap();

        // Check column exists
        let has_column: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('providers') WHERE name = 'models_source'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(has_column, 1);

        // Check manual_models table was created
        let has_table: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='manual_models'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(has_table, 1);
    }

    #[test]
    fn test_normalize_base_urls() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::config_schema::init_schema(&conn).unwrap();

        // Insert provider with bad base URL
        conn.execute(
            "INSERT INTO providers (id, name, icon, is_custom, api_type, is_configured, models_source, created_at, updated_at)
             VALUES ('test', 'Test', 'T', TRUE, 'openai', FALSE, 'auto', datetime('now'), datetime('now'))",
            [],
        ).unwrap();
        conn.execute(
            "UPDATE providers SET base_url = 'https://api.example.com/chat/completions' WHERE id = 'test'",
            [],
        ).unwrap();

        // Run migration
        normalize_base_urls(&conn).unwrap();

        // Check URL was normalized
        let base_url: Option<String> = conn
            .query_row(
                "SELECT base_url FROM providers WHERE id = 'test'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(base_url, Some("https://api.example.com".to_string()));
    }

    #[test]
    fn test_migrate_api_type_constraint_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::config_schema::init_schema(&conn).unwrap();
        crate::infrastructure::sqlite::repositories::config::providers::seed_builtin_providers(
            &conn,
        )
        .unwrap();

        // Run migration twice - should not fail
        migrate_api_type_constraint(&conn).unwrap();
        migrate_api_type_constraint(&conn).unwrap();

        // Verify built-in providers exist
        let providers: Vec<String> = conn
            .prepare("SELECT id FROM providers WHERE id IN ('anthropic', 'openai', 'google')")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(providers.len(), 3);
    }

    #[test]
    fn test_migrate_models_source_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::config_schema::init_schema(&conn).unwrap();

        // Run migration twice
        migrate_models_source(&conn).unwrap();
        migrate_models_source(&conn).unwrap();

        // Verify column exists
        let has_column: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('providers') WHERE name = 'models_source'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(has_column, 1);
    }

    #[test]
    fn test_normalize_base_urls_handles_builtin_providers() {
        let conn = Connection::open_in_memory().unwrap();
        crate::infrastructure::sqlite::migrations::config_schema::init_schema(&conn).unwrap();
        crate::infrastructure::sqlite::repositories::config::providers::seed_builtin_providers(
            &conn,
        )
        .unwrap();

        // Set base URL for built-in provider (should be cleared)
        conn.execute(
            "UPDATE providers SET base_url = 'https://api.openai.com/chat/completions' WHERE id = 'openai'",
            [],
        ).unwrap();

        normalize_base_urls(&conn).unwrap();

        // Built-in providers should have NULL base_url
        let base_url: Option<String> = conn
            .query_row(
                "SELECT base_url FROM providers WHERE id = 'openai'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(base_url, None);
    }

    #[test]
    fn test_migrate_models_source_creates_manual_models_table() {
        let conn = Connection::open_in_memory().unwrap();

        // Initialize schema without models_source column (simulating old database)
        conn.execute_batch(
            r#"
            CREATE TABLE providers (
                id             TEXT PRIMARY KEY,
                name           TEXT NOT NULL,
                icon           TEXT NOT NULL,
                is_custom      BOOLEAN NOT NULL DEFAULT FALSE,
                api_type       TEXT NOT NULL CHECK(api_type IN ('openai', 'anthropic', 'google')),
                base_url       TEXT,
                is_configured  BOOLEAN NOT NULL DEFAULT FALSE,
                created_at     TEXT NOT NULL,
                updated_at     TEXT NOT NULL
            );

            -- Insert a test provider for the foreign key
            INSERT INTO providers (id, name, icon, is_custom, api_type, is_configured, created_at, updated_at)
            VALUES ('openai', 'OpenAI', 'O', FALSE, 'openai', FALSE, datetime('now'), datetime('now'));
            "#,
        ).unwrap();

        // Run migration
        migrate_models_source(&conn).unwrap();

        // Verify manual_models table exists
        let table_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='manual_models'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_exists, 1);

        // Verify we can insert into manual_models
        conn.execute(
            "INSERT INTO manual_models (provider_id, name, supports_analysis, supports_code,
             supports_vision, supports_audio, supports_tips, supports_review, created_at)
             VALUES ('openai', 'custom-model', TRUE, TRUE, FALSE, FALSE, TRUE, TRUE, datetime('now'))",
            [],
        ).unwrap();
    }
}
