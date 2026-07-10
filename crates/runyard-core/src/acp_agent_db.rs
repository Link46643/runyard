// ACP agent registry (engineering-todo-v2.md 1.6.1): persisted configuration
// for every ACP agent Runyard knows about - built-in suggestions, agents
// discovered on PATH, and manually-added ones. Follows the exact pattern
// established in chat_db.rs (same SQLite file, same open-per-call style,
// same migration approach) for consistency across the codebase.
//
// Security note on env vars (1.6.8): values for entries marked `is_secret`
// are stored as-is in the local SQLite file (which already lives in the
// user's own home directory, never transmitted). True OS-keychain-backed
// storage would need a new dependency (e.g. the `keyring` crate) that not
// been approved for this workspace - flagged here rather than silently
// implemented. `acp_agent_export` never includes secret values (see below),
// so at minimum, secrets can't leak through config sharing/export.

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::chat_db::get_db_path;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DbAcpAgentEnvVar {
    pub key: String,
    pub value: String,
    pub is_secret: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbAcpAgent {
    pub id: String,
    pub name: String,
    pub agent_id: String,
    pub executable_path: Option<String>,
    pub spawn_command: Option<String>,
    pub remote_url: Option<String>,
    pub transport: String, // "stdio" | "http" | "websocket"
    pub env_vars: Vec<DbAcpAgentEnvVar>,
    pub capabilities: Option<Value>,
    pub discovery_source: String, // "builtin" | "path_scan" | "registry" | "manual"
    pub status: String,           // mirrors runyard_acp::ConnectionStatus, as a string
    pub last_error: Option<String>,
    pub is_builtin: bool,
    pub is_active: bool,
    pub is_default: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Portable subset of `DbAcpAgent` used for import/export (1.6.10).
/// Deliberately excludes local-only bookkeeping (id, status, last_error,
/// is_default, timestamps) and never carries secret env var values - only
/// their key names and the `is_secret` flag, so a shared/exported config
/// tells the importer which env vars they need to fill in themselves.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AcpAgentExport {
    pub name: String,
    pub agent_id: String,
    pub executable_path: Option<String>,
    pub spawn_command: Option<String>,
    pub remote_url: Option<String>,
    pub transport: String,
    pub env_vars: Vec<DbAcpAgentEnvVar>,
}

pub fn init_acp_agent_tables(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS acp_agents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            agent_id TEXT NOT NULL,
            executable_path TEXT,
            spawn_command TEXT,
            remote_url TEXT,
            transport TEXT NOT NULL DEFAULT 'stdio',
            env_vars_json TEXT NOT NULL DEFAULT '[]',
            capabilities_json TEXT,
            discovery_source TEXT NOT NULL DEFAULT 'manual',
            status TEXT NOT NULL DEFAULT 'disconnected',
            last_error TEXT,
            is_builtin INTEGER NOT NULL DEFAULT 0,
            is_active INTEGER NOT NULL DEFAULT 1,
            is_default INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    // Per-project "last used / preferred agent" (1.6.5). One row per
    // workspace path; re-inserting for the same path replaces it.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS acp_agent_project_defaults (
            workspace_path TEXT PRIMARY KEY,
            agent_row_id TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (agent_row_id) REFERENCES acp_agents(id) ON DELETE CASCADE
        )",
        [],
    )?;

    Ok(())
}

fn row_to_agent(row: &rusqlite::Row) -> SqlResult<DbAcpAgent> {
    let env_vars_json: String = row.get(7)?;
    let env_vars: Vec<DbAcpAgentEnvVar> = serde_json::from_str(&env_vars_json).unwrap_or_default();
    let capabilities_json: Option<String> = row.get(8)?;
    let capabilities = capabilities_json.and_then(|s| serde_json::from_str(&s).ok());

    Ok(DbAcpAgent {
        id: row.get(0)?,
        name: row.get(1)?,
        agent_id: row.get(2)?,
        executable_path: row.get(3)?,
        spawn_command: row.get(4)?,
        remote_url: row.get(5)?,
        transport: row.get(6)?,
        env_vars,
        capabilities,
        discovery_source: row.get(9)?,
        status: row.get(10)?,
        last_error: row.get(11)?,
        is_builtin: row.get(12)?,
        is_active: row.get(13)?,
        is_default: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

const SELECT_COLUMNS: &str = "id, name, agent_id, executable_path, spawn_command, remote_url, transport, env_vars_json, capabilities_json, discovery_source, status, last_error, is_builtin, is_active, is_default, created_at, updated_at";

fn fetch_agent(conn: &Connection, id: &str) -> Result<DbAcpAgent, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT {SELECT_COLUMNS} FROM acp_agents WHERE id = ?1"))
        .map_err(|e| e.to_string())?;
    stmt.query_row(params![id], row_to_agent).map_err(|e| e.to_string())
}

// ── Commands ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn acp_agent_list() -> Result<Vec<DbAcpAgent>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(&format!("SELECT {SELECT_COLUMNS} FROM acp_agents ORDER BY is_builtin DESC, name ASC"))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_agent).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn acp_agent_get(id: String) -> Result<DbAcpAgent, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    fetch_agent(&conn, &id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn acp_agent_create(
    name: String,
    agent_id: String,
    transport: String,
    executable_path: Option<String>,
    spawn_command: Option<String>,
    remote_url: Option<String>,
    env_vars: Option<Vec<DbAcpAgentEnvVar>>,
    discovery_source: Option<String>,
    is_builtin: Option<bool>,
) -> Result<DbAcpAgent, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();
    let env_vars = env_vars.unwrap_or_default();
    let env_vars_json = serde_json::to_string(&env_vars).map_err(|e| e.to_string())?;
    let source = discovery_source.unwrap_or_else(|| "manual".to_string());
    let builtin = is_builtin.unwrap_or(false);

    conn.execute(
        "INSERT INTO acp_agents (id, name, agent_id, executable_path, spawn_command, remote_url, transport, env_vars_json, capabilities_json, discovery_source, status, last_error, is_builtin, is_active, is_default, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, 'disconnected', NULL, ?10, 1, 0, ?11, ?12)",
        params![id, name, agent_id, executable_path, spawn_command, remote_url, transport, env_vars_json, source, builtin, now, now],
    ).map_err(|e| e.to_string())?;

    fetch_agent(&conn, &id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn acp_agent_update(
    id: String,
    name: Option<String>,
    executable_path: Option<String>,
    spawn_command: Option<String>,
    remote_url: Option<String>,
    transport: Option<String>,
    env_vars: Option<Vec<DbAcpAgentEnvVar>>,
) -> Result<DbAcpAgent, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();

    if let Some(v) = name {
        conn.execute("UPDATE acp_agents SET name = ?1, updated_at = ?2 WHERE id = ?3", params![v, now, id]).map_err(|e| e.to_string())?;
    }
    // executable_path / spawn_command / remote_url are all individually
    // nullable-by-design (e.g. clearing spawn_command when switching an
    // agent to remote), so an explicit empty string clears the column while
    // omitting the field entirely leaves it untouched.
    if let Some(v) = executable_path {
        let stored = if v.is_empty() { None } else { Some(v) };
        conn.execute("UPDATE acp_agents SET executable_path = ?1, updated_at = ?2 WHERE id = ?3", params![stored, now, id]).map_err(|e| e.to_string())?;
    }
    if let Some(v) = spawn_command {
        let stored = if v.is_empty() { None } else { Some(v) };
        conn.execute("UPDATE acp_agents SET spawn_command = ?1, updated_at = ?2 WHERE id = ?3", params![stored, now, id]).map_err(|e| e.to_string())?;
    }
    if let Some(v) = remote_url {
        let stored = if v.is_empty() { None } else { Some(v) };
        conn.execute("UPDATE acp_agents SET remote_url = ?1, updated_at = ?2 WHERE id = ?3", params![stored, now, id]).map_err(|e| e.to_string())?;
    }
    if let Some(v) = transport {
        conn.execute("UPDATE acp_agents SET transport = ?1, updated_at = ?2 WHERE id = ?3", params![v, now, id]).map_err(|e| e.to_string())?;
    }
    if let Some(v) = env_vars {
        let json = serde_json::to_string(&v).map_err(|e| e.to_string())?;
        conn.execute("UPDATE acp_agents SET env_vars_json = ?1, updated_at = ?2 WHERE id = ?3", params![json, now, id]).map_err(|e| e.to_string())?;
    }

    fetch_agent(&conn, &id)
}

#[tauri::command]
pub fn acp_agent_delete(id: String) -> Result<(), String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM acp_agents WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn acp_agent_set_active(id: String, is_active: bool) -> Result<DbAcpAgent, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE acp_agents SET is_active = ?1, updated_at = ?2 WHERE id = ?3",
        params![is_active, now, id],
    ).map_err(|e| e.to_string())?;
    fetch_agent(&conn, &id)
}

/// Called by the Tauri ACP bridge (not directly by the frontend) whenever a
/// connection's status changes, so the agent panel's status badge (1.6.6)
/// survives an app restart instead of resetting to "disconnected" only in
/// memory.
#[tauri::command]
pub fn acp_agent_set_status(id: String, status: String, last_error: Option<String>) -> Result<DbAcpAgent, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "UPDATE acp_agents SET status = ?1, last_error = ?2, updated_at = ?3 WHERE id = ?4",
        params![status, last_error, now, id],
    ).map_err(|e| e.to_string())?;
    fetch_agent(&conn, &id)
}

/// Called by the bridge on a successful `Connected` event, persisting what
/// the agent actually advertised (1.6.7 capabilities display).
#[tauri::command]
pub fn acp_agent_set_capabilities(id: String, capabilities: Value) -> Result<DbAcpAgent, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    let json = serde_json::to_string(&capabilities).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE acp_agents SET capabilities_json = ?1, updated_at = ?2 WHERE id = ?3",
        params![json, now, id],
    ).map_err(|e| e.to_string())?;
    fetch_agent(&conn, &id)
}

#[tauri::command]
pub fn acp_agent_set_default_for_project(workspace_path: String, agent_row_id: String) -> Result<(), String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "INSERT INTO acp_agent_project_defaults (workspace_path, agent_row_id, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(workspace_path) DO UPDATE SET agent_row_id = excluded.agent_row_id, updated_at = excluded.updated_at",
        params![workspace_path, agent_row_id, now],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn acp_agent_get_default_for_project(workspace_path: String) -> Result<Option<DbAcpAgent>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let agent_row_id: Option<String> = conn
        .query_row(
            "SELECT agent_row_id FROM acp_agent_project_defaults WHERE workspace_path = ?1",
            params![workspace_path],
            |row| row.get(0),
        )
        .ok();

    match agent_row_id {
        Some(row_id) => fetch_agent(&conn, &row_id).map(Some),
        None => Ok(None),
    }
}

#[tauri::command]
pub fn acp_agent_export(id: String) -> Result<String, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let agent = fetch_agent(&conn, &id)?;
    let export = AcpAgentExport {
        name: agent.name,
        agent_id: agent.agent_id,
        executable_path: agent.executable_path,
        spawn_command: agent.spawn_command,
        remote_url: agent.remote_url,
        transport: agent.transport,
        env_vars: agent
            .env_vars
            .into_iter()
            .map(|v| if v.is_secret { DbAcpAgentEnvVar { value: String::new(), ..v } } else { v })
            .collect(),
    };
    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn acp_agent_import(json: String) -> Result<DbAcpAgent, String> {
    let export: AcpAgentExport = serde_json::from_str(&json).map_err(|e| format!("invalid agent config JSON: {e}"))?;
    acp_agent_create(
        export.name,
        export.agent_id,
        export.transport,
        export.executable_path,
        export.spawn_command,
        export.remote_url,
        Some(export.env_vars),
        Some("manual".to_string()),
        Some(false),
    )
}

/// Thin wrapper around `runyard_acp::discover_known_agents` so the frontend
/// has one command namespace (`acp_agent_*`) for everything agent-related,
/// without needing to know the discovery logic lives in a different crate.
#[tauri::command]
pub fn acp_agent_discover() -> Result<Vec<runyard_acp::DiscoveredAgent>, String> {
    Ok(runyard_acp::discover_known_agents())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("failed to open in-memory sqlite db");
        init_acp_agent_tables(&conn).expect("failed to init acp_agents tables");
        conn
    }

    #[test]
    fn schema_creates_and_round_trips_a_row() {
        let conn = setup_conn();
        let now = 1_000_i64;
        let env_vars = vec![DbAcpAgentEnvVar { key: "ANTHROPIC_API_KEY".into(), value: "sk-test".into(), is_secret: true }];
        let env_vars_json = serde_json::to_string(&env_vars).unwrap();

        conn.execute(
            "INSERT INTO acp_agents (id, name, agent_id, executable_path, spawn_command, remote_url, transport, env_vars_json, capabilities_json, discovery_source, status, last_error, is_builtin, is_active, is_default, created_at, updated_at)
             VALUES ('row-1', 'Claude Code', 'claude-code', '/usr/bin/claude', NULL, NULL, 'stdio', ?1, NULL, 'path_scan', 'disconnected', NULL, 0, 1, 0, ?2, ?2)",
            params![env_vars_json, now],
        ).unwrap();

        let agent = fetch_agent(&conn, "row-1").unwrap();
        assert_eq!(agent.name, "Claude Code");
        assert_eq!(agent.transport, "stdio");
        assert_eq!(agent.env_vars.len(), 1);
        assert_eq!(agent.env_vars[0].key, "ANTHROPIC_API_KEY");
        assert!(agent.env_vars[0].is_secret);
        assert!(agent.capabilities.is_none());
    }

    #[test]
    fn export_strips_secret_values_but_keeps_keys() {
        let agent = DbAcpAgent {
            id: "row-1".into(),
            name: "Claude Code".into(),
            agent_id: "claude-code".into(),
            executable_path: Some("/usr/bin/claude".into()),
            spawn_command: None,
            remote_url: None,
            transport: "stdio".into(),
            env_vars: vec![
                DbAcpAgentEnvVar { key: "ANTHROPIC_API_KEY".into(), value: "sk-real-secret".into(), is_secret: true },
                DbAcpAgentEnvVar { key: "MODEL".into(), value: "claude-sonnet".into(), is_secret: false },
            ],
            capabilities: None,
            discovery_source: "path_scan".into(),
            status: "disconnected".into(),
            last_error: None,
            is_builtin: false,
            is_active: true,
            is_default: false,
            created_at: 1000,
            updated_at: 1000,
        };

        let export = AcpAgentExport {
            name: agent.name,
            agent_id: agent.agent_id,
            executable_path: agent.executable_path,
            spawn_command: agent.spawn_command,
            remote_url: agent.remote_url,
            transport: agent.transport,
            env_vars: agent
                .env_vars
                .into_iter()
                .map(|v| if v.is_secret { DbAcpAgentEnvVar { value: String::new(), ..v } } else { v })
                .collect(),
        };

        assert_eq!(export.env_vars[0].key, "ANTHROPIC_API_KEY");
        assert_eq!(export.env_vars[0].value, "", "secret values must never be exported");
        assert_eq!(export.env_vars[1].value, "claude-sonnet", "non-secret values should export unchanged");
    }

    #[test]
    fn project_default_upsert_replaces_previous_value() {
        let conn = setup_conn();
        let now = 1_000_i64;
        for (id, name) in [("row-1", "Agent One"), ("row-2", "Agent Two")] {
            conn.execute(
                "INSERT INTO acp_agents (id, name, agent_id, transport, env_vars_json, discovery_source, status, is_builtin, is_active, is_default, created_at, updated_at)
                 VALUES (?1, ?2, 'x', 'stdio', '[]', 'manual', 'disconnected', 0, 1, 0, ?3, ?3)",
                params![id, name, now],
            ).unwrap();
        }

        conn.execute(
            "INSERT INTO acp_agent_project_defaults (workspace_path, agent_row_id, updated_at) VALUES ('/proj', 'row-1', ?1)
             ON CONFLICT(workspace_path) DO UPDATE SET agent_row_id = excluded.agent_row_id, updated_at = excluded.updated_at",
            params![now],
        ).unwrap();
        conn.execute(
            "INSERT INTO acp_agent_project_defaults (workspace_path, agent_row_id, updated_at) VALUES ('/proj', 'row-2', ?1)
             ON CONFLICT(workspace_path) DO UPDATE SET agent_row_id = excluded.agent_row_id, updated_at = excluded.updated_at",
            params![now + 1],
        ).unwrap();

        let agent_row_id: String = conn
            .query_row("SELECT agent_row_id FROM acp_agent_project_defaults WHERE workspace_path = '/proj'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(agent_row_id, "row-2", "second upsert should have replaced the first");
    }
}
