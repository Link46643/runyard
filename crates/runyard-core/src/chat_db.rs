use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbConversation {
    pub id: String,
    pub title: String,
    pub workspace_path: String,
    pub model: String,
    pub provider: String,
    pub system_prompt: Option<String>,
    pub context_budget: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub message_count: i64,
    pub total_tokens_used: i64,
    pub total_cost: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbMessage {
    pub id: String,
    pub conversation_id: String,
    pub parent_id: Option<String>,
    pub role: String,
    pub content: Value,
    pub created_at: i64,
    pub is_pinned: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbBranch {
    pub id: String,
    pub conversation_id: String,
    pub name: String,
    pub message_id: String,
    pub created_at: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DbPinnedContext {
    pub id: String,
    pub conversation_id: String,
    pub file_path: String,
    pub created_at: i64,
}

pub fn get_db_path() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
    path.push(".runyard");
    let _ = fs::create_dir_all(&path);
    path.push("chat.db");
    path
}

pub fn get_media_dir() -> PathBuf {
    let mut path = dirs::home_dir().unwrap_or_else(|| std::env::current_dir().unwrap());
    path.push(".runyard");
    path.push("media");
    let _ = fs::create_dir_all(&path);
    path
}

pub fn init_db() -> SqlResult<()> {
    let conn = Connection::open(get_db_path())?;
    
    // Enable WAL mode — pragma_update is required here because
    // PRAGMA journal_mode returns a result row; conn.execute() would throw ExecuteReturnedResults.
    conn.pragma_update(None, "journal_mode", "WAL")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS conversations (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            workspace_path TEXT NOT NULL DEFAULT '',
            model TEXT NOT NULL,
            provider TEXT NOT NULL DEFAULT '',
            system_prompt TEXT,
            context_budget INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            message_count INTEGER DEFAULT 0,
            total_tokens_used INTEGER DEFAULT 0,
            total_cost REAL DEFAULT 0.0
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            parent_id TEXT,
            role TEXT NOT NULL,
            content BLOB NOT NULL,
            created_at INTEGER NOT NULL,
            is_pinned INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS branches (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            name TEXT NOT NULL,
            message_id TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS pinned_context (
            id TEXT PRIMARY KEY,
            conversation_id TEXT NOT NULL,
            file_path TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Create FTS5 virtual table
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(message_id UNINDEXED, content_text)",
        [],
    )?;

    // Run migrations to add columns in case the table existed previously
    run_migrations(&conn)?;

    Ok(())
}

fn run_migrations(conn: &Connection) -> SqlResult<()> {
    let _ = conn.execute("ALTER TABLE conversations ADD COLUMN workspace_path TEXT NOT NULL DEFAULT ''", []);
    let _ = conn.execute("ALTER TABLE conversations ADD COLUMN provider TEXT NOT NULL DEFAULT ''", []);
    let _ = conn.execute("ALTER TABLE conversations ADD COLUMN system_prompt TEXT", []);
    let _ = conn.execute("ALTER TABLE conversations ADD COLUMN context_budget INTEGER NOT NULL DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE conversations ADD COLUMN message_count INTEGER DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE conversations ADD COLUMN total_tokens_used INTEGER DEFAULT 0", []);
    let _ = conn.execute("ALTER TABLE conversations ADD COLUMN total_cost REAL DEFAULT 0.0", []);
    let _ = conn.execute("ALTER TABLE messages ADD COLUMN is_pinned INTEGER NOT NULL DEFAULT 0", []);
    Ok(())
}

// ── Compression Helpers ──────────────────────────────────────────────────────

fn compress_content(text: &str) -> Vec<u8> {
    zstd::encode_all(text.as_bytes(), 3).unwrap_or_else(|_| text.as_bytes().to_vec())
}

fn decompress_content(bytes: &[u8]) -> Result<String, String> {
    if let Ok(decompressed) = zstd::decode_all(bytes) {
        if let Ok(s) = String::from_utf8(decompressed) {
            return Ok(s);
        }
    }
    String::from_utf8(bytes.to_vec()).map_err(|e| e.to_string())
}

// ── Image Handling ───────────────────────────────────────────────────────────

fn process_images_in_value(val: &mut Value) {
    match val {
        Value::String(s) => {
            if s.starts_with("data:image/") && s.contains(";base64,") {
                if let Some(comma_pos) = s.find(";base64,") {
                    let mime = &s[5..comma_pos];
                    let ext = match mime {
                        "image/png" => "png",
                        "image/jpeg" => "jpg",
                        "image/gif" => "gif",
                        "image/webp" => "webp",
                        _ => "png",
                    };
                    let base64_data = &s[comma_pos + 8..];
                    use base64::{Engine as _, engine::general_purpose};
                    if let Ok(bytes) = general_purpose::STANDARD.decode(base64_data) {
                        let filename = format!("{}.{}", Uuid::new_v4(), ext);
                        let mut filepath = get_media_dir();
                        filepath.push(&filename);
                        if fs::write(&filepath, bytes).is_ok() {
                            *s = format!("media://{}", filename);
                        }
                    }
                }
            }
        }
        Value::Array(arr) => {
            for v in arr {
                process_images_in_value(v);
            }
        }
        Value::Object(obj) => {
            for (_, v) in obj {
                process_images_in_value(v);
            }
        }
        _ => {}
    }
}

// ── FTS Parsing Helper ────────────────────────────────────────────────────────

fn extract_text_from_content(content: &Value) -> String {
    let mut text = String::new();
    if let Some(arr) = content.as_array() {
        for block in arr {
            if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                text.push_str(t);
                text.push(' ');
            }
            if let Some(c) = block.get("code").and_then(|v| v.as_str()) {
                text.push_str(c);
                text.push(' ');
            }
            if let Some(th) = block.get("thought").and_then(|v| v.as_str()) {
                text.push_str(th);
                text.push(' ');
            }
        }
    } else if let Some(t) = content.get("text").and_then(|v| v.as_str()) {
        text.push_str(t);
    }
    text
}

// ── Conversation Commands ────────────────────────────────────────────────────

#[tauri::command]
pub fn chat_conversation_list() -> Result<Vec<DbConversation>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, title, workspace_path, model, provider, system_prompt, context_budget, created_at, updated_at, message_count, total_tokens_used, total_cost FROM conversations ORDER BY updated_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok(DbConversation {
            id: row.get(0)?,
            title: row.get(1)?,
            workspace_path: row.get(2)?,
            model: row.get(3)?,
            provider: row.get(4)?,
            system_prompt: row.get(5)?,
            context_budget: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
            message_count: row.get(9)?,
            total_tokens_used: row.get(10)?,
            total_cost: row.get(11)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        if let Ok(conv) = r {
            list.push(conv);
        }
    }
    Ok(list)
}

#[tauri::command]
pub fn chat_conversation_create(
    title: String,
    model: String,
    workspace_path: Option<String>,
    provider: Option<String>,
    system_prompt: Option<String>,
    context_budget: Option<i64>,
) -> Result<DbConversation, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();
    let ws_path = workspace_path.unwrap_or_default();
    let prov = provider.unwrap_or_default();
    let budget = context_budget.unwrap_or(0);

    conn.execute(
        "INSERT INTO conversations (id, title, workspace_path, model, provider, system_prompt, context_budget, created_at, updated_at, message_count, total_tokens_used, total_cost) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, 0, 0.0)",
        params![id, title, ws_path, model, prov, system_prompt, budget, now, now],
    ).map_err(|e| e.to_string())?;

    Ok(DbConversation {
        id,
        title,
        workspace_path: ws_path,
        model,
        provider: prov,
        system_prompt,
        context_budget: budget,
        created_at: now,
        updated_at: now,
        message_count: 0,
        total_tokens_used: 0,
        total_cost: 0.0,
    })
}

#[tauri::command]
pub fn chat_conversation_update(
    id: String,
    title: Option<String>,
    model: Option<String>,
    provider: Option<String>,
    system_prompt: Option<String>,
    context_budget: Option<i64>,
    workspace_path: Option<String>,
) -> Result<DbConversation, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    
    if let Some(t) = title {
        conn.execute("UPDATE conversations SET title = ?1, updated_at = ?2 WHERE id = ?3", params![t, now, id]).map_err(|e| e.to_string())?;
    }
    if let Some(m) = model {
        conn.execute("UPDATE conversations SET model = ?1, updated_at = ?2 WHERE id = ?3", params![m, now, id]).map_err(|e| e.to_string())?;
    }
    if let Some(p) = provider {
        conn.execute("UPDATE conversations SET provider = ?1, updated_at = ?2 WHERE id = ?3", params![p, now, id]).map_err(|e| e.to_string())?;
    }
    if let Some(sp) = system_prompt {
        conn.execute("UPDATE conversations SET system_prompt = ?1, updated_at = ?2 WHERE id = ?3", params![sp, now, id]).map_err(|e| e.to_string())?;
    }
    if let Some(cb) = context_budget {
        conn.execute("UPDATE conversations SET context_budget = ?1, updated_at = ?2 WHERE id = ?3", params![cb, now, id]).map_err(|e| e.to_string())?;
    }
    if let Some(wp) = workspace_path {
        conn.execute("UPDATE conversations SET workspace_path = ?1, updated_at = ?2 WHERE id = ?3", params![wp, now, id]).map_err(|e| e.to_string())?;
    }

    let mut stmt = conn.prepare("SELECT id, title, workspace_path, model, provider, system_prompt, context_budget, created_at, updated_at, message_count, total_tokens_used, total_cost FROM conversations WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let conv = stmt.query_row(params![id], |row| {
        Ok(DbConversation {
            id: row.get(0)?,
            title: row.get(1)?,
            workspace_path: row.get(2)?,
            model: row.get(3)?,
            provider: row.get(4)?,
            system_prompt: row.get(5)?,
            context_budget: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
            message_count: row.get(9)?,
            total_tokens_used: row.get(10)?,
            total_cost: row.get(11)?,
        })
    }).map_err(|e| e.to_string())?;

    Ok(conv)
}

#[tauri::command]
pub fn chat_conversation_delete(id: String) -> Result<(), String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM conversations WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Message Commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn chat_messages_load(conversation_id: String, page: Option<u32>, limit: Option<u32>) -> Result<Vec<DbMessage>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let mut query_str = "SELECT id, conversation_id, parent_id, role, content, created_at, is_pinned FROM messages WHERE conversation_id = ?1 ORDER BY created_at ASC".to_string();
    let mut params_vec: Vec<rusqlite::types::Value> = vec![conversation_id.into()];

    if let Some(lim) = limit {
        query_str.push_str(" LIMIT ?2");
        params_vec.push((lim as i64).into());
        if let Some(p) = page {
            let offset = (p.saturating_sub(1) * lim) as i64;
            query_str.push_str(" OFFSET ?3");
            params_vec.push(offset.into());
        }
    }

    let mut stmt = conn.prepare(&query_str).map_err(|e| e.to_string())?;
    let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|v| v as &dyn rusqlite::ToSql).collect();

    let rows = stmt.query_map(&*params_refs, |row| {
        let content_bytes: Vec<u8> = row.get(4)?;
        let content_str = decompress_content(&content_bytes).unwrap_or_else(|_| String::new());
        let content_val = serde_json::from_str(&content_str).unwrap_or(Value::Null);
        Ok(DbMessage {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            parent_id: row.get(2)?,
            role: row.get(3)?,
            content: content_val,
            created_at: row.get(5)?,
            is_pinned: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        if let Ok(msg) = r {
            list.push(msg);
        }
    }
    Ok(list)
}

#[tauri::command]
pub fn chat_message_insert(conversation_id: String, parent_id: Option<String>, role: String, mut content: Value) -> Result<DbMessage, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().timestamp_millis();
    let id = Uuid::new_v4().to_string();

    process_images_in_value(&mut content);
    let content_str = serde_json::to_string(&content).map_err(|e| e.to_string())?;
    let content_bytes = compress_content(&content_str);

    conn.execute(
        "INSERT INTO messages (id, conversation_id, parent_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![id, conversation_id, parent_id, role, content_bytes, now],
    ).map_err(|e| e.to_string())?;

    let content_text = extract_text_from_content(&content);
    conn.execute(
        "INSERT INTO messages_fts (message_id, content_text) VALUES (?1, ?2)",
        params![id, content_text],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE conversations SET updated_at = ?1, message_count = message_count + 1 WHERE id = ?2",
        params![now, conversation_id],
    ).map_err(|e| e.to_string())?;

    Ok(DbMessage {
        id,
        conversation_id,
        parent_id,
        role,
        content,
        created_at: now,
        is_pinned: false,
    })
}

#[tauri::command]
pub fn chat_message_update(id: String, mut content: Value) -> Result<DbMessage, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    process_images_in_value(&mut content);
    let content_str = serde_json::to_string(&content).map_err(|e| e.to_string())?;
    let content_bytes = compress_content(&content_str);
    let content_text = extract_text_from_content(&content);

    conn.execute(
        "UPDATE messages SET content = ?1 WHERE id = ?2",
        params![content_bytes, id],
    ).map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE messages_fts SET content_text = ?1 WHERE message_id = ?2",
        params![content_text, id],
    ).map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare("SELECT id, conversation_id, parent_id, role, content, created_at, is_pinned FROM messages WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let msg = stmt.query_row(params![id], |row| {
        let content_bytes: Vec<u8> = row.get(4)?;
        let content_str = decompress_content(&content_bytes).unwrap_or_else(|_| String::new());
        let content_val = serde_json::from_str(&content_str).unwrap_or(Value::Null);
        Ok(DbMessage {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            parent_id: row.get(2)?,
            role: row.get(3)?,
            content: content_val,
            created_at: row.get(5)?,
            is_pinned: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?;

    Ok(msg)
}

#[tauri::command]
pub fn chat_message_set_pinned(id: String, is_pinned: bool) -> Result<DbMessage, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE messages SET is_pinned = ?1 WHERE id = ?2",
        params![is_pinned, id],
    ).map_err(|e| e.to_string())?;

    let mut stmt = conn.prepare("SELECT id, conversation_id, parent_id, role, content, created_at, is_pinned FROM messages WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let msg = stmt.query_row(params![id], |row| {
        let content_bytes: Vec<u8> = row.get(4)?;
        let content_str = decompress_content(&content_bytes).unwrap_or_else(|_| String::new());
        let content_val = serde_json::from_str(&content_str).unwrap_or(Value::Null);
        Ok(DbMessage {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            parent_id: row.get(2)?,
            role: row.get(3)?,
            content: content_val,
            created_at: row.get(5)?,
            is_pinned: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?;

    Ok(msg)
}

#[tauri::command]
pub fn chat_search(query: String) -> Result<Vec<DbMessage>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "SELECT m.id, m.conversation_id, m.parent_id, m.role, m.content, m.created_at, m.is_pinned
         FROM messages m
         JOIN messages_fts fts ON m.id = fts.message_id
         WHERE fts.content_text MATCH ?1
         ORDER BY m.created_at DESC"
    ).map_err(|e| e.to_string())?;

    let rows = stmt.query_map(params![query], |row| {
        let content_bytes: Vec<u8> = row.get(4)?;
        let content_str = decompress_content(&content_bytes).unwrap_or_else(|_| String::new());
        let content_val = serde_json::from_str(&content_str).unwrap_or(Value::Null);
        Ok(DbMessage {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            parent_id: row.get(2)?,
            role: row.get(3)?,
            content: content_val,
            created_at: row.get(5)?,
            is_pinned: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        if let Ok(msg) = r {
            list.push(msg);
        }
    }
    Ok(list)
}

// ── Branch Commands ──────────────────────────────────────────────────────────

#[tauri::command]
pub fn chat_branch_create(conversation_id: String, name: String, message_id: String) -> Result<DbBranch, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "INSERT INTO branches (id, conversation_id, name, message_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, conversation_id, name, message_id, now],
    ).map_err(|e| e.to_string())?;

    Ok(DbBranch { id, conversation_id, name, message_id, created_at: now })
}

#[tauri::command]
pub fn chat_branch_list(conversation_id: String) -> Result<Vec<DbBranch>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, conversation_id, name, message_id, created_at FROM branches WHERE conversation_id = ?1 ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![conversation_id], |row| {
        Ok(DbBranch {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            name: row.get(2)?,
            message_id: row.get(3)?,
            created_at: row.get(4)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        if let Ok(b) = r {
            list.push(b);
        }
    }
    Ok(list)
}

#[tauri::command]
pub fn chat_branch_delete(id: String) -> Result<(), String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM branches WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}

// ── Pinned Context Commands ──────────────────────────────────────────────────

#[tauri::command]
pub fn chat_pinned_context_load(conversation_id: String) -> Result<Vec<DbPinnedContext>, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, conversation_id, file_path, created_at FROM pinned_context WHERE conversation_id = ?1 ORDER BY created_at ASC")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map(params![conversation_id], |row| {
        Ok(DbPinnedContext {
            id: row.get(0)?,
            conversation_id: row.get(1)?,
            file_path: row.get(2)?,
            created_at: row.get(3)?,
        })
    }).map_err(|e| e.to_string())?;

    let mut list = Vec::new();
    for r in rows {
        if let Ok(pc) = r {
            list.push(pc);
        }
    }
    Ok(list)
}

#[tauri::command]
pub fn chat_pinned_context_save(conversation_id: String, file_path: String) -> Result<DbPinnedContext, String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    conn.execute(
        "INSERT INTO pinned_context (id, conversation_id, file_path, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, conversation_id, file_path, now],
    ).map_err(|e| e.to_string())?;

    Ok(DbPinnedContext { id, conversation_id, file_path, created_at: now })
}

#[tauri::command]
pub fn chat_pinned_context_delete(id: String) -> Result<(), String> {
    let conn = Connection::open(get_db_path()).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM pinned_context WHERE id = ?1", params![id]).map_err(|e| e.to_string())?;
    Ok(())
}
