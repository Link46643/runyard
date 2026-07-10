// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use runyard_core::{LspState, TerminalState};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Register managed state for terminal and LSP
        .manage(TerminalState::default())
        .manage(LspState::default())
        .invoke_handler(tauri::generate_handler![
            // Filesystem
            runyard_core::commands::fs_list,
            runyard_core::commands::fs_read,
            runyard_core::commands::fs_write,
            runyard_core::commands::fs_watch,
            // Git basics (already existed)
            runyard_core::commands::git_branch,
            runyard_core::commands::get_home_dir,
            // Settings
            runyard_core::settings::settings_load,
            runyard_core::settings::settings_save,
            // Git operations (M2)
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
            // Terminal (M2)
            runyard_core::terminal::terminal_create,
            runyard_core::terminal::terminal_write,
            runyard_core::terminal::terminal_resize,
            runyard_core::terminal::terminal_close,
            runyard_core::terminal::terminal_list,
            // LSP (M2)
            runyard_core::lsp_manager::lsp_start,
            runyard_core::lsp_manager::lsp_send,
            runyard_core::lsp_manager::lsp_stop,
            runyard_core::lsp_manager::lsp_status,
            runyard_core::lsp_manager::lsp_status_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
