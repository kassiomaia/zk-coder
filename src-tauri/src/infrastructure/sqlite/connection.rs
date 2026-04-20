//! Shared SQLite connection factory.

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

/// Opens a file-backed SQLite connection shared via `Arc<Mutex<Connection>>`.
///
/// Enables WAL journal mode and foreign key enforcement on the connection.
///
/// # Errors
///
/// Returns an error if the file cannot be opened or pragmas fail.
pub fn open_shared(db_path: &str) -> Result<Arc<Mutex<Connection>>, rusqlite::Error> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    Ok(Arc::new(Mutex::new(conn)))
}

/// Opens an in-memory SQLite connection shared via `Arc<Mutex<Connection>>`.
///
/// # Errors
///
/// Returns an error if the connection cannot be created.
pub fn open_in_memory_shared() -> Result<Arc<Mutex<Connection>>, rusqlite::Error> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    Ok(Arc::new(Mutex::new(conn)))
}
