// Skills system storage (engineering-todo-v2.md 1.9.1).
// Skills are SKILL.md files with YAML frontmatter + markdown body.
// The IDE manages them; agents receive the skill catalog via ACP and
// execute skills on demand.

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::chat_db::get_db_path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbSkill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub scope: String, // "global" | "project" | "nested"
    pub directory_path: String,
    pub file_path: String,
    pub frontmatter: serde_json::Value,
    pub body: String,
    pub is_builtin: bool,
    pub is_active: bool,
    pub when_to_use: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub fn init_skills_tables(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS skills (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT NOT NULL DEFAULT '',
            scope TEXT NOT NULL DEFAULT 'project',
            directory_path TEXT NOT NULL DEFAULT '',
            file_path TEXT NOT NULL DEFAULT '',
            frontmatter_json TEXT NOT NULL DEFAULT '{}',
            body TEXT NOT NULL DEFAULT '',
            is_builtin INTEGER NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 1,
            when_to_use TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}

fn row_to_skill(row: &rusqlite::Row) -> SqlResult<DbSkill> {
    let fm_json: String = row.get(6)?;
    let frontmatter = serde_json::from_str(&fm_json).unwrap_or(serde_json::Value::Object(Default::default()));
    Ok(DbSkill {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        scope: row.get(3)?,
        directory_path: row.get(4)?,
        file_path: row.get(5)?,
        frontmatter,
        body: row.get(7)?,
        is_builtin: row.get(8)?,
        is_active: row.get(9)?,
        when_to_use: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

const SELECT_COLS: &str = "id, name, description, scope, directory_path, file_path, frontmatter_json, body, is_builtin, is_active, when_to_use, created_at, updated_at";

fn fetch_skill(conn: &Connection, id: &str) -> Result<DbSkill, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT {SELECT_COLS} FROM skills WHERE id = ?1"))
        .map_err(|e| e.to_string())?;
    stmt.query_row(params![id], row_to_skill).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn skill_list(scope: Option<String>) -> Result<Vec<DbSkill>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let (q, use_param) = match scope {
        Some(_) => (format!("SELECT {SELECT_COLS} FROM skills WHERE scope=?1 ORDER BY name ASC"), true),
        None => (format!("SELECT {SELECT_COLS} FROM skills ORDER BY scope ASC, name ASC"), false),
    };
    let mut stmt = conn.prepare(&q).map_err(|e| e.to_string())?;
    if use_param {
        // Re-do with param
        let scope_val = scope.unwrap_or_default();
        return Ok(stmt.query_map(params![scope_val], row_to_skill).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect());
    }
    Ok(stmt.query_map([], row_to_skill).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn skill_get(id: String) -> Result<DbSkill, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    fetch_skill(&conn, &id)
}

#[tauri::command]
pub fn skill_create(
    name: String,
    description: String,
    scope: String,
    directory_path: String,
    file_path: String,
    body: Option<String>,
    when_to_use: Option<String>,
    is_builtin: Option<bool>,
) -> Result<DbSkill, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();
    let body_text = body.unwrap_or_default();
    let builtin = is_builtin.unwrap_or(false);
    conn.execute(
        "INSERT INTO skills (id,name,description,scope,directory_path,file_path,frontmatter_json,body,is_builtin,is_active,when_to_use,created_at,updated_at) VALUES (?1,?2,?3,?4,?5,?6,'{}',?7,?8,1,?9,?10,?11)",
        params![id, name, description, scope, directory_path, file_path, body_text, builtin, when_to_use, now, now],
    ).map_err(|e| e.to_string())?;
    fetch_skill(&conn, &id)
}

#[tauri::command]
pub fn skill_update(
    id: String,
    name: Option<String>,
    description: Option<String>,
    body: Option<String>,
    when_to_use: Option<String>,
    is_active: Option<bool>,
) -> Result<DbSkill, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    if let Some(v) = name { conn.execute("UPDATE skills SET name=?1,updated_at=?2 WHERE id=?3", params![v,now,id]).map_err(|e|e.to_string())?; }
    if let Some(v) = description { conn.execute("UPDATE skills SET description=?1,updated_at=?2 WHERE id=?3", params![v,now,id]).map_err(|e|e.to_string())?; }
    if let Some(v) = body { conn.execute("UPDATE skills SET body=?1,updated_at=?2 WHERE id=?3", params![v,now,id]).map_err(|e|e.to_string())?; }
    if let Some(v) = when_to_use { let s = if v.is_empty() { None } else { Some(v) }; conn.execute("UPDATE skills SET when_to_use=?1,updated_at=?2 WHERE id=?3", params![s,now,id]).map_err(|e|e.to_string())?; }
    if let Some(v) = is_active { conn.execute("UPDATE skills SET is_active=?1,updated_at=?2 WHERE id=?3", params![v,now,id]).map_err(|e|e.to_string())?; }
    fetch_skill(&conn, &id)
}

#[tauri::command]
pub fn skill_delete(id: String) -> Result<(), String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM skills WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

/// Scan well-known skill directories and return what was found (without
/// inserting into SQLite - insertion is done per-item by the UI after preview).
/// Paths checked: ~/.claude/skills/, .claude/skills/ (cwd), .agents/skills/
#[tauri::command]
pub fn skill_scan_directories(workspace_path: Option<String>) -> Result<Vec<serde_json::Value>, String> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut candidates: Vec<std::path::PathBuf> = vec![
        home.join(".claude").join("skills"),
        home.join(".agents").join("skills"),
    ];
    if let Some(ws) = workspace_path {
        let p = std::path::PathBuf::from(ws);
        candidates.push(p.join(".claude").join("skills"));
        candidates.push(p.join(".agents").join("skills"));
        candidates.push(p.join(".cursor").join("skills"));
    }
    let mut found = Vec::new();
    for dir in candidates {
        if !dir.is_dir() { continue; }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let skill_md = path.join("SKILL.md");
                    if skill_md.exists() {
                        if let Ok(content) = std::fs::read_to_string(&skill_md) {
                            // Very basic frontmatter extraction (everything between --- delimiters)
                            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                            found.push(serde_json::json!({
                                "name": name,
                                "file_path": skill_md.to_string_lossy(),
                                "directory_path": path.to_string_lossy(),
                                "preview": &content[..content.len().min(200)],
                            }));
                        }
                    }
                }
            }
        }
    }
    Ok(found)
}

/// Catalog-level listing for agents: just name + description for all active
/// skills, suitable for injection into ACP session/new or session/prompt metadata.
#[tauri::command]
pub fn skill_catalog() -> Result<Vec<serde_json::Value>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT name, description, when_to_use FROM skills WHERE is_active=1 ORDER BY name ASC").map_err(|e| e.to_string())?;
    let rows: Vec<serde_json::Value> = stmt.query_map([], |row| {
        Ok(serde_json::json!({
            "name": row.get::<_, String>(0)?,
            "description": row.get::<_, String>(1)?,
            "when_to_use": row.get::<_, Option<String>>(2)?,
        }))
    }).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
    Ok(rows)
}
