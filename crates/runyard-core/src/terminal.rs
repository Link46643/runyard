use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Runtime, State};
use uuid::Uuid;
use crate::EventBridge;

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TerminalSessionInfo {
    pub id: String,
    pub cwd: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TerminalOutputEvent {
    pub id: String,
    pub data: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TerminalExitEvent {
    pub id: String,
    pub exit_code: i32,
}

// ─── Session storage ─────────────────────────────────────────────────────────

pub struct TerminalSession {
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
    pub cwd: String,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Default)]
pub struct TerminalManagerInner {
    pub sessions: HashMap<String, TerminalSession>,
}

#[derive(Default, Clone)]
pub struct TerminalState(pub Arc<Mutex<TerminalManagerInner>>);

// ─── Helper: resolve default shell ───────────────────────────────────────────

fn default_shell() -> String {
    if cfg!(target_os = "windows") {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}

// ─── Tauri commands ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn terminal_create<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, TerminalState>,
    cwd: Option<String>,
    shell: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
) -> Result<TerminalSessionInfo, String> {
    terminal_create_core(Arc::new(app), &state, cwd, shell, cols, rows)
}

pub fn terminal_create_core(
    bridge: Arc<dyn EventBridge>,
    state: &TerminalState,
    cwd: Option<String>,
    shell: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
) -> Result<TerminalSessionInfo, String> {
    let id = Uuid::new_v4().to_string();
    let terminal_cols = cols.unwrap_or(80);
    let terminal_rows = rows.unwrap_or(24);

    let cwd_path = cwd.unwrap_or_else(|| {
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string())
    });

    let shell_cmd = shell.unwrap_or_else(default_shell);

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: terminal_rows,
            cols: terminal_cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to open PTY: {}", e))?;

    // Build the shell command
    let mut cmd = CommandBuilder::new(&shell_cmd);
    cmd.cwd(&cwd_path);

    // Spawn the shell into the slave PTY
    let _child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn shell: {}", e))?;

    // Clone a reader from master (for the output-forwarding thread)
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Failed to clone PTY reader: {}", e))?;

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Failed to get PTY writer: {}", e))?;

    // Spawn output-forwarding thread
    let bridge_clone = bridge.clone();
    let id_clone = id.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    // Forward raw bytes as a lossy UTF-8 string (xterm handles encoding)
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = bridge_clone.send_event(
                        "terminal:output",
                        serde_json::json!(TerminalOutputEvent {
                            id: id_clone.clone(),
                            data,
                        }),
                    );
                }
                Err(_) => break,
            }
        }
        // Emit exit event
        let _ = bridge_clone.send_event(
            "terminal:exit",
            serde_json::json!(TerminalExitEvent {
                id: id_clone,
                exit_code: 0,
            }),
        );
    });

    // Store session
    let session = TerminalSession {
        writer,
        master: pair.master,
        cwd: cwd_path.clone(),
        cols: terminal_cols,
        rows: terminal_rows,
    };

    {
        let mut mgr = state.0.lock().unwrap();
        mgr.sessions.insert(id.clone(), session);
    }

    Ok(TerminalSessionInfo {
        id,
        cwd: cwd_path,
        cols: terminal_cols,
        rows: terminal_rows,
    })
}

#[tauri::command]
pub fn terminal_write(
    state: State<'_, TerminalState>,
    id: String,
    data: String,
) -> Result<(), String> {
    terminal_write_core(&state, id, data)
}

pub fn terminal_write_core(
    state: &TerminalState,
    id: String,
    data: String,
) -> Result<(), String> {
    let mut mgr = state.0.lock().unwrap();
    let session = mgr
        .sessions
        .get_mut(&id)
        .ok_or_else(|| format!("Terminal session '{}' not found", id))?;
    session
        .writer
        .write_all(data.as_bytes())
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn terminal_resize(
    state: State<'_, TerminalState>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    terminal_resize_core(&state, id, cols, rows)
}

pub fn terminal_resize_core(
    state: &TerminalState,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let mut mgr = state.0.lock().unwrap();
    let session = mgr
        .sessions
        .get_mut(&id)
        .ok_or_else(|| format!("Terminal session '{}' not found", id))?;
    session
        .master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;
    session.cols = cols;
    session.rows = rows;
    Ok(())
}

#[tauri::command]
pub fn terminal_close(state: State<'_, TerminalState>, id: String) -> Result<(), String> {
    terminal_close_core(&state, id)
}

pub fn terminal_close_core(state: &TerminalState, id: String) -> Result<(), String> {
    let mut mgr = state.0.lock().unwrap();
    if mgr.sessions.remove(&id).is_none() {
        // Already closed — not an error
    }
    // Dropping the session drops writer + master, causing SIGHUP to the child process
    Ok(())
}

#[tauri::command]
pub fn terminal_list(state: State<'_, TerminalState>) -> Result<Vec<TerminalSessionInfo>, String> {
    terminal_list_core(&state)
}

pub fn terminal_list_core(state: &TerminalState) -> Result<Vec<TerminalSessionInfo>, String> {
    let mgr = state.0.lock().unwrap();
    let list = mgr
        .sessions
        .iter()
        .map(|(id, s)| TerminalSessionInfo {
            id: id.clone(),
            cwd: s.cwd.clone(),
            cols: s.cols,
            rows: s.rows,
        })
        .collect::<Vec<_>>();
    Ok(list)
}
