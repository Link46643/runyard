// Notes and TODOs storage (engineering-todo-v2.md 1.11.1 and 1.11.2).
// Per-project markdown notes and checkbox TODO items.

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::chat_db::get_db_path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbNote {
    pub id: String,
    pub workspace_path: String,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbTodo {
    pub id: String,
    pub workspace_path: String,
    pub text: String,
    pub is_done: bool,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

pub fn init_notes_tables(conn: &Connection) -> SqlResult<()> {
    // One note per workspace (auto-upserted).
    conn.execute(
        "CREATE TABLE IF NOT EXISTS notes (
            id TEXT PRIMARY KEY,
            workspace_path TEXT NOT NULL UNIQUE,
            content TEXT NOT NULL DEFAULT '',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS todos (
            id TEXT PRIMARY KEY,
            workspace_path TEXT NOT NULL,
            text TEXT NOT NULL,
            is_done INTEGER NOT NULL DEFAULT 0,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}

// ── Notes ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn note_load(workspace_path: String) -> Result<DbNote, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    // Return existing or a blank note (auto-create row so save is always an upsert).
    match conn.query_row(
        "SELECT id, workspace_path, content, created_at, updated_at FROM notes WHERE workspace_path=?1",
        params![workspace_path],
        |row| Ok(DbNote {
            id: row.get(0)?,
            workspace_path: row.get(1)?,
            content: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        }),
    ) {
        Ok(note) => Ok(note),
        Err(_) => {
            let id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO notes (id, workspace_path, content, created_at, updated_at) VALUES (?1,?2,'',?3,?4)",
                params![id, workspace_path, now, now],
            ).map_err(|e| e.to_string())?;
            Ok(DbNote { id, workspace_path, content: String::new(), created_at: now, updated_at: now })
        }
    }
}

#[tauri::command]
pub fn note_save(workspace_path: String, content: String) -> Result<DbNote, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO notes (id, workspace_path, content, created_at, updated_at) VALUES (?1,?2,?3,?4,?5)
         ON CONFLICT(workspace_path) DO UPDATE SET content=excluded.content, updated_at=excluded.updated_at",
        params![id, workspace_path, content, now, now],
    ).map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, workspace_path, content, created_at, updated_at FROM notes WHERE workspace_path=?1",
        params![workspace_path],
        |row| Ok(DbNote {
            id: row.get(0)?,
            workspace_path: row.get(1)?,
            content: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        }),
    ).map_err(|e| e.to_string())
}

// ── TODOs ───────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn todo_list(workspace_path: String) -> Result<Vec<DbTodo>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id,workspace_path,text,is_done,sort_order,created_at,updated_at FROM todos WHERE workspace_path=?1 ORDER BY sort_order ASC, created_at ASC")
        .map_err(|e| e.to_string())?;
    Ok(stmt.query_map(params![workspace_path], |row| {
        Ok(DbTodo {
            id: row.get(0)?,
            workspace_path: row.get(1)?,
            text: row.get(2)?,
            is_done: row.get(3)?,
            sort_order: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn todo_create(workspace_path: String, text: String) -> Result<DbTodo, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();
    // Place at end
    let max_order: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order),0) FROM todos WHERE workspace_path=?1",
        params![workspace_path],
        |r| r.get(0),
    ).unwrap_or(0);
    conn.execute(
        "INSERT INTO todos (id,workspace_path,text,is_done,sort_order,created_at,updated_at) VALUES (?1,?2,?3,0,?4,?5,?6)",
        params![id, workspace_path, text, max_order + 1, now, now],
    ).map_err(|e| e.to_string())?;
    Ok(DbTodo { id, workspace_path, text, is_done: false, sort_order: max_order + 1, created_at: now, updated_at: now })
}

#[tauri::command]
pub fn todo_update(id: String, text: Option<String>, is_done: Option<bool>) -> Result<DbTodo, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    if let Some(v) = text { conn.execute("UPDATE todos SET text=?1,updated_at=?2 WHERE id=?3", params![v,now,id]).map_err(|e|e.to_string())?; }
    if let Some(v) = is_done { conn.execute("UPDATE todos SET is_done=?1,updated_at=?2 WHERE id=?3", params![v,now,id]).map_err(|e|e.to_string())?; }
    conn.query_row(
        "SELECT id,workspace_path,text,is_done,sort_order,created_at,updated_at FROM todos WHERE id=?1",
        params![id],
        |row| Ok(DbTodo {
            id: row.get(0)?, workspace_path: row.get(1)?, text: row.get(2)?,
            is_done: row.get(3)?, sort_order: row.get(4)?, created_at: row.get(5)?, updated_at: row.get(6)?,
        }),
    ).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn todo_delete(id: String) -> Result<(), String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM todos WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn todo_reorder(workspace_path: String, ordered_ids: Vec<String>) -> Result<(), String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    for (i, id) in ordered_ids.iter().enumerate() {
        conn.execute(
            "UPDATE todos SET sort_order=?1,updated_at=?2 WHERE id=?3 AND workspace_path=?4",
            params![i as i64, now, id, workspace_path],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}
