// ACP agent sandbox enforcement (engineering-todo-v2.md 1.15.5 preview).
// Implements a process-level permission model and audit log for agent tool calls.
// True VM-based sandboxing (Firecracker/gVisor) is production infrastructure
// beyond the scope of IDE code; this layer provides:
//   1. allowed_roots enforcement: block file operations outside workspace
//   2. Audit log: append-only SQLite record of every agent tool execution
//   3. Resource configuration: store per-agent timeout + file size limits

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

use crate::chat_db::get_db_path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AuditLogEntry {
    pub id: String,
    pub agent_id: String,
    pub connection_id: Option<String>,
    pub session_id: Option<String>,
    pub tool: String,
    pub args_json: String,
    pub outcome: String, // "allowed" | "denied" | "error"
    pub denied_reason: Option<String>,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SandboxConfig {
    pub agent_id: String,
    /// Comma-separated list of absolute paths the agent may read/write within.
    /// Empty means "workspace root only" (the session's cwd).
    pub allowed_roots: String,
    /// Maximum file size in bytes the agent may read/write. Default: 10MB.
    pub max_file_bytes: i64,
    /// Maximum time in seconds any single tool call may take. Default: 300.
    pub tool_timeout_secs: i64,
    /// Whether to allow network access via terminal. Default: true.
    pub allow_network: bool,
}

pub fn init_sandbox_tables(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_audit_log (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            connection_id TEXT,
            session_id TEXT,
            tool TEXT NOT NULL,
            args_json TEXT NOT NULL DEFAULT '{}',
            outcome TEXT NOT NULL DEFAULT 'allowed',
            denied_reason TEXT,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_sandbox_config (
            agent_id TEXT PRIMARY KEY,
            allowed_roots TEXT NOT NULL DEFAULT '',
            max_file_bytes INTEGER NOT NULL DEFAULT 10485760,
            tool_timeout_secs INTEGER NOT NULL DEFAULT 300,
            allow_network INTEGER NOT NULL DEFAULT 1
        )",
        [],
    )?;

    Ok(())
}

/// Check whether a given file path is permitted under the agent's sandbox config.
/// Returns Ok(()) if allowed, Err(reason) if denied.
pub fn check_path_allowed(
    sandbox: &SandboxConfig,
    workspace_root: &str,
    file_path: &str,
) -> Result<(), String> {
    let canonical = std::fs::canonicalize(file_path)
        .unwrap_or_else(|_| std::path::PathBuf::from(file_path));

    // Always allow within the session workspace root.
    let ws = std::path::PathBuf::from(workspace_root);
    if canonical.starts_with(&ws) {
        return Ok(());
    }

    // Check additional allowed roots.
    if !sandbox.allowed_roots.is_empty() {
        for root in sandbox.allowed_roots.split(',') {
            let root = root.trim();
            if !root.is_empty() {
                let root_path = std::path::PathBuf::from(root);
                if canonical.starts_with(&root_path) {
                    return Ok(());
                }
            }
        }
    }

    Err(format!(
        "Path '{}' is outside the agent's allowed workspace ('{}' and {} additional roots). Configure allowed_roots in the agent sandbox settings to grant access.",
        file_path,
        workspace_root,
        if sandbox.allowed_roots.is_empty() { "no".to_string() } else {
            sandbox.allowed_roots.split(',').count().to_string()
        }
    ))
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn sandbox_get_config(agent_id: String) -> Result<Option<SandboxConfig>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    match conn.query_row(
        "SELECT agent_id, allowed_roots, max_file_bytes, tool_timeout_secs, allow_network FROM agent_sandbox_config WHERE agent_id = ?1",
        params![agent_id],
        |row| Ok(SandboxConfig {
            agent_id: row.get(0)?,
            allowed_roots: row.get(1)?,
            max_file_bytes: row.get(2)?,
            tool_timeout_secs: row.get(3)?,
            allow_network: row.get(4)?,
        }),
    ) {
        Ok(cfg) => Ok(Some(cfg)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn sandbox_set_config(
    agent_id: String,
    allowed_roots: Option<String>,
    max_file_bytes: Option<i64>,
    tool_timeout_secs: Option<i64>,
    allow_network: Option<bool>,
) -> Result<SandboxConfig, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO agent_sandbox_config (agent_id, allowed_roots, max_file_bytes, tool_timeout_secs, allow_network)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(agent_id) DO UPDATE SET
           allowed_roots = COALESCE(?2, allowed_roots),
           max_file_bytes = COALESCE(?3, max_file_bytes),
           tool_timeout_secs = COALESCE(?4, tool_timeout_secs),
           allow_network = COALESCE(?5, allow_network)",
        params![
            agent_id,
            allowed_roots.unwrap_or_default(),
            max_file_bytes.unwrap_or(10_485_760),
            tool_timeout_secs.unwrap_or(300),
            allow_network.unwrap_or(true)
        ],
    ).map_err(|e| e.to_string())?;

    sandbox_get_config(agent_id).map(|opt| opt.expect("just inserted"))
}

#[tauri::command]
pub fn sandbox_audit_log(
    agent_id: String,
    connection_id: Option<String>,
    session_id: Option<String>,
    tool: String,
    args_json: String,
    outcome: String,
    denied_reason: Option<String>,
) -> Result<AuditLogEntry, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO agent_audit_log (id, agent_id, connection_id, session_id, tool, args_json, outcome, denied_reason, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![id, agent_id, connection_id, session_id, tool, args_json, outcome, denied_reason, now],
    ).map_err(|e| e.to_string())?;
    Ok(AuditLogEntry {
        id,
        agent_id,
        connection_id: None,
        session_id: None,
        tool,
        args_json,
        outcome,
        denied_reason,
        created_at: now,
    })
}

#[tauri::command]
pub fn sandbox_get_audit_log(
    agent_id: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<AuditLogEntry>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let lim = limit.unwrap_or(200);
    let rows = match agent_id {
        Some(aid) => {
            let mut stmt = conn.prepare(
                "SELECT id, agent_id, connection_id, session_id, tool, args_json, outcome, denied_reason, created_at
                 FROM agent_audit_log WHERE agent_id = ?1 ORDER BY created_at DESC LIMIT ?2"
            ).map_err(|e| e.to_string())?;
            stmt.query_map(params![aid, lim], |row| Ok(AuditLogEntry {
                id: row.get(0)?, agent_id: row.get(1)?, connection_id: row.get(2)?,
                session_id: row.get(3)?, tool: row.get(4)?, args_json: row.get(5)?,
                outcome: row.get(6)?, denied_reason: row.get(7)?, created_at: row.get(8)?,
            })).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect()
        }
        None => {
            let mut stmt = conn.prepare(
                "SELECT id, agent_id, connection_id, session_id, tool, args_json, outcome, denied_reason, created_at
                 FROM agent_audit_log ORDER BY created_at DESC LIMIT ?1"
            ).map_err(|e| e.to_string())?;
            stmt.query_map(params![lim], |row| Ok(AuditLogEntry {
                id: row.get(0)?, agent_id: row.get(1)?, connection_id: row.get(2)?,
                session_id: row.get(3)?, tool: row.get(4)?, args_json: row.get(5)?,
                outcome: row.get(6)?, denied_reason: row.get(7)?, created_at: row.get(8)?,
            })).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect()
        }
    };
    Ok(rows)
}

#[tauri::command]
pub fn sandbox_check_path(
    agent_id: String,
    workspace_root: String,
    file_path: String,
) -> Result<bool, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let sandbox = match conn.query_row(
        "SELECT agent_id, allowed_roots, max_file_bytes, tool_timeout_secs, allow_network FROM agent_sandbox_config WHERE agent_id = ?1",
        params![agent_id],
        |row| Ok(SandboxConfig {
            agent_id: row.get(0)?,
            allowed_roots: row.get(1)?,
            max_file_bytes: row.get(2)?,
            tool_timeout_secs: row.get(3)?,
            allow_network: row.get(4)?,
        }),
    ) {
        Ok(cfg) => cfg,
        Err(_) => SandboxConfig {
            agent_id: agent_id.clone(),
            allowed_roots: String::new(),
            max_file_bytes: 10_485_760,
            tool_timeout_secs: 300,
            allow_network: true,
        },
    };

    match check_path_allowed(&sandbox, &workspace_root, &file_path) {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
    }
}
