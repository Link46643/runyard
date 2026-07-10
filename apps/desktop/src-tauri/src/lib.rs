use runyard_core::{TerminalState, LspState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Initialize SQLite database — runs after Tauri runtime is ready.
            // Panic on failure so errors are never silently swallowed.
            runyard_core::chat_db::init_db()
                .expect("[Tauri] FATAL: Failed to initialize SQLite chat.db");
            // Initialize ACP agent registry tables (Phase 1.6.1).
            {
                let conn = rusqlite::Connection::open(runyard_core::chat_db::get_db_path())
                    .expect("[Tauri] FATAL: Failed to open chat.db for ACP agent table init");
                runyard_core::acp_agent_db::init_acp_agent_tables(&conn)
                    .expect("[Tauri] FATAL: Failed to initialize ACP agent tables");
            }
            // ACP connection pool + event-forwarding task (Phase 1.6/1.7).
            // Needs a real AppHandle to emit events, so it's set up here
            // rather than via a plain .manage(Default::default()) call.
            app.manage(runyard_core::acp_bridge::init_acp_bridge(app.handle()));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        // Managed state: terminal sessions and LSP servers
        .manage(TerminalState::default())
        .manage(LspState::default())
        .invoke_handler(tauri::generate_handler![
            // ── Filesystem ──────────────────────────────────────────────────
            runyard_core::commands::fs_list,
            runyard_core::commands::fs_read,
            runyard_core::commands::fs_write,
            runyard_core::commands::fs_watch,
            // ── Git (legacy single-command) ──────────────────────────────
            runyard_core::commands::git_branch,
            // ── Git (M2 full operations) ─────────────────────────────────
            runyard_core::git_ops::git_status,
            runyard_core::git_ops::git_stage,
            runyard_core::git_ops::git_unstage,
            runyard_core::git_ops::git_discard,
            runyard_core::git_ops::git_commit,
            runyard_core::git_ops::git_log,
            runyard_core::git_ops::git_branches,
            runyard_core::git_ops::git_checkout,
            runyard_core::git_ops::git_create_branch,
            runyard_core::git_ops::git_worktrees,
            runyard_core::git_ops::git_worktree_create,
            runyard_core::git_ops::git_worktree_remove,
            // ── Settings ────────────────────────────────────────────────
            runyard_core::settings::settings_load,
            runyard_core::settings::settings_save,
            // ── Terminal ────────────────────────────────────────────────
            runyard_core::terminal::terminal_create,
            runyard_core::terminal::terminal_write,
            runyard_core::terminal::terminal_resize,
            runyard_core::terminal::terminal_close,
            runyard_core::terminal::terminal_list,
            // ── LSP ─────────────────────────────────────────────────────
            runyard_core::lsp_manager::lsp_start,
            runyard_core::lsp_manager::lsp_send,
            runyard_core::lsp_manager::lsp_stop,
            runyard_core::lsp_manager::lsp_status,
            runyard_core::lsp_manager::lsp_status_all,
            // ── Chat Database ───────────────────────────────────────────
            runyard_core::chat_db::chat_conversation_list,
            runyard_core::chat_db::chat_conversation_create,
            runyard_core::chat_db::chat_conversation_update,
            runyard_core::chat_db::chat_conversation_delete,
            runyard_core::chat_db::chat_messages_load,
            runyard_core::chat_db::chat_message_insert,
            runyard_core::chat_db::chat_message_update,
            runyard_core::chat_db::chat_message_set_pinned,
            runyard_core::chat_db::chat_message_delete,
            runyard_core::chat_db::chat_search,
            runyard_core::chat_db::chat_branch_create,
            runyard_core::chat_db::chat_branch_list,
            runyard_core::chat_db::chat_branch_delete,
            runyard_core::chat_db::chat_pinned_context_load,
            runyard_core::chat_db::chat_pinned_context_save,
            runyard_core::chat_db::chat_pinned_context_delete,
            // ── Misc ────────────────────────────────────────────────────
            runyard_core::commands::get_home_dir,
            runyard_core::commands::ssh_bootstrap,
            // ── ACP Agent Registry (1.6.1, 1.6.10) ───────────────────────
            runyard_core::acp_agent_db::acp_agent_list,
            runyard_core::acp_agent_db::acp_agent_get,
            runyard_core::acp_agent_db::acp_agent_create,
            runyard_core::acp_agent_db::acp_agent_update,
            runyard_core::acp_agent_db::acp_agent_delete,
            runyard_core::acp_agent_db::acp_agent_set_active,
            runyard_core::acp_agent_db::acp_agent_set_status,
            runyard_core::acp_agent_db::acp_agent_set_capabilities,
            runyard_core::acp_agent_db::acp_agent_set_default_for_project,
            runyard_core::acp_agent_db::acp_agent_get_default_for_project,
            runyard_core::acp_agent_db::acp_agent_export,
            runyard_core::acp_agent_db::acp_agent_import,
            runyard_core::acp_agent_db::acp_agent_discover,
            // ── ACP Registry API (1.6.2) ─────────────────────────────────
            runyard_core::acp_registry::acp_agent_fetch_registry,
            // ── ACP Bridge: connections + sessions (1.6.4-6, 1.7.6-14) ───
            runyard_core::acp_bridge::acp_connect,
            runyard_core::acp_bridge::acp_disconnect,
            runyard_core::acp_bridge::acp_list_connections,
            runyard_core::acp_bridge::acp_new_session,
            runyard_core::acp_bridge::acp_load_session,
            runyard_core::acp_bridge::acp_resume_session,
            runyard_core::acp_bridge::acp_list_sessions,
            runyard_core::acp_bridge::acp_close_session,
            runyard_core::acp_bridge::acp_send_prompt,
            runyard_core::acp_bridge::acp_cancel,
            runyard_core::acp_bridge::acp_respond_permission,
            runyard_core::acp_bridge::acp_set_mode,
            runyard_core::acp_bridge::acp_set_config_option,
            runyard_core::acp_bridge::acp_authenticate,
            runyard_core::acp_bridge::acp_logout,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
