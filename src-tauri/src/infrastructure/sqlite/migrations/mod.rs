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
/// Foreign key enforcement is disabled for the duration of this function so that
/// schema-rebuilding migrations (e.g. `migrate_api_type_constraint`) can rename/drop
/// tables without triggering cascade deletes. It is re-enabled before returning.
///
/// # Errors
///
/// Returns the first error encountered during schema or migration execution.
pub fn init_all(
    conn: &rusqlite::Connection,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    conn.execute_batch("PRAGMA foreign_keys=OFF;")?;
    config_schema::init_schema(conn)?;
    chat_schema::init_schema(conn)?;
    nlp_context::init_schema(conn)?;
    agent::init_schema(conn)?;
    config::migrate_api_type_constraint(conn)?;
    config::migrate_models_source(conn)?;
    config::normalize_base_urls(conn)?;
    chat::migrate_orphan_messages_to_threads(conn)?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    type SchemaObject = (String, String, Option<String>);

    fn schema_snapshot(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<SchemaObject>> {
        let mut stmt = conn.prepare(
            "SELECT type, name, sql
             FROM sqlite_master
             WHERE type IN ('table', 'index', 'view', 'trigger')
               AND name NOT LIKE 'sqlite_%'
             ORDER BY type, name",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;

        rows.collect()
    }

    fn init_domains_individually(
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

    #[test]
    fn init_all_creates_the_same_schema_as_the_explicit_bootstrap_sequence() {
        let conn_from_init_all = rusqlite::Connection::open_in_memory().unwrap();
        init_all(&conn_from_init_all).unwrap();

        let conn_from_explicit_steps = rusqlite::Connection::open_in_memory().unwrap();
        init_domains_individually(&conn_from_explicit_steps).unwrap();

        assert_eq!(
            schema_snapshot(&conn_from_init_all).unwrap(),
            schema_snapshot(&conn_from_explicit_steps).unwrap()
        );
    }

    #[test]
    fn init_all_is_idempotent_across_repeated_runs() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();

        init_all(&conn).unwrap();
        let schema_after_first_run = schema_snapshot(&conn).unwrap();

        init_all(&conn).unwrap();
        let schema_after_second_run = schema_snapshot(&conn).unwrap();

        assert_eq!(schema_after_first_run, schema_after_second_run);
    }
}
