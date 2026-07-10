use git2::{
    BranchType, Repository,
    Signature, StatusOptions,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitFileEntry {
    pub path: String,
    pub status: String, // "modified" | "added" | "deleted" | "renamed" | "untracked"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitStatus {
    pub branch: Option<String>,
    pub changed: Vec<GitFileEntry>,   // unstaged changes
    pub staged: Vec<GitFileEntry>,    // staged changes
    pub untracked: Vec<GitFileEntry>, // untracked files
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitBranch {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitWorktree {
    pub name: String,
    pub path: String,
    pub branch: Option<String>,
    pub is_main: bool,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn open_repo(path: &str) -> Result<Repository, String> {
    Repository::discover(path).map_err(|e| e.to_string())
}

fn file_status_str(flags: git2::Status) -> String {
    if flags.contains(git2::Status::WT_NEW) {
        "untracked".to_string()
    } else if flags.contains(git2::Status::WT_MODIFIED)
        || flags.contains(git2::Status::INDEX_MODIFIED)
    {
        "modified".to_string()
    } else if flags.contains(git2::Status::WT_DELETED)
        || flags.contains(git2::Status::INDEX_DELETED)
    {
        "deleted".to_string()
    } else if flags.contains(git2::Status::WT_RENAMED)
        || flags.contains(git2::Status::INDEX_RENAMED)
    {
        "renamed".to_string()
    } else if flags.contains(git2::Status::INDEX_NEW) {
        "added".to_string()
    } else {
        "unknown".to_string()
    }
}

fn default_signature(repo: &Repository) -> Result<Signature<'static>, String> {
    repo.signature()
        .or_else(|_| Signature::now("Runyard User", "user@runyard.dev"))
        .map_err(|e| e.to_string())
}

// ─── Tauri commands ───────────────────────────────────────────────────────────

/// Get a comprehensive status snapshot of the working tree
#[tauri::command]
pub fn git_status(path: String) -> Result<GitStatus, String> {
    let repo = open_repo(&path)?;

    let head_branch = repo.head().ok().and_then(|h| {
        h.shorthand().map(|s| s.to_string())
    });

    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts)).map_err(|e| e.to_string())?;

    let mut changed: Vec<GitFileEntry> = Vec::new();
    let mut staged: Vec<GitFileEntry> = Vec::new();
    let mut untracked: Vec<GitFileEntry> = Vec::new();

    for entry in statuses.iter() {
        let path_str = entry.path().unwrap_or("").to_string();
        let flags = entry.status();

        if flags.contains(git2::Status::WT_NEW) {
            untracked.push(GitFileEntry {
                path: path_str,
                status: "untracked".to_string(),
            });
            continue;
        }

        // Staged changes (index vs HEAD)
        let index_flags = flags
            & (git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED);
        if !index_flags.is_empty() {
            staged.push(GitFileEntry {
                path: path_str.clone(),
                status: file_status_str(index_flags),
            });
        }

        // Unstaged changes (working tree vs index)
        let wt_flags = flags
            & (git2::Status::WT_MODIFIED | git2::Status::WT_DELETED | git2::Status::WT_RENAMED);
        if !wt_flags.is_empty() {
            changed.push(GitFileEntry {
                path: path_str,
                status: file_status_str(wt_flags),
            });
        }
    }

    // Ahead/behind count (best-effort, skip if no remote)
    let (ahead, behind) = compute_ahead_behind(&repo).unwrap_or((0, 0));

    Ok(GitStatus {
        branch: head_branch,
        changed,
        staged,
        untracked,
        ahead,
        behind,
    })
}

fn compute_ahead_behind(repo: &Repository) -> Option<(usize, usize)> {
    let head = repo.head().ok()?;
    let local_oid = head.target()?;
    let branch_name = head.shorthand()?;
    let upstream_ref = format!("refs/remotes/origin/{}", branch_name);
    let upstream_oid = repo.refname_to_id(&upstream_ref).ok()?;
    repo.graph_ahead_behind(local_oid, upstream_oid).ok()
}

/// Stage specific files
#[tauri::command]
pub fn git_stage(path: String, files: Vec<String>) -> Result<(), String> {
    let repo = open_repo(&path)?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    for file in &files {
        index
            .add_path(Path::new(file))
            .map_err(|e| format!("Failed to stage {}: {}", file, e))?;
    }
    index.write().map_err(|e| e.to_string())?;
    Ok(())
}

/// Unstage specific files (reset HEAD)
#[tauri::command]
pub fn git_unstage(path: String, files: Vec<String>) -> Result<(), String> {
    let repo = open_repo(&path)?;

    // Get HEAD commit to reset to
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => {
            // No HEAD yet (first commit): just remove from index
            let mut index = repo.index().map_err(|e| e.to_string())?;
            for file in &files {
                index
                    .remove_path(Path::new(file))
                    .map_err(|e| format!("Failed to unstage {}: {}", file, e))?;
            }
            index.write().map_err(|e| e.to_string())?;
            return Ok(());
        }
    };

    let head_commit = head.peel_to_commit().map_err(|e| e.to_string())?;
    let head_tree = head_commit.tree().map_err(|e| e.to_string())?;

    // Reset index entries to HEAD for specified files
    let mut index = repo.index().map_err(|e| e.to_string())?;
    for file in &files {
        let file_path = Path::new(file);
        // Try to find the entry in HEAD tree and restore it to index
        match head_tree.get_path(file_path) {
            Ok(tree_entry) => {
                let blob = repo
                    .find_blob(tree_entry.id())
                    .map_err(|e| e.to_string())?;
                let ie = git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: tree_entry.filemode() as u32,
                    uid: 0,
                    gid: 0,
                    file_size: blob.size() as u32,
                    id: tree_entry.id(),
                    flags: 0,
                    flags_extended: 0,
                    path: file.as_bytes().to_vec(),
                };
                index.add(&ie).map_err(|e| e.to_string())?;
            }
            Err(_) => {
                // File not in HEAD — just remove from index
                let _ = index.remove_path(file_path);
            }
        }
    }
    index.write().map_err(|e| e.to_string())?;
    Ok(())
}

/// Discard working tree changes for specific files
#[tauri::command]
pub fn git_discard(path: String, files: Vec<String>) -> Result<(), String> {
    let repo = open_repo(&path)?;
    let mut checkout_opts = git2::build::CheckoutBuilder::new();
    checkout_opts.force();
    for file in &files {
        checkout_opts.path(file);
    }
    repo.checkout_index(None, Some(&mut checkout_opts))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Create a commit from the current staged index
#[tauri::command]
pub fn git_commit(path: String, message: String) -> Result<String, String> {
    let repo = open_repo(&path)?;
    let sig = default_signature(&repo)?;

    let mut index = repo.index().map_err(|e| e.to_string())?;
    let tree_id = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;

    let parents = match repo.head() {
        Ok(head) => {
            let parent_commit = head.peel_to_commit().map_err(|e| e.to_string())?;
            vec![parent_commit]
        }
        Err(_) => vec![], // initial commit
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, &message, &tree, &parent_refs)
        .map_err(|e| e.to_string())?;

    Ok(format!("{}", oid))
}

/// Get recent commit log
#[tauri::command]
pub fn git_log(path: String, limit: usize) -> Result<Vec<GitCommit>, String> {
    let repo = open_repo(&path)?;
    let mut revwalk = repo.revwalk().map_err(|e| e.to_string())?;
    revwalk.push_head().map_err(|e| e.to_string())?;
    revwalk.set_sorting(git2::Sort::TIME).map_err(|e| e.to_string())?;

    let mut commits = Vec::new();
    for (i, oid_result) in revwalk.enumerate() {
        if i >= limit {
            break;
        }
        let oid = oid_result.map_err(|e| e.to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;
        let short_hash = format!("{:.7}", commit.id());
        let message = commit
            .summary()
            .unwrap_or("(no message)")
            .to_string();
        let author = commit.author().name().unwrap_or("Unknown").to_string();
        let timestamp = commit.time().seconds();

        commits.push(GitCommit {
            hash: commit.id().to_string(),
            short_hash,
            message,
            author,
            timestamp,
        });
    }
    Ok(commits)
}

/// List all branches
#[tauri::command]
pub fn git_branches(path: String) -> Result<Vec<GitBranch>, String> {
    let repo = open_repo(&path)?;
    let current_branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()));

    let mut branches = Vec::new();

    for branch_result in repo.branches(None).map_err(|e| e.to_string())? {
        let (branch, branch_type) = branch_result.map_err(|e| e.to_string())?;
        let name = branch
            .name()
            .map_err(|e| e.to_string())?
            .unwrap_or("")
            .to_string();
        let is_remote = matches!(branch_type, BranchType::Remote);
        let is_current = current_branch.as_deref() == Some(&name);

        branches.push(GitBranch {
            name,
            is_current,
            is_remote,
        });
    }
    Ok(branches)
}

/// Checkout a branch
#[tauri::command]
pub fn git_checkout(path: String, branch: String) -> Result<(), String> {
    let repo = open_repo(&path)?;
    let (object, reference) = repo
        .revparse_ext(&branch)
        .map_err(|e| format!("Branch not found: {}", e))?;

    repo.checkout_tree(&object, None)
        .map_err(|e| e.to_string())?;

    if let Some(gref) = reference {
        repo.set_head(gref.name().unwrap_or(""))
            .map_err(|e| e.to_string())?;
    } else {
        repo.set_head_detached(object.id())
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Create a new local branch from HEAD
#[tauri::command]
pub fn git_create_branch(path: String, name: String) -> Result<(), String> {
    let repo = open_repo(&path)?;
    let head_commit = repo
        .head()
        .map_err(|e| e.to_string())?
        .peel_to_commit()
        .map_err(|e| e.to_string())?;
    repo.branch(&name, &head_commit, false)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// List worktrees
#[tauri::command]
pub fn git_worktrees(path: String) -> Result<Vec<GitWorktree>, String> {
    let repo = open_repo(&path)?;
    let worktree_names = repo.worktrees().map_err(|e| e.to_string())?;

    let mut result = Vec::new();

    // Add main worktree
    let main_path = repo
        .workdir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    let main_branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()));

    result.push(GitWorktree {
        name: "main".to_string(),
        path: main_path,
        branch: main_branch,
        is_main: true,
    });

    // Add linked worktrees
    for name_binding in worktree_names.iter() {
        if let Some(name_str) = name_binding {
            let name = name_str.to_string();
            if let Ok(wt) = repo.find_worktree(&name) {
                let wt_path = wt.path().to_string_lossy().to_string();
                // Try to get the branch of the worktree
                let wt_branch = if let Ok(wt_repo) = Repository::open(&wt_path) {
                    wt_repo
                        .head()
                        .ok()
                        .and_then(|h| h.shorthand().map(|s| s.to_string()))
                } else {
                    None
                };

                result.push(GitWorktree {
                    name,
                    path: wt_path,
                    branch: wt_branch,
                    is_main: false,
                });
            }
        }
    }

    Ok(result)
}

/// Create a linked worktree
#[tauri::command]
pub fn git_worktree_create(
    path: String,
    name: String,
    target_path: String,
    branch: Option<String>,
) -> Result<(), String> {
    let repo = open_repo(&path)?;

    let mut opts = git2::WorktreeAddOptions::new();

    // If a branch name is provided, resolve it
    let reference = if let Some(ref branch_name) = branch {
        let branch_obj = repo
            .find_branch(branch_name, BranchType::Local)
            .map_err(|e| format!("Branch '{}' not found: {}", branch_name, e))?;
        Some(branch_obj.into_reference())
    } else {
        None
    };
    if let Some(ref r) = reference {
        opts.reference(Some(r));
    }

    repo.worktree(&name, Path::new(&target_path), Some(&opts))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove a linked worktree
#[tauri::command]
pub fn git_worktree_remove(path: String, name: String) -> Result<(), String> {
    let repo = open_repo(&path)?;
    let wt = repo
        .find_worktree(&name)
        .map_err(|e| format!("Worktree '{}' not found: {}", name, e))?;
    let mut prune_opts = git2::WorktreePruneOptions::new();
    prune_opts.valid(true);
    wt.prune(Some(&mut prune_opts))
        .map_err(|e| e.to_string())?;
    Ok(())
}
