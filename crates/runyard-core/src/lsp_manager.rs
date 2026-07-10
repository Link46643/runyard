use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Runtime, State};
use crate::EventBridge;

// ─── Types ───────────────────────────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum LspStatusKind {
    Disconnected,
    Starting,
    Ready,
    Error,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LspServerStatus {
    pub language: String,
    pub status: LspStatusKind,
    pub error: Option<String>,
    pub executable: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LspMessageEvent {
    pub language: String,
    pub message: Value,
}

// ─── Server storage ───────────────────────────────────────────────────────────

pub struct LspServer {
    pub language: String,
    pub stdin: ChildStdin,
    pub status: LspStatusKind,
    pub executable: String,
}

#[derive(Default)]
pub struct LspManagerInner {
    pub servers: HashMap<String, LspServer>, // keyed by language
}

#[derive(Default, Clone)]
pub struct LspState(pub Arc<Mutex<LspManagerInner>>);

// ─── Known language server executables ───────────────────────────────────────

/// Returns the list of candidate executable names for a language, in priority order.
fn candidates_for_language(language: &str) -> Vec<&'static str> {
    match language {
        "typescript" | "javascript" => vec!["typescript-language-server"],
        "python" => vec!["basedpyright-langserver", "pyright-langserver", "pylsp"],
        "rust" => vec!["rust-analyzer"],
        "go" => vec!["gopls"],
        _ => vec![],
    }
}

/// Find the first available language server for a language, checking PATH.
fn find_language_server(
    language: &str,
    path_override: Option<&str>,
) -> Option<String> {
    if let Some(override_path) = path_override {
        if std::path::Path::new(override_path).exists() {
            return Some(override_path.to_string());
        }
    }
    for candidate in candidates_for_language(language) {
        if which::which(candidate).is_ok() {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Build spawn args for a language server (some need `--stdio`)
fn server_args(executable: &str) -> Vec<&'static str> {
    if executable.contains("typescript-language-server")
        || executable.contains("pyright")
        || executable.contains("basedpyright")
    {
        vec!["--stdio"]
    } else {
        // rust-analyzer and gopls speak stdio by default
        vec![]
    }
}

// ─── JSON-RPC framing helpers ─────────────────────────────────────────────────

pub fn write_lsp_message(stdin: &mut ChildStdin, message: &str) -> std::io::Result<()> {
    let header = format!("Content-Length: {}\r\n\r\n", message.len());
    stdin.write_all(header.as_bytes())?;
    stdin.write_all(message.as_bytes())?;
    stdin.flush()
}

fn read_lsp_message(reader: &mut impl BufRead) -> Option<String> {
    let mut content_length: Option<usize> = None;

    // Read headers
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        if let Some(rest) = line.strip_prefix("Content-Length: ") {
            content_length = rest.trim().parse().ok();
        }
    }

    let len = content_length?;
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

// ─── Tauri commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn lsp_start<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, LspState>,
    language: String,
    workspace_path: String,
    path_override: Option<String>,
) -> Result<LspServerStatus, String> {
    lsp_start_core(Arc::new(app), &state, language, workspace_path, path_override)
}

pub fn lsp_start_core(
    bridge: Arc<dyn EventBridge>,
    state: &LspState,
    language: String,
    workspace_path: String,
    path_override: Option<String>,
) -> Result<LspServerStatus, String> {
    // Check if already running
    {
        let mgr = state.0.lock().unwrap();
        if let Some(srv) = mgr.servers.get(&language) {
            return Ok(LspServerStatus {
                language: language.clone(),
                status: srv.status.clone(),
                error: None,
                executable: Some(srv.executable.clone()),
            });
        }
    }

    let executable = match find_language_server(&language, path_override.as_deref()) {
        Some(exe) => exe,
        None => {
            return Ok(LspServerStatus {
                language: language.clone(),
                status: LspStatusKind::Error,
                error: Some(format!("No language server found for '{}'", language)),
                executable: None,
            });
        }
    };

    let args = server_args(&executable);
    let mut cmd = Command::new(&executable);
    cmd.args(&args)
        .current_dir(&workspace_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null()); // suppress LSP server stderr noise

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn '{}': {}", executable, e))?;

    let stdin = child.stdin.take().ok_or("Failed to get stdin")?;
    let stdout = child.stdout.take().ok_or("Failed to get stdout")?;

    // Spawn reader thread
    let bridge_clone = bridge.clone();
    let lang_clone = language.clone();
    let state_clone = state.0.clone();
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_lsp_message(&mut reader) {
                Some(msg_str) => {
                    if let Ok(json) = serde_json::from_str::<Value>(&msg_str) {
                        // Mark server as ready on first response
                        {
                            let mut mgr = state_clone.lock().unwrap();
                            if let Some(srv) = mgr.servers.get_mut(&lang_clone) {
                                if srv.status == LspStatusKind::Starting {
                                    srv.status = LspStatusKind::Ready;
                                }
                            }
                        }
                        let _ = bridge_clone.send_event(
                            "lsp:message",
                            serde_json::json!(LspMessageEvent {
                                language: lang_clone.clone(),
                                message: json,
                            }),
                        );
                    }
                }
                None => break, // server closed stdout
            }
        }
        // Server exited — update status
        let mut mgr = state_clone.lock().unwrap();
        mgr.servers.remove(&lang_clone);
    });

    let srv = LspServer {
        language: language.clone(),
        stdin,
        status: LspStatusKind::Starting,
        executable: executable.clone(),
    };

    {
        let mut mgr = state.0.lock().unwrap();
        mgr.servers.insert(language.clone(), srv);
    }

    Ok(LspServerStatus {
        language: language.clone(),
        status: LspStatusKind::Starting,
        error: None,
        executable: Some(executable),
    })
}

#[tauri::command]
pub fn lsp_send(
    state: State<'_, LspState>,
    language: String,
    message: String,
) -> Result<(), String> {
    lsp_send_core(&state, language, message)
}

pub fn lsp_send_core(
    state: &LspState,
    language: String,
    message: String,
) -> Result<(), String> {
    let mut mgr = state.0.lock().unwrap();
    let srv = mgr
        .servers
        .get_mut(&language)
        .ok_or_else(|| format!("LSP server '{}' not running", language))?;
    write_lsp_message(&mut srv.stdin, &message).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn lsp_stop(state: State<'_, LspState>, language: String) -> Result<(), String> {
    lsp_stop_core(&state, language)
}

pub fn lsp_stop_core(state: &LspState, language: String) -> Result<(), String> {
    let mut mgr = state.0.lock().unwrap();
    mgr.servers.remove(&language);
    // Dropping the LspServer drops stdin, which sends EOF to the server process
    Ok(())
}

#[tauri::command]
pub fn lsp_status(
    state: State<'_, LspState>,
    language: String,
) -> Result<LspServerStatus, String> {
    lsp_status_core(&state, language)
}

pub fn lsp_status_core(
    state: &LspState,
    language: String,
) -> Result<LspServerStatus, String> {
    let mgr = state.0.lock().unwrap();
    match mgr.servers.get(&language) {
        Some(srv) => Ok(LspServerStatus {
            language: language,
            status: srv.status.clone(),
            error: None,
            executable: Some(srv.executable.clone()),
        }),
        None => Ok(LspServerStatus {
            language,
            status: LspStatusKind::Disconnected,
            error: None,
            executable: None,
        }),
    }
}

#[tauri::command]
pub fn lsp_status_all(state: State<'_, LspState>) -> Result<Vec<LspServerStatus>, String> {
    lsp_status_all_core(&state)
}

pub fn lsp_status_all_core(state: &LspState) -> Result<Vec<LspServerStatus>, String> {
    let mgr = state.0.lock().unwrap();
    let statuses = mgr
        .servers
        .iter()
        .map(|(lang, srv)| LspServerStatus {
            language: lang.clone(),
            status: srv.status.clone(),
            error: None,
            executable: Some(srv.executable.clone()),
        })
        .collect();
    Ok(statuses)
}
