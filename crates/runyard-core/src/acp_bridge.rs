// Tauri bridge for runyard-acp (engineering-todo-v2.md 1.7.17 "ACP event
// bridge to frontend", plus the connection-management half of 1.6.4/1.6.5/
// 1.6.6). Thin by design: `runyard_acp::AcpConnectionPool` already owns all
// the actual protocol logic and is exhaustively tested in its own crate
// (crates/runyard-acp) - this module's only job is (a) translating Tauri
// command calls into pool method calls and (b) forwarding every AcpEvent
// from the pool's shared event channel to the frontend as a Tauri event.
//
// Can't be compiled in this sandbox (same pre-existing GTK/glib wall that
// blocks all of runyard-core), so this is hand-written and hand-reviewed
// against the real runyard_acp public API (crates/runyard-acp/src/client.rs,
// pool.rs), not verified by `cargo build` here - disclosed plainly rather
// than claimed as tested. The one thing that *can't* regress silently is the
// protocol logic itself, since that lives in and is tested by runyard-acp.

use std::sync::Arc;

use runyard_acp::{AcpConnectionPool, AcpEvent, AgentTransportConfig};
use serde_json::Value;
use tauri::{AppHandle, Runtime};
use tokio::sync::mpsc;

use crate::acp_agent_db::{acp_agent_get, acp_agent_set_status};
use crate::EventBridge;

/// Max simultaneous agent connections (1.7.14). Matches the todo's own
/// number ("Pool limits (max 10 concurrent)").
const MAX_CONCURRENT_CONNECTIONS: usize = 10;

/// Managed Tauri state - one pool for the whole app's lifetime.
#[derive(Clone)]
pub struct AcpBridgeState(pub Arc<AcpConnectionPool>);

/// Sets up the shared connection pool and spawns the task that forwards
/// every event from every connection to the frontend as a single
/// `"acp:event"` Tauri event (payload = the serialized `AcpEvent`, tag
/// `"type"` - see runyard_acp::events for the exact shape). Call once from
/// Tauri's `.setup()` hook and `.manage()` the result.
pub fn init_acp_bridge<R: Runtime>(app: &AppHandle<R>) -> AcpBridgeState {
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AcpEvent>();
    let pool = Arc::new(AcpConnectionPool::new(MAX_CONCURRENT_CONNECTIONS, event_tx));

    // On every startup, any agent that was still marked "connected", "connecting",
    // or "processing" in the DB is stale — the previous session either crashed or
    // was killed without clean shutdown. Reset them to "disconnected" so the UI
    // badge doesn't show a phantom green dot.
    reset_stale_agent_statuses();

    let bridge: Arc<dyn EventBridge> = Arc::new(app.clone());
    tauri::async_runtime::spawn(async move {
        while let Some(event) = event_rx.recv().await {
            // Best-effort persistence of connection status/capabilities so
            // the agent panel's status badge (1.6.6) and capabilities
            // display (1.6.7) survive a restart, not just live in memory.
            // agent_row_id isn't part of AcpEvent (it only knows about
            // runyard_acp's own connection_id) - the frontend is the one
            // that knows which DB row a given connection_id belongs to, so
            // it calls acp_agent_set_status/acp_agent_set_capabilities
            // itself in response to the forwarded event. This task's only
            // job is forwarding.
            let payload = serde_json::to_value(&event).unwrap_or_else(|_| Value::Null);
            let _ = bridge.send_event("acp:event", payload);
        }
    });

    AcpBridgeState(pool)
}

/// Resets every agent whose DB status is "connected", "connecting", or
/// "processing" to "disconnected". Called on startup to clear stale state
/// from a previous session that ended without clean shutdown.
fn reset_stale_agent_statuses() {
    use crate::acp_agent_db::acp_agent_list;
    if let Ok(agents) = acp_agent_list() {
        for agent in agents {
            let stale = matches!(
                agent.status.as_str(),
                "connected" | "connecting" | "processing"
            );
            if stale {
                let _ = acp_agent_set_status(
                    agent.id.to_string(),
                    "disconnected".to_string(),
                    None,
                );
            }
        }
    }
}

fn transport_for_agent(agent: &crate::acp_agent_db::DbAcpAgent) -> Result<AgentTransportConfig, String> {
    match agent.transport.as_str() {
        "stdio" => {
            // No OS-specific wrapping here on purpose. AcpAgent::from_str
            // (agent-client-protocol-tokio) parses this string with
            // shell_words::split - a plain cross-platform argv tokenizer,
            // not a shell invocation - then spawns it via
            // tokio::process::Command::new(binary).args(args). Modern Rust's
            // std::process::Command already resolves PATH (including
            // PATHEXT, so npm-installed .cmd/.bat shims like opencode.cmd
            // resolve correctly) and safely executes batch files on Windows
            // with correct argument escaping - no cmd.exe /c wrapper is
            // needed, and adding one would double-tokenize the command
            // string and break on any argument containing spaces or quotes.
            let command = agent
                .spawn_command
                .clone()
                .or_else(|| agent.executable_path.clone())
                .ok_or_else(|| format!("agent '{}' has no spawn_command or executable_path configured", agent.name))?;
            Ok(AgentTransportConfig::stdio(command))
        }
        "http" => {
            let url = agent
                .remote_url
                .clone()
                .ok_or_else(|| format!("agent '{}' has no remote_url configured", agent.name))?;
            Ok(AgentTransportConfig::http(url))
        }
        "websocket" => {
            let url = agent
                .remote_url
                .clone()
                .ok_or_else(|| format!("agent '{}' has no remote_url configured", agent.name))?;
            Ok(AgentTransportConfig::websocket(url))
        }
        other => Err(format!("unknown transport '{other}' for agent '{}'", agent.name)),
    }
}

// ── Connection lifecycle (1.6.4 launcher, 1.6.6 status monitoring) ─────────

#[tauri::command]
pub async fn acp_connect(state: tauri::State<'_, AcpBridgeState>, agent_row_id: String) -> Result<String, String> {
    let agent = acp_agent_get(agent_row_id.clone())?;
    let transport = transport_for_agent(&agent)?;

    let _ = acp_agent_set_status(agent_row_id.clone(), "connecting".to_string(), None);
    match state.0.connect(transport).await {
        Ok(connection_id) => {
            let _ = acp_agent_set_status(agent_row_id, "connected".to_string(), None);
            Ok(connection_id)
        }
        Err(e) => {
            let _ = acp_agent_set_status(agent_row_id, "error".to_string(), Some(e.to_string()));
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn acp_disconnect(state: tauri::State<'_, AcpBridgeState>, connection_id: String) -> Result<(), String> {
    state.0.disconnect(&connection_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn acp_list_connections(state: tauri::State<'_, AcpBridgeState>) -> Result<Vec<String>, String> {
    Ok(state.0.active_connection_ids().await)
}

// ── Session lifecycle (1.7.7) ───────────────────────────────────────────────

#[tauri::command]
pub async fn acp_new_session(
    state: tauri::State<'_, AcpBridgeState>,
    connection_id: String,
    cwd: String,
    mcp_servers: Option<Vec<Value>>,
) -> Result<String, String> {
    state
        .0
        .with_client(&connection_id, move |client| {
            Box::pin(async move { client.new_session(cwd, mcp_servers.unwrap_or_default()).await })
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn acp_load_session(
    state: tauri::State<'_, AcpBridgeState>,
    connection_id: String,
    session_id: String,
    cwd: String,
) -> Result<(), String> {
    state
        .0
        .with_client(&connection_id, move |client| Box::pin(async move { client.load_session(session_id, cwd).await }))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn acp_resume_session(
    state: tauri::State<'_, AcpBridgeState>,
    connection_id: String,
    session_id: String,
    cwd: String,
) -> Result<(), String> {
    state
        .0
        .with_client(&connection_id, move |client| Box::pin(async move { client.resume_session(session_id, cwd).await }))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn acp_list_sessions(state: tauri::State<'_, AcpBridgeState>, connection_id: String) -> Result<Vec<String>, String> {
    state
        .0
        .with_client(&connection_id, |client| Box::pin(async move { client.list_sessions().await }))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn acp_close_session(state: tauri::State<'_, AcpBridgeState>, connection_id: String, session_id: String) -> Result<(), String> {
    state
        .0
        .with_client(&connection_id, move |client| Box::pin(async move { client.close_session(session_id).await }))
        .await
        .map_err(|e| e.to_string())
}

// ── Prompting (1.7.8, 1.7.9) ────────────────────────────────────────────────

#[tauri::command]
pub async fn acp_send_prompt(
    state: tauri::State<'_, AcpBridgeState>,
    connection_id: String,
    session_id: String,
    text: String,
) -> Result<(), String> {
    state
        .0
        .with_client(&connection_id, move |client| Box::pin(async move { client.send_prompt(session_id, text).await }))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn acp_cancel(state: tauri::State<'_, AcpBridgeState>, connection_id: String, session_id: String) -> Result<(), String> {
    state
        .0
        .with_client(&connection_id, move |client| Box::pin(async move { client.cancel(session_id).await }))
        .await
        .map_err(|e| e.to_string())
}

// ── Permissions (1.7.11) ────────────────────────────────────────────────────

#[tauri::command]
pub async fn acp_respond_permission(
    state: tauri::State<'_, AcpBridgeState>,
    connection_id: String,
    request_id: String,
    option_id: Option<String>,
) -> Result<(), String> {
    state
        .0
        .with_client(&connection_id, move |client| Box::pin(async move { client.respond_permission(request_id, option_id).await }))
        .await
        .map_err(|e| e.to_string())
}

// ── Config options / modes (1.7.13) ─────────────────────────────────────────

#[tauri::command]
pub async fn acp_set_mode(state: tauri::State<'_, AcpBridgeState>, connection_id: String, session_id: String, mode: String) -> Result<(), String> {
    state
        .0
        .with_client(&connection_id, move |client| Box::pin(async move { client.set_mode(session_id, mode).await }))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn acp_set_config_option(
    state: tauri::State<'_, AcpBridgeState>,
    connection_id: String,
    session_id: String,
    key: String,
    value: Value,
) -> Result<(), String> {
    state
        .0
        .with_client(&connection_id, move |client| Box::pin(async move { client.set_config_option(session_id, key, value).await }))
        .await
        .map_err(|e| e.to_string())
}

// ── Auth (1.7.6) ─────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn acp_authenticate(state: tauri::State<'_, AcpBridgeState>, connection_id: String, method_id: String) -> Result<(), String> {
    state
        .0
        .with_client(&connection_id, move |client| Box::pin(async move { client.authenticate(method_id).await }))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn acp_logout(state: tauri::State<'_, AcpBridgeState>, connection_id: String) -> Result<(), String> {
    state
        .0
        .with_client(&connection_id, |client| Box::pin(async move { client.logout().await }))
        .await
        .map_err(|e| e.to_string())
}
