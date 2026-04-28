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

        -- Per-workspace domain policy for the web search tool.
        --
        -- `workspace_path` is the canonicalized absolute path of the workspace
        -- root (empty string = global fallback for sessions without one).
        -- `allowlist` / `denylist` are JSON arrays of domain patterns
        -- (exact host, leading-dot for subdomains, or '*' for any).
        CREATE TABLE IF NOT EXISTS workspace_search_policy (
            workspace_path TEXT PRIMARY KEY,
            allowlist      TEXT NOT NULL,
            denylist       TEXT NOT NULL,
            updated_at     TEXT NOT NULL
        );

        -- Web search result cache.
        --
        -- Key is SHA-256 of the normalized request + workspace path so
        -- per-workspace partitioning is baked into the primary key and a
        -- policy change in one workspace cannot bleed into another. The
        -- `results` column is JSON-serialized Vec<WebSearchResult>. The
        -- index on `expires_at` lets the startup sweep purge stale rows
        -- in one pass.
        CREATE TABLE IF NOT EXISTS web_search_cache (
            cache_key       TEXT PRIMARY KEY,
            query           TEXT NOT NULL,
            num_results     INTEGER NOT NULL,
            language        TEXT,
            site            TEXT,
            workspace_path  TEXT NOT NULL,
            provider        TEXT NOT NULL,
            results         TEXT NOT NULL,
            created_at      TEXT NOT NULL,
            expires_at      TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_web_search_cache_expires
        ON web_search_cache(expires_at);

        -- PDF theme selection (T3.3)
        CREATE TABLE IF NOT EXISTS pdf_theme_selection (
            workspace_path  TEXT PRIMARY KEY,
            theme_name      TEXT NOT NULL,
            updated_at      TEXT NOT NULL
        );

        -- UI preferences (single-row global app preferences).
        --
        -- Holds the user-selected UI theme. Value is one of:
        --   'light'  | 'dark' | 'system'
        -- 'system' means follow the OS preferred-color-scheme. The frontend
        -- still computes the effective theme on boot — this row only
        -- persists the user's intent so it survives app restarts.
        CREATE TABLE IF NOT EXISTS ui_preferences (
            id          INTEGER PRIMARY KEY CHECK (id = 1),
            theme       TEXT NOT NULL CHECK (theme IN ('light', 'dark', 'system')),
            updated_at  TEXT NOT NULL
        );

        -- Prompt templates.
        --
        -- Each row maps a slash-command slug (the token after / in chat,
        -- e.g. refactor) to a prompt body with a {args} placeholder. The
        -- backend expands /{slug} <rest> into the body with {args} replaced
        -- by <rest> before the user message reaches the LLM.
        --
        -- is_builtin = 1 rows are seeded on startup via INSERT OR IGNORE,
        -- so deleting a builtin row and re-running seed restores it exactly
        -- to the factory default (used by the reset action in the UI).
        CREATE TABLE IF NOT EXISTS prompt_templates (
            id           TEXT PRIMARY KEY,
            slug         TEXT NOT NULL UNIQUE,
            title        TEXT NOT NULL,
            description  TEXT NOT NULL,
            icon         TEXT,
            category     TEXT NOT NULL,
            content      TEXT NOT NULL,
            is_builtin   INTEGER NOT NULL DEFAULT 0,
            is_enabled   INTEGER NOT NULL DEFAULT 1,
            sort_order   INTEGER NOT NULL DEFAULT 0,
            created_at   TEXT NOT NULL,
            updated_at   TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_prompt_templates_slug
        ON prompt_templates(slug);
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
        assert!(tables.contains(&"workspace_search_policy".to_string()));
        assert!(tables.contains(&"web_search_cache".to_string()));
        assert!(tables.contains(&"prompt_templates".to_string()));
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
