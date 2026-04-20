//! Centralized `SQLite` schema and migration entrypoints.

pub mod agent;
pub mod chat;
pub mod chat_schema;
pub mod config;
pub mod config_schema;
pub mod nlp_context;

/// Initialises every domain's schema and runs all data migrations on a single connection.
///
/// Safe to call on every startup — all `CREATE TABLE` statements use `IF NOT EXISTS`
/// and data migrations are idempotent.
///
/// # Errors
///
/// Returns the first error encountered during schema or migration execution.
pub fn init_all(
    conn: &rusqlite::Connection,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    config_schema::init_schema(conn)?;
    chat_schema::init_schema(conn)?;
    nlp_context::init_schema(conn)?;
    agent::init_schema(conn)?;
    config::migrate_api_type_constraint(conn)?;
    config::migrate_models_source(conn)?;
    config::normalize_base_urls(conn)?;
    chat::migrate_orphan_messages_to_threads(conn)?;
    Ok(())
}
