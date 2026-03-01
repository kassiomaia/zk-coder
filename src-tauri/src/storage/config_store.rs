//! SQLite storage for LLM provider configuration.
//!
//! This module provides persistence for providers, models, and operation defaults
//! using SQLite.

use std::sync::Mutex;

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

use crate::nlp::config::{
    ApiType, Model, ModelCapabilities, OperationDefault, OperationType, Provider,
};

/// SQLite-based configuration store.
pub struct ConfigStore {
    /// Database connection (protected by mutex for thread safety)
    conn: Mutex<Connection>,
}

/// Errors from the config store.
#[derive(Debug, Error)]
pub enum ConfigStoreError {
    /// SQLite error
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Not found error
    #[error("{0} not found: {1}")]
    NotFound(&'static str, String),
    /// Lock error
    #[error("Lock error: {0}")]
    LockError(String),
}

impl ConfigStore {
    /// Creates a new config store at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or initialized.
    pub fn new(db_path: &str) -> Result<Self, ConfigStoreError> {
        let conn = Connection::open(db_path)?;
        Self::init_schema(&conn)?;
        Self::migrate_api_type_constraint(&conn)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.seed_builtin_providers()?;
        Ok(store)
    }

    /// Creates an in-memory config store (for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be initialized.
    pub fn in_memory() -> Result<Self, ConfigStoreError> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;
        Self::migrate_api_type_constraint(&conn)?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.seed_builtin_providers()?;
        Ok(store)
    }

    fn init_schema(conn: &Connection) -> Result<(), ConfigStoreError> {
        conn.execute_batch(
            r#"
            -- Providers table
            CREATE TABLE IF NOT EXISTS providers (
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
            "#,
        )?;
        Ok(())
    }

    /// Migrates the api_type constraint to include 'google' for existing databases
    /// and fixes is_custom for built-in providers.
    fn migrate_api_type_constraint(conn: &Connection) -> Result<(), ConfigStoreError> {
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

                    -- Copy data from old table, fixing built-in providers' api_type and is_custom
                    INSERT INTO providers
                    SELECT id, name, icon,
                           CASE WHEN id IN ('anthropic', 'openai', 'google') THEN FALSE ELSE is_custom END,
                           CASE
                               WHEN id = 'anthropic' THEN 'anthropic'
                               WHEN id = 'google' THEN 'google'
                               ELSE api_type
                           END,
                           base_url, is_configured, created_at, updated_at
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

    fn seed_builtin_providers(&self) -> Result<(), ConfigStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))?;

        // Use INSERT OR IGNORE to be idempotent
        let builtin_providers = [
            ("anthropic", "Anthropic", "AC", "anthropic"),
            ("openai", "OpenAI", "OA", "openai"),
            ("google", "Google", "GC", "google"),
        ];

        let now = Utc::now().to_rfc3339();
        for (id, name, icon, api_type) in builtin_providers {
            conn.execute(
                "INSERT OR IGNORE INTO providers (id, name, icon, is_custom, api_type, is_configured, created_at, updated_at)
                 VALUES (?1, ?2, ?3, FALSE, ?4, FALSE, ?5, ?5)",
                params![id, name, icon, api_type, now],
            )?;
        }

        Ok(())
    }

    // =========================================================================
    // Provider Operations
    // =========================================================================

    /// Gets all providers.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_providers(&self) -> Result<Vec<Provider>, ConfigStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, icon, is_custom, api_type, base_url, is_configured, created_at, updated_at
             FROM providers ORDER BY name",
        )?;

        let providers = stmt
            .query_map([], |row| {
                let api_type_str: String = row.get(4)?;
                let api_type = ApiType::from_str(&api_type_str);
                let created_at_str: String = row.get(7)?;
                let updated_at_str: String = row.get(8)?;

                Ok(Provider {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    icon: row.get(2)?,
                    is_custom: row.get(3)?,
                    api_type,
                    base_url: row.get(5)?,
                    is_configured: row.get(6)?,
                    created_at: parse_datetime(&created_at_str),
                    updated_at: parse_datetime(&updated_at_str),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(providers)
    }

    /// Gets a provider by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_provider(&self, id: &str) -> Result<Option<Provider>, ConfigStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, icon, is_custom, api_type, base_url, is_configured, created_at, updated_at
             FROM providers WHERE id = ?1",
        )?;

        let provider = stmt
            .query_row(params![id], |row| {
                let api_type_str: String = row.get(4)?;
                let api_type = ApiType::from_str(&api_type_str);
                let created_at_str: String = row.get(7)?;
                let updated_at_str: String = row.get(8)?;

                Ok(Provider {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    icon: row.get(2)?,
                    is_custom: row.get(3)?,
                    api_type,
                    base_url: row.get(5)?,
                    is_configured: row.get(6)?,
                    created_at: parse_datetime(&created_at_str),
                    updated_at: parse_datetime(&updated_at_str),
                })
            })
            .optional()?;

        Ok(provider)
    }

    /// Saves a provider (create or update).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn save_provider(&self, provider: &Provider) -> Result<(), ConfigStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))?;

        conn.execute(
            "INSERT INTO providers (id, name, icon, is_custom, api_type, base_url, is_configured, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                icon = excluded.icon,
                api_type = excluded.api_type,
                base_url = excluded.base_url,
                is_configured = excluded.is_configured,
                updated_at = excluded.updated_at",
            params![
                &provider.id,
                &provider.name,
                &provider.icon,
                provider.is_custom,
                provider.api_type.as_str(),
                &provider.base_url,
                provider.is_configured,
                &provider.created_at.to_rfc3339(),
                &provider.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(())
    }

    /// Deletes a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn delete_provider(&self, id: &str) -> Result<(), ConfigStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))?;

        let rows_affected = conn.execute(
            "DELETE FROM providers WHERE id = ?1 AND is_custom = TRUE",
            params![id],
        )?;

        if rows_affected == 0 {
            return Err(ConfigStoreError::NotFound(
                "Provider",
                id.to_string(),
            ));
        }

        Ok(())
    }

    /// Sets the configured status of a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn set_provider_configured(
        &self,
        id: &str,
        configured: bool,
    ) -> Result<(), ConfigStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))?;

        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE providers SET is_configured = ?1, updated_at = ?2 WHERE id = ?3",
            params![configured, now, id],
        )?;

        Ok(())
    }

    // =========================================================================
    // Model Operations
    // =========================================================================

    /// Gets all models for a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_models(&self, provider_id: &str) -> Result<Vec<Model>, ConfigStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, provider_id, name, supports_analysis, supports_code,
                    supports_vision, supports_audio, supports_tips, supports_review, last_fetched
             FROM models WHERE provider_id = ?1 ORDER BY name",
        )?;

        let models = stmt
            .query_map(params![provider_id], |row| {
                let last_fetched_str: String = row.get(9)?;
                Ok(Model {
                    id: row.get(0)?,
                    provider_id: row.get(1)?,
                    name: row.get(2)?,
                    capabilities: ModelCapabilities {
                        analysis: row.get(3)?,
                        code: row.get(4)?,
                        vision: row.get(5)?,
                        audio: row.get(6)?,
                        tips: row.get(7)?,
                        review: row.get(8)?,
                    },
                    last_fetched: parse_datetime(&last_fetched_str),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(models)
    }

    /// Saves models for a provider (replaces existing).
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn save_models(
        &self,
        provider_id: &str,
        models: &[Model],
    ) -> Result<(), ConfigStoreError> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))?;

        let tx = conn.transaction()?;
        tx.execute("DELETE FROM models WHERE provider_id = ?1", params![provider_id])?;

        for model in models {
            tx.execute(
                "INSERT INTO models (id, provider_id, name, supports_analysis, supports_code,
                 supports_vision, supports_audio, supports_tips, supports_review, last_fetched)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    &model.id,
                    &model.provider_id,
                    &model.name,
                    model.capabilities.analysis,
                    model.capabilities.code,
                    model.capabilities.vision,
                    model.capabilities.audio,
                    model.capabilities.tips,
                    model.capabilities.review,
                    &model.last_fetched.to_rfc3339(),
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Gets a model by ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_model(&self, id: &str) -> Result<Option<Model>, ConfigStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))?;

        let mut stmt = conn.prepare(
            "SELECT id, provider_id, name, supports_analysis, supports_code,
                    supports_vision, supports_audio, supports_tips, supports_review, last_fetched
             FROM models WHERE id = ?1",
        )?;

        let model = stmt
            .query_row(params![id], |row| {
                let last_fetched_str: String = row.get(9)?;
                Ok(Model {
                    id: row.get(0)?,
                    provider_id: row.get(1)?,
                    name: row.get(2)?,
                    capabilities: ModelCapabilities {
                        analysis: row.get(3)?,
                        code: row.get(4)?,
                        vision: row.get(5)?,
                        audio: row.get(6)?,
                        tips: row.get(7)?,
                        review: row.get(8)?,
                    },
                    last_fetched: parse_datetime(&last_fetched_str),
                })
            })
            .optional()?;

        Ok(model)
    }

    // =========================================================================
    // Operation Default Operations
    // =========================================================================

    /// Gets all operation defaults.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn get_operation_defaults(&self) -> Result<Vec<OperationDefault>, ConfigStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))?;

        let mut stmt = conn.prepare("SELECT operation, model_id FROM operation_defaults")?;

        let defaults = stmt
            .query_map([], |row| {
                let operation_str: String = row.get(0)?;
                let operation = OperationType::from_str(&operation_str)
                    .unwrap_or(OperationType::Analysis);

                Ok(OperationDefault {
                    operation,
                    model_id: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(defaults)
    }

    /// Sets the default model for an operation.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn set_operation_default(
        &self,
        operation: OperationType,
        model_id: &str,
    ) -> Result<(), ConfigStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))?;

        conn.execute(
            "INSERT OR REPLACE INTO operation_defaults (operation, model_id) VALUES (?1, ?2)",
            params![operation.as_str(), model_id],
        )?;

        Ok(())
    }

    /// Clears the default model for an operation.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn clear_operation_default(&self, operation: OperationType) -> Result<(), ConfigStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))?;

        conn.execute(
            "DELETE FROM operation_defaults WHERE operation = ?1",
            params![operation.as_str()],
        )?;

        Ok(())
    }

    /// Clears all operation defaults for models belonging to a provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails.
    pub fn clear_defaults_for_provider(&self, provider_id: &str) -> Result<(), ConfigStoreError> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| ConfigStoreError::LockError("Failed to acquire lock".to_string()))?;

        conn.execute(
            "DELETE FROM operation_defaults WHERE model_id IN (SELECT id FROM models WHERE provider_id = ?1)",
            params![provider_id],
        )?;

        Ok(())
    }
}

/// Parses a datetime string from the database.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_creation() {
        let store = ConfigStore::in_memory().expect("Failed to create store");
        let providers = store.get_providers().expect("Failed to get providers");
        assert_eq!(providers.len(), 3); // Built-in providers
    }

    #[test]
    fn test_save_and_get_provider() {
        let store = ConfigStore::in_memory().expect("Failed to create store");
        let provider =
            Provider::custom("Test Provider", "TP", ApiType::OpenAI, Some("https://api.test.com".to_string()));
        store.save_provider(&provider).expect("Failed to save provider");

        let retrieved = store.get_provider(&provider.id).expect("Failed to get provider");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, "Test Provider");
        assert!(retrieved.is_custom);
    }

    #[test]
    fn test_delete_provider() {
        let store = ConfigStore::in_memory().expect("Failed to create store");
        let provider = Provider::custom("ToDelete", "TD", ApiType::OpenAI, None);
        store.save_provider(&provider).expect("Failed to save provider");

        store.delete_provider(&provider.id).expect("Failed to delete provider");

        let retrieved = store.get_provider(&provider.id).expect("Failed to get provider");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_save_and_get_models() {
        let store = ConfigStore::in_memory().expect("Failed to create store");
        let models = vec![
            Model::new("openai", "gpt-4", ModelCapabilities::default()),
            Model::new("openai", "gpt-3.5-turbo", ModelCapabilities::default()),
        ];

        store.save_models("openai", &models).expect("Failed to save models");

        let retrieved = store.get_models("openai").expect("Failed to get models");
        assert_eq!(retrieved.len(), 2);
    }

    #[test]
    fn test_operation_defaults() {
        let store = ConfigStore::in_memory().expect("Failed to create store");

        // First save a model (required for foreign key constraint)
        let model = Model::new("openai", "gpt-4", ModelCapabilities::default());
        store.save_models("openai", &[model]).expect("Failed to save model");

        // Now set the operation default
        store
            .set_operation_default(OperationType::Code, "openai-gpt-4")
            .expect("Failed to set default");

        let defaults = store.get_operation_defaults().expect("Failed to get defaults");
        assert_eq!(defaults.len(), 1);
        assert_eq!(defaults[0].operation, OperationType::Code);
        assert_eq!(defaults[0].model_id, "openai-gpt-4");
    }
}
