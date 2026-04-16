//! SQLite implementation of the PDF theme selection repository (T3.3).

use rusqlite::Connection;

use super::ConfigRepositoryError;

/// Returns the selected theme for `workspace_path`, or `None`.
pub fn get_pdf_theme(
    conn: &Connection,
    workspace_path: &str,
) -> Result<Option<String>, ConfigRepositoryError> {
    let mut stmt =
        conn.prepare("SELECT theme_name FROM pdf_theme_selection WHERE workspace_path = ?1")?;
    let result: Option<String> = stmt.query_row([workspace_path], |row| row.get(0)).ok();
    Ok(result)
}

/// Persists the selected theme for `workspace_path`.
pub fn set_pdf_theme(
    conn: &Connection,
    workspace_path: &str,
    theme_name: &str,
) -> Result<(), ConfigRepositoryError> {
    conn.execute(
        "INSERT OR REPLACE INTO pdf_theme_selection (workspace_path, theme_name, updated_at) VALUES (?1, ?2, datetime('now'))",
        rusqlite::params![workspace_path, theme_name],
    )?;
    Ok(())
}
