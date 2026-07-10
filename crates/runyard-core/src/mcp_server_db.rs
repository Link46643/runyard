// MCP server configuration storage (engineering-todo-v2.md 1.8.1).
// The IDE stores MCP server configs here and passes them to agents via ACP
// session/new. The agent (not the IDE) actually connects to MCP servers.
// Same pattern as acp_agent_db.rs.

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::chat_db::get_db_path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbMcpEnvVar {
    pub key: String,
    pub value: String,
    pub is_secret: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbMcpAuth {
    pub kind: String, // "none" | "bearer" | "api_key"
    pub token: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbMcpServer {
    pub id: String,
    pub name: String,
    pub transport: String, // "stdio" | "http" | "websocket"
    pub command: Option<String>,
    pub url: Option<String>,
    pub env_vars: Vec<DbMcpEnvVar>,
    pub auth: Option<DbMcpAuth>,
    pub is_global: bool,
    pub is_active: bool,
    pub project_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

pub fn init_mcp_server_tables(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS mcp_servers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            transport TEXT NOT NULL DEFAULT 'stdio',
            command TEXT,
            url TEXT,
            env_vars_json TEXT NOT NULL DEFAULT '[]',
            auth_json TEXT,
            is_global INTEGER NOT NULL DEFAULT 1,
            is_active INTEGER NOT NULL DEFAULT 1,
            project_id TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(())
}

fn row_to_server(row: &rusqlite::Row) -> SqlResult<DbMcpServer> {
    let env_vars_json: String = row.get(5)?;
    let env_vars: Vec<DbMcpEnvVar> = serde_json::from_str(&env_vars_json).unwrap_or_default();
    let auth_json: Option<String> = row.get(6)?;
    let auth = auth_json.and_then(|s| serde_json::from_str(&s).ok());
    Ok(DbMcpServer {
        id: row.get(0)?,
        name: row.get(1)?,
        transport: row.get(2)?,
        command: row.get(3)?,
        url: row.get(4)?,
        env_vars,
        auth,
        is_global: row.get(7)?,
        is_active: row.get(8)?,
        project_id: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

const SELECT_COLS: &str = "id, name, transport, command, url, env_vars_json, auth_json, is_global, is_active, project_id, created_at, updated_at";

fn fetch_server(conn: &Connection, id: &str) -> Result<DbMcpServer, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT {SELECT_COLS} FROM mcp_servers WHERE id = ?1"))
        .map_err(|e| e.to_string())?;
    stmt.query_row(params![id], row_to_server).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn mcp_server_list(project_id: Option<String>) -> Result<Vec<DbMcpServer>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let (query, rows_result) = match project_id {
        Some(pid) => {
            let q = format!("SELECT {SELECT_COLS} FROM mcp_servers WHERE is_global = 1 OR project_id = ?1 ORDER BY name ASC");
            let mut stmt = conn.prepare(&q).map_err(|e| e.to_string())?;
            let rows: Vec<DbMcpServer> = stmt.query_map(params![pid], row_to_server)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok()).collect();
            return Ok(rows);
        }
        None => {
            let q = format!("SELECT {SELECT_COLS} FROM mcp_servers ORDER BY is_global DESC, name ASC");
            (q, conn.prepare(&format!("SELECT {SELECT_COLS} FROM mcp_servers ORDER BY is_global DESC, name ASC")).map_err(|e| e.to_string())?)
        }
    };
    let _ = query;
    rows_result.query_map([], row_to_server)
        .map_err(|e| e.to_string())
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn mcp_server_get(id: String) -> Result<DbMcpServer, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    fetch_server(&conn, &id)
}

#[tauri::command]
pub fn mcp_server_create(
    name: String,
    transport: String,
    command: Option<String>,
    url: Option<String>,
    env_vars: Option<Vec<DbMcpEnvVar>>,
    auth: Option<DbMcpAuth>,
    is_global: Option<bool>,
    project_id: Option<String>,
) -> Result<DbMcpServer, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();
    let env_vars_json = serde_json::to_string(&env_vars.unwrap_or_default()).map_err(|e| e.to_string())?;
    let auth_json = auth.as_ref().map(|a| serde_json::to_string(a).unwrap_or_default());
    let global = is_global.unwrap_or(true);
    conn.execute(
        "INSERT INTO mcp_servers (id, name, transport, command, url, env_vars_json, auth_json, is_global, is_active, project_id, created_at, updated_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,1,?9,?10,?11)",
        params![id, name, transport, command, url, env_vars_json, auth_json, global, project_id, now, now],
    ).map_err(|e| e.to_string())?;
    fetch_server(&conn, &id)
}

#[tauri::command]
pub fn mcp_server_update(
    id: String,
    name: Option<String>,
    transport: Option<String>,
    command: Option<String>,
    url: Option<String>,
    env_vars: Option<Vec<DbMcpEnvVar>>,
    auth: Option<Value>,
    is_global: Option<bool>,
    project_id: Option<String>,
) -> Result<DbMcpServer, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    if let Some(v) = name { conn.execute("UPDATE mcp_servers SET name=?1,updated_at=?2 WHERE id=?3", params![v,now,id]).map_err(|e|e.to_string())?; }
    if let Some(v) = transport { conn.execute("UPDATE mcp_servers SET transport=?1,updated_at=?2 WHERE id=?3", params![v,now,id]).map_err(|e|e.to_string())?; }
    if let Some(v) = command { let s = if v.is_empty() { None } else { Some(v) }; conn.execute("UPDATE mcp_servers SET command=?1,updated_at=?2 WHERE id=?3", params![s,now,id]).map_err(|e|e.to_string())?; }
    if let Some(v) = url { let s = if v.is_empty() { None } else { Some(v) }; conn.execute("UPDATE mcp_servers SET url=?1,updated_at=?2 WHERE id=?3", params![s,now,id]).map_err(|e|e.to_string())?; }
    if let Some(v) = env_vars { let j = serde_json::to_string(&v).map_err(|e|e.to_string())?; conn.execute("UPDATE mcp_servers SET env_vars_json=?1,updated_at=?2 WHERE id=?3", params![j,now,id]).map_err(|e|e.to_string())?; }
    if let Some(v) = auth { let j = v.to_string(); conn.execute("UPDATE mcp_servers SET auth_json=?1,updated_at=?2 WHERE id=?3", params![j,now,id]).map_err(|e|e.to_string())?; }
    if let Some(v) = is_global { conn.execute("UPDATE mcp_servers SET is_global=?1,updated_at=?2 WHERE id=?3", params![v,now,id]).map_err(|e|e.to_string())?; }
    if let Some(v) = project_id { let s = if v.is_empty() { None } else { Some(v) }; conn.execute("UPDATE mcp_servers SET project_id=?1,updated_at=?2 WHERE id=?3", params![s,now,id]).map_err(|e|e.to_string())?; }
    fetch_server(&conn, &id)
}

#[tauri::command]
pub fn mcp_server_delete(id: String) -> Result<(), String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM mcp_servers WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn mcp_server_set_active(id: String, is_active: bool) -> Result<DbMcpServer, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute("UPDATE mcp_servers SET is_active=?1,updated_at=?2 WHERE id=?3", params![is_active,now,id]).map_err(|e| e.to_string())?;
    fetch_server(&conn, &id)
}

#[tauri::command]
pub fn mcp_server_export(id: String) -> Result<String, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let server = fetch_server(&conn, &id)?;
    // Strip secret env var values on export
    let mut export = server.clone();
    for v in &mut export.env_vars {
        if v.is_secret { v.value = String::new(); }
    }
    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn mcp_server_import(json: String) -> Result<DbMcpServer, String> {
    let server: DbMcpServer = serde_json::from_str(&json).map_err(|e| format!("invalid MCP server JSON: {e}"))?;
    mcp_server_create(
        server.name, server.transport, server.command, server.url,
        Some(server.env_vars), server.auth, Some(server.is_global), server.project_id,
    )
}
