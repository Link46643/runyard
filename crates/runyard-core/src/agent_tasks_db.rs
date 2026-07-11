// ACP agent task tracking (engineering-todo-v2.md 1.10.1).
// Records tasks dispatched to ACP agents: status, cost, tool activity.
// Updated by the Tauri ACP bridge as events arrive.

use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::chat_db::get_db_path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbAgentTask {
    pub id: String,
    pub agent_id: String,
    pub agent_name: String,
    pub connection_id: Option<String>,
    pub session_id: Option<String>,
    pub conversation_id: Option<String>,
    pub project: Option<String>,
    pub description: String,
    pub status: String, // queued|running|awaiting_hil|completed|failed|cancelled
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
    pub cost_usd: f64,
    pub current_tool: Option<String>,
    pub error: Option<String>,
}

pub fn init_agent_task_tables(conn: &Connection) -> SqlResult<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS agent_tasks (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            agent_name TEXT NOT NULL DEFAULT '',
            connection_id TEXT,
            session_id TEXT,
            conversation_id TEXT,
            project TEXT,
            description TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'queued',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            completed_at INTEGER,
            cost_usd REAL NOT NULL DEFAULT 0.0,
            current_tool TEXT,
            error TEXT
        )",
        [],
    )?;
    Ok(())
}

fn row_to_task(row: &rusqlite::Row) -> SqlResult<DbAgentTask> {
    Ok(DbAgentTask {
        id: row.get(0)?,
        agent_id: row.get(1)?,
        agent_name: row.get(2)?,
        connection_id: row.get(3)?,
        session_id: row.get(4)?,
        conversation_id: row.get(5)?,
        project: row.get(6)?,
        description: row.get(7)?,
        status: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
        completed_at: row.get(11)?,
        cost_usd: row.get(12)?,
        current_tool: row.get(13)?,
        error: row.get(14)?,
    })
}

const SELECT_COLS: &str = "id, agent_id, agent_name, connection_id, session_id, conversation_id, project, description, status, created_at, updated_at, completed_at, cost_usd, current_tool, error";

fn fetch_task(conn: &Connection, id: &str) -> Result<DbAgentTask, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT {SELECT_COLS} FROM agent_tasks WHERE id = ?1"))
        .map_err(|e| e.to_string())?;
    stmt.query_row(params![id], row_to_task).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn agent_task_list(status: Option<String>) -> Result<Vec<DbAgentTask>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let rows = match status {
        Some(s) => {
            let mut stmt = conn.prepare(&format!("SELECT {SELECT_COLS} FROM agent_tasks WHERE status=?1 ORDER BY created_at DESC")).map_err(|e| e.to_string())?;
            let list: Vec<DbAgentTask> = stmt.query_map(params![s], row_to_task).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
            list
        }
        None => {
            let mut stmt = conn.prepare(&format!("SELECT {SELECT_COLS} FROM agent_tasks ORDER BY created_at DESC LIMIT 500")).map_err(|e| e.to_string())?;
            let list: Vec<DbAgentTask> = stmt.query_map([], row_to_task).map_err(|e| e.to_string())?.filter_map(|r| r.ok()).collect();
            list
        }
    };
    Ok(rows)
}

#[tauri::command]
pub fn agent_task_get(id: String) -> Result<DbAgentTask, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    fetch_task(&conn, &id)
}

#[tauri::command]
pub fn agent_task_create(
    agent_id: String,
    agent_name: String,
    connection_id: Option<String>,
    session_id: Option<String>,
    conversation_id: Option<String>,
    project: Option<String>,
    description: String,
) -> Result<DbAgentTask, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO agent_tasks (id,agent_id,agent_name,connection_id,session_id,conversation_id,project,description,status,created_at,updated_at,cost_usd) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'running',?9,?10,0.0)",
        params![id, agent_id, agent_name, connection_id, session_id, conversation_id, project, description, now, now],
    ).map_err(|e| e.to_string())?;
    fetch_task(&conn, &id)
}

#[tauri::command]
pub fn agent_task_update_status(
    id: String,
    status: String,
    current_tool: Option<String>,
    error: Option<String>,
    cost_usd: Option<f64>,
) -> Result<DbAgentTask, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    let is_terminal = matches!(status.as_str(), "completed" | "failed" | "cancelled");
    let completed_at: Option<i64> = if is_terminal { Some(now) } else { None };
    conn.execute(
        "UPDATE agent_tasks SET status=?1,current_tool=?2,error=?3,updated_at=?4,completed_at=COALESCE(?5,completed_at) WHERE id=?6",
        params![status, current_tool, error, now, completed_at, id],
    ).map_err(|e| e.to_string())?;
    if let Some(cost) = cost_usd {
        conn.execute("UPDATE agent_tasks SET cost_usd=?1 WHERE id=?2", params![cost, id]).map_err(|e| e.to_string())?;
    }
    fetch_task(&conn, &id)
}

#[tauri::command]
pub fn agent_task_delete(id: String) -> Result<(), String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM agent_tasks WHERE id=?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn agent_task_clear_completed() -> Result<usize, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let count = conn.execute("DELETE FROM agent_tasks WHERE status IN ('completed','failed','cancelled')", []).map_err(|e| e.to_string())?;
    Ok(count)
}

/// Global stats for the agent manager dashboard header.
#[tauri::command]
pub fn agent_task_stats() -> Result<serde_json::Value, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let active: i64 = conn.query_row("SELECT COUNT(*) FROM agent_tasks WHERE status IN ('running','awaiting_hil','queued')", [], |r| r.get(0)).unwrap_or(0);
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM agent_tasks", [], |r| r.get(0)).unwrap_or(0);
    let total_cost: f64 = conn.query_row("SELECT COALESCE(SUM(cost_usd),0) FROM agent_tasks", [], |r| r.get(0)).unwrap_or(0.0);
    let hil: i64 = conn.query_row("SELECT COUNT(*) FROM agent_tasks WHERE status='awaiting_hil'", [], |r| r.get(0)).unwrap_or(0);
    Ok(serde_json::json!({
        "activeTasks": active,
        "totalTasks": total,
        "totalCostUsd": total_cost,
        "hilPending": hil,
    }))
}
