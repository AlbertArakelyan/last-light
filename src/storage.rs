//! Local storage shared by mini apps: a per-user data folder holding one
//! SQLite database. Each app creates its own tables in it.

use std::path::PathBuf;

use rusqlite::Connection;

const DB_FILE: &str = "last-light.db";

/// Per-user app data folder, e.g. `%APPDATA%\Last Light` on Windows.
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Last Light")
}

pub fn open_db() -> Result<Connection, String> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("can't create {}: {e}", dir.display()))?;
    let conn = Connection::open(dir.join(DB_FILE)).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| e.to_string())?;
    Ok(conn)
}
