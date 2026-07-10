use runyard_core::{TerminalState, LspState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            // ── Misc ────────────────────────────────────────────────────
            runyard_core::commands::get_home_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
