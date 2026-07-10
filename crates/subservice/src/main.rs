use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::pin::Pin;
use std::task::{Context, Poll};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use rust_embed::RustEmbed;
use uuid::Uuid;
use bcrypt::{hash, verify, DEFAULT_COST};

use runyard_core::{
    EventBridge, LspState, TerminalState,
    commands::{fs_list, fs_read, fs_write, fs_watch_core, git_branch, get_home_dir},
    git_ops::{
        git_status, git_stage, git_unstage, git_discard, git_commit, git_log,
        git_branches, git_checkout, git_create_branch, git_worktrees,
        git_worktree_create, git_worktree_remove,
    },
    terminal::{
        terminal_write_core, terminal_resize_core, terminal_close_core, terminal_list_core,
    },
    lsp_manager::{
        lsp_send_core, lsp_stop_core, lsp_status_core, lsp_status_all_core,
    },
    settings::{settings_load, settings_save},
};

// ─── Embedded static assets ──────────────────────────────────────────────────
#[derive(RustEmbed)]
#[folder = "../../apps/desktop/build/"]
struct Asset;

// ─── Configuration ───────────────────────────────────────────────────────────
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
struct Config {
    port: u16,
    auth_token: String,
    auth_token_hash: String,
    allowed_roots: Vec<String>,
    lsp_enabled: bool,
    mcp_enabled: bool,
    agent_proxy_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 7820,
            auth_token: String::new(),
            auth_token_hash: String::new(),
            allowed_roots: vec![],
            lsp_enabled: true,
            mcp_enabled: true,
            agent_proxy_enabled: true,
        }
    }
}

// ─── JSON-RPC types ──────────────────────────────────────────────────────────
#[derive(Deserialize, Debug)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Option<Value>,
}

#[derive(Serialize, Debug)]
struct JsonRpcResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    id: Option<Value>,
}

#[derive(Serialize, Debug)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<Value>,
}

// ─── WebSocket Event Bridge ──────────────────────────────────────────────────
struct WsEventBridge {
    tx: tokio::sync::mpsc::UnboundedSender<Message>,
}

impl EventBridge for WsEventBridge {
    fn send_event(&self, event: &str, payload: Value) -> Result<(), String> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": event,
            "params": payload
        });
        self.tx
            .send(Message::Text(msg.to_string()))
            .map_err(|e| e.to_string())
    }
}

struct ActiveSessionInfo {
    session_id: String,
    close_tx: tokio::sync::oneshot::Sender<()>,
}

// ─── Global Server State ─────────────────────────────────────────────────────
struct ServerState {
    config: Config,
    terminal_state: TerminalState,
    lsp_state: LspState,
    active_session: Mutex<Option<ActiveSessionInfo>>,
}

#[tokio::main]
async fn main() {
    println!("Runyard sub-service starting...");

    // 1. Load or generate configuration
    let config_path = Path::new("subservice.toml");
    let config = if !config_path.exists() {
        let token = format!("ry_tok_{}", Uuid::new_v4().simple());
        let hashed = hash(&token, DEFAULT_COST).expect("Failed to hash token");
        let mut cfg = Config::default();
        cfg.auth_token = token;
        cfg.auth_token_hash = hashed;
        
        let toml_str = toml::to_string_pretty(&cfg).expect("Failed to serialize config");
        fs::write(config_path, toml_str).expect("Failed to write subservice.toml");
        
        println!("[Config] Generated new subservice.toml with a secure auth token.");
        cfg
    } else {
        let toml_str = fs::read_to_string(config_path).expect("Failed to read subservice.toml");
        let mut cfg: Config = toml::from_str(&toml_str).expect("Failed to parse subservice.toml");
        
        // Migrate existing config if it is missing the plaintext token field
        if cfg.auth_token.is_empty() {
            let token = format!("ry_tok_{}", Uuid::new_v4().simple());
            let hashed = hash(&token, DEFAULT_COST).expect("Failed to hash token");
            cfg.auth_token = token;
            cfg.auth_token_hash = hashed;
            
            let toml_str = toml::to_string_pretty(&cfg).expect("Failed to serialize config");
            fs::write(config_path, toml_str).expect("Failed to write subservice.toml");
            println!("[Config] Migrated subservice.toml to include the plaintext auth token.");
        }
        cfg
    };

    let port = config.port;
    let server_state = Arc::new(ServerState {
        config: config.clone(),
        terminal_state: TerminalState::default(),
        lsp_state: LspState::default(),
        active_session: Mutex::new(None),
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind to port");

    println!("[Server] Listening on http://{}", addr);
    println!("[Auth] Use this connection URL to open Runyard in your browser:");
    println!("http://localhost:{}?token={}", port, config.auth_token);

    loop {
        let (stream, client_addr) = match listener.accept().await {
            Ok(val) => val,
            Err(e) => {
                eprintln!("[Server] Accept error: {}", e);
                continue;
            }
        };

        let state_clone = server_state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, client_addr, state_clone).await {
                eprintln!("[Server] Connection error with {}: {}", client_addr, e);
            }
        });
    }
}

struct PrefixedStream {
    inner: TcpStream,
    prefix: std::io::Cursor<Vec<u8>>,
}

impl PrefixedStream {
    fn new(inner: TcpStream, prefix: Vec<u8>) -> Self {
        Self {
            inner,
            prefix: std::io::Cursor::new(prefix),
        }
    }
}

impl tokio::io::AsyncRead for PrefixedStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let pos = self.prefix.position() as usize;
        let data = self.prefix.get_ref();
        if pos < data.len() {
            let amt = std::cmp::min(data.len() - pos, buf.remaining());
            buf.put_slice(&data[pos..pos + amt]);
            self.prefix.set_position((pos + amt) as u64);
            Poll::Ready(Ok(()))
        } else {
            Pin::new(&mut self.inner).poll_read(cx, buf)
        }
    }
}

impl tokio::io::AsyncWrite for PrefixedStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

// ─── Simple HTTP & WebSocket upgrade router ──────────────────────────────────
async fn handle_connection(
    mut stream: TcpStream,
    client_addr: SocketAddr,
    state: Arc<ServerState>,
 ) -> Result<(), Box<dyn std::error::Error>> {
    let mut buf = [0u8; 4096];
    let n = stream.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut req = httparse::Request::new(&mut headers);
    let status = req.parse(&buf[..n])?;

    if status.is_partial() {
        return Ok(());
    }

    let path = req.path.unwrap_or("/");
    let method = req.method.unwrap_or("GET");

    // Check if it is a WebSocket upgrade request
    let mut is_ws = false;
    let mut auth_header = None;

    for h in req.headers.iter() {
        let name = h.name.to_lowercase();
        if name == "upgrade" && std::str::from_utf8(h.value)?.to_lowercase().contains("websocket") {
            is_ws = true;
        }
        if name == "authorization" {
            auth_header = Some(std::str::from_utf8(h.value)?);
        }
    }

    if is_ws {
        // Authenticate the request
        let mut token = "";
        
        // 1. Try Bearer header
        if let Some(auth) = auth_header {
            if auth.starts_with("Bearer ") {
                token = &auth[7..];
            }
        }

        // 2. Try query parameter
        if token.is_empty() {
            if let Some(pos) = path.find('?') {
                let query = &path[pos + 1..];
                for pair in query.split('&') {
                    let mut parts = pair.splitn(2, '=');
                    if let (Some(k), Some(v)) = (parts.next(), parts.next()) {
                        if k == "token" {
                            token = v;
                        }
                    }
                }
            }
        }

        // Verify token against hash
        if token.is_empty() || !verify(token, &state.config.auth_token_hash).unwrap_or(false) {
            eprintln!("[Auth] Rejected unauthorized connection from {}", client_addr);
            stream.write_all(b"HTTP/1.1 401 Unauthorized\r\nConnection: close\r\n\r\nUnauthorized").await?;
            return Ok(());
        }

        // Session lock check and takeover
        let session_id = Uuid::new_v4().to_string();
        let (close_tx, close_rx) = tokio::sync::oneshot::channel::<()>();

        {
            let mut active = state.active_session.lock().unwrap();
            if let Some(old_session) = active.take() {
                println!("[Auth] Terminating old active session: {}", old_session.session_id);
                let _ = old_session.close_tx.send(());
            }
            *active = Some(ActiveSessionInfo {
                session_id: session_id.clone(),
                close_tx,
            });
        }

        println!("[Server] WebSocket session started: {} for {}", session_id, client_addr);

        // Put TCP stream back together with the prefix buffer and upgrade
        let prefixed = PrefixedStream::new(stream, buf[..n].to_vec());
        let ws_stream = tokio_tungstenite::accept_async(prefixed).await?;

        // Run WebSocket session
        run_websocket_session(ws_stream, state.clone(), close_rx).await;

        // Clear session lock on disconnect, only if it is still our session
        {
            let mut active = state.active_session.lock().unwrap();
            if let Some(ref current) = *active {
                if current.session_id == session_id {
                    *active = None;
                    println!("[Server] WebSocket session ended for {}", client_addr);
                }
            }
        }

        Ok(())
    } else {
        // Handle standard HTTP asset serving
        if method != "GET" {
            stream.write_all(b"HTTP/1.1 405 Method Not Allowed\r\nConnection: close\r\n\r\n").await?;
            return Ok(());
        }

        // Strip query params
        let clean_path = match path.find('?') {
            Some(pos) => &path[..pos],
            None => path,
        };

        // Normalize path
        let mut asset_path = if clean_path == "/" || clean_path.is_empty() {
            "index.html".to_string()
        } else {
            clean_path.trim_start_matches('/').to_string()
        };

        let mut asset = Asset::get(&asset_path);
        
        // SPA Fallback: serve index.html if the resource is not found (and doesn't have an extension)
        if asset.is_none() && !asset_path.contains('.') {
            asset_path = "index.html".to_string();
            asset = Asset::get(&asset_path);
        }

        match asset {
            Some(data) => {
                let mime = mime_guess::from_path(&asset_path).first_or_octet_stream();
                let body = data.data.as_ref();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    mime,
                    body.len()
                );
                stream.write_all(response.as_bytes()).await?;
                stream.write_all(body).await?;
            }
            None => {
                stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nNot Found").await?;
            }
        }

        Ok(())
    }
}

// ─── WebSocket Event Processing Loop ─────────────────────────────────────────
async fn run_websocket_session(
    ws_stream: tokio_tungstenite::WebSocketStream<PrefixedStream>,
    state: Arc<ServerState>,
    mut close_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let (mut ws_write, mut ws_read) = ws_stream.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    // Spawn writer task
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = ws_write.send(msg).await {
                eprintln!("[WebSocket] Write error: {}", e);
                break;
            }
        }
    });

    let bridge = Arc::new(WsEventBridge { tx });

    loop {
        tokio::select! {
            _ = &mut close_rx => {
                println!("[WebSocket] Session terminated by takeover");
                break;
            }
            msg_res = ws_read.next() => {
                let msg = match msg_res {
                    Some(Ok(msg)) => msg,
                    Some(Err(e)) => {
                        eprintln!("[WebSocket] Read error: {}", e);
                        break;
                    }
                    None => break, // Connection closed
                };

                if msg.is_text() {
                    let text_str = msg.to_text().unwrap().to_string();
                    let bridge_clone = bridge.clone();
                    let state_clone = state.clone();
                    
                    // Spawn request handler in parallel task
                    tokio::spawn(async move {
                        if let Err(e) = handle_rpc_message(&text_str, bridge_clone, state_clone).await {
                            eprintln!("[WebSocket] RPC Error: {}", e);
                        }
                    });
                }
            }
        }
    }
}

// ─── JSON-RPC Dispatcher ─────────────────────────────────────────────────────
async fn handle_rpc_message(
    text: &str,
    bridge: Arc<WsEventBridge>,
    state: Arc<ServerState>,
) -> Result<(), Box<dyn std::error::Error>> {
    let request: JsonRpcRequest = match serde_json::from_str(text) {
        Ok(req) => req,
        Err(e) => {
            let err_res = JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(JsonRpcError {
                    code: -32700,
                    message: format!("Parse error: {}", e),
                    data: None,
                }),
                id: None,
            };
            bridge.tx.send(Message::Text(serde_json::to_string(&err_res)?))?;
            return Ok(());
        }
    };

    let id = request.id.clone();
    let method = request.method.as_str();
    let params = request.params.unwrap_or(Value::Null);

    let result = match method {
        // ── Filesystem ───────────────────────────────────────────────────────
        "fs_list" => {
            let p: FsListParams = serde_json::from_value(params)?;
            fs_list(p.path).map(|r| serde_json::to_value(r).unwrap())
        }
        "fs_read" => {
            let p: FsReadParams = serde_json::from_value(params)?;
            fs_read(p.path).map(|r| serde_json::to_value(r).unwrap())
        }
        "fs_write" => {
            let p: FsWriteParams = serde_json::from_value(params)?;
            fs_write(p.path, p.contents).map(|_| Value::Null)
        }
        "fs_watch" => {
            let p: FsWatchParams = serde_json::from_value(params)?;
            fs_watch_core(bridge.clone(), p.path).map(|_| Value::Null)
        }

        // ── Git ──────────────────────────────────────────────────────────────
        "git_branch" => {
            let p: GitBranchParams = serde_json::from_value(params)?;
            git_branch(p.path).map(|r| serde_json::to_value(r).unwrap())
        }
        "git_status" => {
            let p: GitStatusParams = serde_json::from_value(params)?;
            git_status(p.path).map(|r| serde_json::to_value(r).unwrap())
        }
        "git_stage" => {
            let p: GitStageParams = serde_json::from_value(params)?;
            git_stage(p.path, p.files).map(|_| Value::Null)
        }
        "git_unstage" => {
            let p: GitUnstageParams = serde_json::from_value(params)?;
            git_unstage(p.path, p.files).map(|_| Value::Null)
        }
        "git_discard" => {
            let p: GitDiscardParams = serde_json::from_value(params)?;
            git_discard(p.path, p.files).map(|_| Value::Null)
        }
        "git_commit" => {
            let p: GitCommitParams = serde_json::from_value(params)?;
            git_commit(p.path, p.message).map(|r| serde_json::to_value(r).unwrap())
        }
        "git_log" => {
            let p: GitLogParams = serde_json::from_value(params)?;
            git_log(p.path, p.limit).map(|r| serde_json::to_value(r).unwrap())
        }
        "git_branches" => {
            let p: GitBranchesParams = serde_json::from_value(params)?;
            git_branches(p.path).map(|r| serde_json::to_value(r).unwrap())
        }
        "git_checkout" => {
            let p: GitCheckoutParams = serde_json::from_value(params)?;
            git_checkout(p.path, p.branch).map(|_| Value::Null)
        }
        "git_create_branch" => {
            let p: GitCreateBranchParams = serde_json::from_value(params)?;
            git_create_branch(p.path, p.name).map(|_| Value::Null)
        }
        "git_worktrees" => {
            let p: GitWorktreesParams = serde_json::from_value(params)?;
            git_worktrees(p.path).map(|r| serde_json::to_value(r).unwrap())
        }
        "git_worktree_create" => {
            let p: GitWorktreeCreateParams = serde_json::from_value(params)?;
            git_worktree_create(p.path, p.name, p.target_path, p.branch).map(|_| Value::Null)
        }
        "git_worktree_remove" => {
            let p: GitWorktreeRemoveParams = serde_json::from_value(params)?;
            git_worktree_remove(p.path, p.name).map(|_| Value::Null)
        }

        // ── Terminal ─────────────────────────────────────────────────────────
        "terminal_create" => {
            let p: TerminalCreateParams = serde_json::from_value(params)?;
            runyard_core::terminal::terminal_create_core(
                bridge.clone(),
                &state.terminal_state,
                p.cwd,
                p.shell,
                p.cols,
                p.rows,
            )
            .map(|r| serde_json::to_value(r).unwrap())
        }
        "terminal_write" => {
            let p: TerminalWriteParams = serde_json::from_value(params)?;
            terminal_write_core(&state.terminal_state, p.id, p.data).map(|_| Value::Null)
        }
        "terminal_resize" => {
            let p: TerminalResizeParams = serde_json::from_value(params)?;
            terminal_resize_core(&state.terminal_state, p.id, p.cols, p.rows).map(|_| Value::Null)
        }
        "terminal_close" => {
            let p: TerminalCloseParams = serde_json::from_value(params)?;
            terminal_close_core(&state.terminal_state, p.id).map(|_| Value::Null)
        }
        "terminal_list" => {
            terminal_list_core(&state.terminal_state).map(|r| serde_json::to_value(r).unwrap())
        }

        // ── LSP ──────────────────────────────────────────────────────────────
        "lsp_start" => {
            let p: LspStartParams = serde_json::from_value(params)?;
            runyard_core::lsp_manager::lsp_start_core(
                bridge.clone(),
                &state.lsp_state,
                p.language,
                p.workspace_path,
                p.path_override,
            )
            .map(|r| serde_json::to_value(r).unwrap())
        }
        "lsp_send" => {
            let p: LspSendParams = serde_json::from_value(params)?;
            lsp_send_core(&state.lsp_state, p.language, p.message).map(|_| Value::Null)
        }
        "lsp_stop" => {
            let p: LspStopParams = serde_json::from_value(params)?;
            lsp_stop_core(&state.lsp_state, p.language).map(|_| Value::Null)
        }
        "lsp_status" => {
            let p: LspStatusParams = serde_json::from_value(params)?;
            lsp_status_core(&state.lsp_state, p.language).map(|r| serde_json::to_value(r).unwrap())
        }
        "lsp_status_all" => {
            lsp_status_all_core(&state.lsp_state).map(|r| serde_json::to_value(r).unwrap())
        }

        // ── Settings & Misc ──────────────────────────────────────────────────
        "settings_load" => {
            settings_load().map(|r| serde_json::to_value(r).unwrap())
        }
        "settings_save" => {
            let p: SettingsSaveParams = serde_json::from_value(params)?;
            settings_save(p.settings).map(|_| Value::Null)
        }
        "get_home_dir" => {
            get_home_dir().map(|r| serde_json::to_value(r).unwrap())
        }

        _ => Err(format!("Method not found: {}", method)),
    };

    // Return response
    let response = match result {
        Ok(res) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(res),
            error: None,
            id,
        },
        Err(err) => JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code: -32603,
                message: err,
                data: None,
            }),
            id,
        },
    };

    bridge.tx.send(Message::Text(serde_json::to_string(&response)?))?;
    Ok(())
}

// ─── JSON-RPC Parameter Structs ──────────────────────────────────────────────
#[derive(Deserialize)]
struct FsListParams {
    path: String,
}
#[derive(Deserialize)]
struct FsReadParams {
    path: String,
}
#[derive(Deserialize)]
struct FsWriteParams {
    path: String,
    contents: String,
}
#[derive(Deserialize)]
struct FsWatchParams {
    path: String,
}
#[derive(Deserialize)]
struct GitBranchParams {
    path: String,
}
#[derive(Deserialize)]
struct GitStatusParams {
    path: String,
}
#[derive(Deserialize)]
struct GitStageParams {
    path: String,
    files: Vec<String>,
}
#[derive(Deserialize)]
struct GitUnstageParams {
    path: String,
    files: Vec<String>,
}
#[derive(Deserialize)]
struct GitDiscardParams {
    path: String,
    files: Vec<String>,
}
#[derive(Deserialize)]
struct GitCommitParams {
    path: String,
    message: String,
}
#[derive(Deserialize)]
struct GitLogParams {
    path: String,
    limit: usize,
}
#[derive(Deserialize)]
struct GitBranchesParams {
    path: String,
}
#[derive(Deserialize)]
struct GitCheckoutParams {
    path: String,
    branch: String,
}
#[derive(Deserialize)]
struct GitCreateBranchParams {
    path: String,
    name: String,
}
#[derive(Deserialize)]
struct GitWorktreesParams {
    path: String,
}
#[derive(Deserialize)]
struct GitWorktreeCreateParams {
    path: String,
    name: String,
    target_path: String,
    branch: Option<String>,
}
#[derive(Deserialize)]
struct GitWorktreeRemoveParams {
    path: String,
    name: String,
}
#[derive(Deserialize)]
struct TerminalCreateParams {
    cwd: Option<String>,
    shell: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
}
#[derive(Deserialize)]
struct TerminalWriteParams {
    id: String,
    data: String,
}
#[derive(Deserialize)]
struct TerminalResizeParams {
    id: String,
    cols: u16,
    rows: u16,
}
#[derive(Deserialize)]
struct TerminalCloseParams {
    id: String,
}
#[derive(Deserialize)]
struct LspStartParams {
    language: String,
    workspace_path: String,
    path_override: Option<String>,
}
#[derive(Deserialize)]
struct LspSendParams {
    language: String,
    message: String,
}
#[derive(Deserialize)]
struct LspStopParams {
    language: String,
}
#[derive(Deserialize)]
struct LspStatusParams {
    language: String,
}
#[derive(Deserialize)]
struct SettingsSaveParams {
    settings: runyard_core::settings::RunyardSettings,
}

