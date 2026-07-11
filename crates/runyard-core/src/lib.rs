use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::io::Write;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use notify::{Watcher, RecursiveMode, Config};
use git2::Repository;
use tempfile::NamedTempFile;

// New M2 modules
pub mod settings;
pub mod git_ops;
pub mod terminal;
pub mod lsp_manager;
pub mod chat_db;

// Phase 1.6/1.7: ACP agent discovery & integration
pub mod acp_agent_db;
pub mod acp_bridge;
pub mod acp_registry;

// Phase 1.8-1.11: MCP config, Skills, Agent tasks, Notes/TODOs
pub mod mcp_server_db;
pub mod skills_db;
pub mod agent_tasks_db;
pub mod notes_db;

// Phase 1.6.8: OS keychain for secret env var storage
pub mod keychain;

// Workspace management (open folder, recent workspaces)
pub mod workspace;

// Agent sandbox enforcement + audit log
pub mod sandbox;


// Re-export state types for Tauri setup
pub use terminal::TerminalState;
pub use lsp_manager::LspState;
pub use acp_bridge::AcpBridgeState;

pub trait EventBridge: Send + Sync + 'static {
    fn send_event(&self, event: &str, payload: serde_json::Value) -> Result<(), String>;
}

impl<R: Runtime> EventBridge for AppHandle<R> {
    fn send_event(&self, event: &str, payload: serde_json::Value) -> Result<(), String> {
        self.emit(event, payload).map_err(|e| e.to_string())
    }
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FsEntry {
    pub name: String,
    pub path: String,
    pub kind: String, // "file" | "dir"
    pub size: u64,
}

pub mod commands {
    use super::*;

    #[tauri::command]
    pub fn fs_list(path: String) -> Result<Vec<FsEntry>, String> {
        let entries = match fs::read_dir(&path) {
            Ok(e) => e,
            Err(e) => return Err(e.to_string()),
        };
        let mut result = Vec::new();

        for entry in entries {
            if let Ok(entry) = entry {
                let (kind, size) = match entry.metadata() {
                    Ok(meta) => (if meta.is_dir() { "dir" } else { "file" }, meta.len()),
                    Err(_) => ("file", 0)
                };
                result.push(FsEntry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path: entry.path().to_string_lossy().into_owned(),
                    kind: kind.to_string(),
                    size,
                });
            }
        }
        Ok(result)
    }

    #[tauri::command]
    pub fn fs_read(path: String) -> Result<String, String> {
        println!("[Core] Reading file: {}", path);
        let bytes = fs::read(&path).map_err(|e| {
            eprintln!("[Core] Failed to read file bytes {}: {}", path, e);
            e.to_string()
        })?;

        // 1. Detect UTF-16 BOMs
        if bytes.len() >= 2 {
            if bytes[0] == 0xFF && bytes[1] == 0xFE {
                // UTF-16LE BOM
                let u16_chars: Vec<u16> = bytes[2..]
                    .chunks_exact(2)
                    .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect();
                if let Ok(content) = String::from_utf16(&u16_chars) {
                    println!("[Core] Successfully read {} characters (UTF-16LE BOM)", content.len());
                    return Ok(content);
                }
            } else if bytes[0] == 0xFE && bytes[1] == 0xFF {
                // UTF-16BE BOM
                let u16_chars: Vec<u16> = bytes[2..]
                    .chunks_exact(2)
                    .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                    .collect();
                if let Ok(content) = String::from_utf16(&u16_chars) {
                    println!("[Core] Successfully read {} characters (UTF-16BE BOM)", content.len());
                    return Ok(content);
                }
            }
        }

        // 2. Try parsing as standard UTF-8
        match String::from_utf8(bytes.clone()) {
            Ok(content) => {
                println!("[Core] Successfully read {} bytes (UTF-8)", content.len());
                Ok(content)
            }
            Err(_) => {
                // 3. Fallback: try decoding as UTF-16LE without BOM (common on Windows)
                if bytes.len() % 2 == 0 {
                    let u16_chars: Vec<u16> = bytes
                        .chunks_exact(2)
                        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                        .collect();
                    if let Ok(content) = String::from_utf16(&u16_chars) {
                        println!("[Core] Successfully decoded {} characters (UTF-16LE no BOM)", content.len());
                        return Ok(content);
                    }
                }
                // 4. Final fallback: standard lossy UTF-8 conversion
                let content = String::from_utf8_lossy(&bytes).into_owned();
                println!("[Core] Decoded {} bytes lossily (invalid UTF-8)", content.len());
                Ok(content)
            }
        }
    }

    #[tauri::command]
    pub fn fs_write(path: String, contents: String) -> Result<(), String> {
        let path_buf = Path::new(&path);
        let dir = path_buf.parent().ok_or("Invalid path")?;

        let mut temp = NamedTempFile::new_in(dir).map_err(|e| e.to_string())?;
        temp.write_all(contents.as_bytes()).map_err(|e| e.to_string())?;
        temp.persist(path_buf).map_err(|e| e.to_string())?;

        Ok(())
    }

    #[tauri::command]
    pub fn fs_create_dir(path: String) -> Result<(), String> {
        println!("[Core] Creating directory: {}", path);
        std::fs::create_dir_all(&path).map_err(|e| {
            eprintln!("[Core] Failed to create directory {}: {}", path, e);
            e.to_string()
        })
    }

    #[tauri::command]
    pub fn fs_watch<R: Runtime>(app: AppHandle<R>, path: String) -> Result<(), String> {
        fs_watch_core(Arc::new(app), path)
    }

    pub fn fs_watch_core(bridge: Arc<dyn EventBridge>, path: String) -> Result<(), String> {
        let path_clone = path.clone();

        std::thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = notify::RecommendedWatcher::new(tx, Config::default()).unwrap();

            watcher.watch(Path::new(&path_clone), RecursiveMode::Recursive).unwrap();

            for res in rx {
                match res {
                    Ok(event) => {
                        for path in event.paths {
                            let _ = bridge.send_event("fs:changed", serde_json::json!(path.to_string_lossy().to_string()));
                        }
                    }
                    Err(e) => println!("watch error: {:?}", e),
                }
            }
        });

        Ok(())
    }

    #[tauri::command]
    pub fn git_branch(path: String) -> Result<Option<String>, String> {
        let repo = Repository::discover(path).map_err(|e| e.to_string())?;
        let head = repo.head().map_err(|e| e.to_string())?;
        let branch = head.shorthand().map(|s| s.to_string());
        Ok(branch)
    }

    #[tauri::command]
    pub fn get_home_dir() -> Result<String, String> {
        let path = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE")
                .or_else(|_| {
                    let drive = std::env::var("HOMEDRIVE").unwrap_or_default();
                    let path = std::env::var("HOMEPATH").unwrap_or_default();
                    if drive.is_empty() || path.is_empty() {
                        Err("No home".to_string())
                    } else {
                        Ok(format!("{}{}", drive, path))
                    }
                })
                .map_err(|e| e.to_string())?
        } else {
            std::env::var("HOME")
                .map_err(|e| e.to_string())?
        };

        let p = std::path::PathBuf::from(path);
        if p.exists() && p.is_dir() {
            Ok(p.to_string_lossy().to_string())
        } else {
            Err("Home directory not found or invalid".to_string())
        }
    }

    #[tauri::command]
    pub fn ssh_bootstrap(host: String, port: u16, username: String, _token: String) -> Result<String, String> {
        use std::net::TcpStream;
        use ssh2::Session;

        let tcp = TcpStream::connect(format!("{}:{}", host, port)).map_err(|e| e.to_string())?;
        let mut sess = Session::new().map_err(|e| e.to_string())?;
        sess.set_tcp_stream(tcp);
        sess.handshake().map_err(|e| e.to_string())?;

        // Authenticate agent or password
        if let Err(_) = sess.userauth_agent(&username) {
            return Err("Authentication failed via ssh-agent. Please ensure key is loaded.".to_string());
        }

        let mut channel = sess.channel_session().map_err(|e| e.to_string())?;
        channel.exec("curl -L https://github.com/Link46643/runyard/releases/latest/download/subservice -o subservice && chmod +x subservice && ./subservice &").map_err(|e| e.to_string())?;
        
        Ok("Subservice bootstrap command executed successfully.".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::commands::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_fs_list() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        File::create(&file_path).unwrap();

        let results = fs_list(dir.path().to_str().unwrap().to_string()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "test.txt");
        assert_eq!(results[0].kind, "file");
    }

    #[test]
    fn test_fs_read_write() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("write_test.txt");
        let path_str = file_path.to_str().unwrap().to_string();
        let content = "hello runyard";

        fs_write(path_str.clone(), content.to_string()).unwrap();
        let read_content = fs_read(path_str).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_git_branch() {
        let result = git_branch("../../".to_string());
        assert!(result.is_ok());
        let branch = result.unwrap();
        assert!(branch.is_some());
        println!("Current branch: {:?}", branch);
    }
}
