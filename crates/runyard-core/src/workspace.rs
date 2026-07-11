// Workspace management commands: open folder, get/set current workspace path,
// get recent workspaces. These are the foundation for "Runyard can open any
// folder the user desires" rather than defaulting to a fixed path.

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};

use crate::chat_db::get_db_path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecentWorkspace {
    pub path: String,
    pub name: String,
    pub last_opened_at: i64,
}

pub fn init_workspace_tables(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS recent_workspaces (
            path TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            last_opened_at INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}

/// Open a folder via the OS native file dialog. Returns the selected path, or
/// None if the user cancelled. Uses tauri-plugin-dialog on the frontend side;
/// this command is the Rust counterpart that records the selection.
#[tauri::command]
pub fn workspace_open(path: String) -> Result<String, String> {
    // Record as a recent workspace.
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let name = std::path::Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "INSERT INTO recent_workspaces (path, name, last_opened_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(path) DO UPDATE SET name = excluded.name, last_opened_at = excluded.last_opened_at",
        params![path, name, now],
    ).map_err(|e| e.to_string())?;
    Ok(path)
}

#[tauri::command]
pub fn workspace_list_recent() -> Result<Vec<RecentWorkspace>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT path, name, last_opened_at FROM recent_workspaces ORDER BY last_opened_at DESC LIMIT 20")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(RecentWorkspace {
            path: row.get(0)?,
            name: row.get(1)?,
            last_opened_at: row.get(2)?,
        })
    }).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn workspace_remove_recent(path: String) -> Result<(), String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM recent_workspaces WHERE path = ?1", params![path])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Resolve a workspace-relative path to an absolute path given the current
/// workspace root. Used by the IDE to resolve "@file" mentions and relative
/// paths in agent requests.
#[tauri::command]
pub fn workspace_resolve_path(workspace: String, relative: String) -> Result<String, String> {
    let base = std::path::Path::new(&workspace);
    let resolved = base.join(&relative);
    Ok(resolved.to_string_lossy().to_string())
}
