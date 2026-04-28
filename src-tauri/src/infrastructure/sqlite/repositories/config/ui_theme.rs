//! SQLite implementation of the UI theme preference repository.
//!
//! The UI theme is a single-row global preference rather than per-workspace,
//! so this module operates on a fixed primary key (`id = 1`).

use rusqlite::Connection;

use super::ConfigRepositoryError;

/// Returns the persisted UI theme, or `None` when no preference has been
/// recorded yet (the frontend then falls back to the system preference).
pub fn get_ui_theme(conn: &Connection) -> Result<Option<String>, ConfigRepositoryError> {
    let mut stmt = conn.prepare("SELECT theme FROM ui_preferences WHERE id = 1")?;
    let result: Option<String> = stmt.query_row([], |row| row.get(0)).ok();
    Ok(result)
}

/// Persists the UI theme. `theme` must be one of `light`, `dark`, `system`;
/// SQLite enforces the constraint at the schema level.
pub fn set_ui_theme(conn: &Connection, theme: &str) -> Result<(), ConfigRepositoryError> {
    conn.execute(
        "INSERT INTO ui_preferences (id, theme, updated_at) VALUES (1, ?1, datetime('now'))
         ON CONFLICT(id) DO UPDATE SET theme = excluded.theme, updated_at = excluded.updated_at",
        rusqlite::params![theme],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::sqlite::migrations::config_schema::init_schema;

    fn fresh_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        init_schema(&conn).expect("init schema");
        conn
    }

    #[test]
    fn get_returns_none_when_unset() {
        let conn = fresh_conn();
        assert!(get_ui_theme(&conn).expect("query").is_none());
    }

    #[test]
    fn set_then_get_round_trips() {
        let conn = fresh_conn();
        set_ui_theme(&conn, "light").expect("set");
        assert_eq!(get_ui_theme(&conn).expect("get"), Some("light".to_string()));
    }

    #[test]
    fn set_overwrites_previous_value() {
        let conn = fresh_conn();
        set_ui_theme(&conn, "dark").expect("set dark");
        set_ui_theme(&conn, "system").expect("set system");
        assert_eq!(
            get_ui_theme(&conn).expect("get"),
            Some("system".to_string())
        );
    }

    #[test]
    fn invalid_theme_is_rejected_by_check_constraint() {
        let conn = fresh_conn();
        assert!(set_ui_theme(&conn, "neon").is_err());
    }
}
