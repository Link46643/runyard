<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import {
    GitBranch,
    GitCommit,
    Plus,
    Minus,
    RotateCcw,
    Check,
    ChevronDown,
    ChevronRight,
    RefreshCw,
    AlertCircle,
    Layers,
    FolderOpen,
  } from "lucide-svelte";
  import { layoutEngine } from "./layoutStore.svelte.js";
  import type {
    GitStatus,
    GitFileEntry,
    GitCommit as GitCommitType,
    GitBranch as GitBranchType,
    GitWorktree,
  } from "@runyard/common";

  let { workspacePath }: { workspacePath: string } = $props();

  // ── State ──────────────────────────────────────────────────────────────────
  let status = $state<GitStatus | null>(null);
  let commits = $state<GitCommitType[]>([]);
  let branches = $state<GitBranchType[]>([]);
  let worktrees = $state<GitWorktree[]>([]);
  let commitMessage = $state("");
  let isLoading = $state(true);
  let error = $state<string | null>(null);
  let showBranchList = $state(false);
  let showWorktrees = $state(false);
  let newBranchName = $state("");
  let showNewBranch = $state(false);
  let isCommitting = $state(false);
  let operationError = $state<string | null>(null);
  let newWorktreeName = $state("");
  let newWorktreePath = $state("");
  let showCreateWorktree = $state(false);

  // ── Load ───────────────────────────────────────────────────────────────────
  async function loadAll() {
    isLoading = true;
    error = null;
    try {
      const [s, c, b, w] = await Promise.all([
        invoke<GitStatus>("git_status", { path: workspacePath }),
        invoke<GitCommitType[]>("git_log", { path: workspacePath, limit: 20 }),
        invoke<GitBranchType[]>("git_branches", { path: workspacePath }),
        invoke<GitWorktree[]>("git_worktrees", { path: workspacePath }),
      ]);
      status = s;
      commits = c;
      branches = b;
      worktrees = w;
    } catch (e) {
      error = String(e);
    } finally {
      isLoading = false;
    }
  }

  async function loadStatus() {
    try {
      status = await invoke<GitStatus>("git_status", { path: workspacePath });
    } catch (e) {
      console.warn("git_status failed:", e);
    }
  }

  onMount(() => {
    loadAll();
    // Refresh on filesystem changes
    const unlisten = listen("fs:changed", () => loadStatus());
    return () => {
      unlisten.then((fn) => fn());
    };
  });

  // ── Actions ────────────────────────────────────────────────────────────────
  async function stageFile(path: string) {
    operationError = null;
    try {
      await invoke("git_stage", { path: workspacePath, files: [path] });
      await loadStatus();
    } catch (e) {
      operationError = String(e);
    }
  }

  async function unstageFile(path: string) {
    operationError = null;
    try {
      await invoke("git_unstage", { path: workspacePath, files: [path] });
      await loadStatus();
    } catch (e) {
      operationError = String(e);
    }
  }

  async function discardFile(path: string) {
    operationError = null;
    try {
      await invoke("git_discard", { path: workspacePath, files: [path] });
      await loadStatus();
    } catch (e) {
      operationError = String(e);
    }
  }

  async function stageAll() {
    operationError = null;
    if (!status) return;
    const files = [...status.changed, ...status.untracked].map((f) => f.path);
    if (files.length === 0) return;
    try {
      await invoke("git_stage", { path: workspacePath, files });
      await loadStatus();
    } catch (e) {
      operationError = String(e);
    }
  }

  async function unstageAll() {
    operationError = null;
    if (!status) return;
    const files = status.staged.map((f) => f.path);
    if (files.length === 0) return;
    try {
      await invoke("git_unstage", { path: workspacePath, files });
      await loadStatus();
    } catch (e) {
      operationError = String(e);
    }
  }

  async function commitChanges() {
    if (!commitMessage.trim()) return;
    isCommitting = true;
    operationError = null;
    try {
      await invoke("git_commit", {
        path: workspacePath,
        message: commitMessage.trim(),
      });
      commitMessage = "";
      await loadAll();
    } catch (e) {
      operationError = String(e);
    } finally {
      isCommitting = false;
    }
  }

  async function checkoutBranch(name: string) {
    operationError = null;
    try {
      await invoke("git_checkout", { path: workspacePath, branch: name });
      showBranchList = false;
      await loadAll();
    } catch (e) {
      operationError = String(e);
    }
  }

  async function createBranch() {
    if (!newBranchName.trim()) return;
    operationError = null;
    try {
      await invoke("git_create_branch", {
        path: workspacePath,
        name: newBranchName.trim(),
      });
      await invoke("git_checkout", {
        path: workspacePath,
        branch: newBranchName.trim(),
      });
      newBranchName = "";
      showNewBranch = false;
      await loadAll();
    } catch (e) {
      operationError = String(e);
    }
  }

  async function createWorktree() {
    if (!newWorktreeName.trim() || !newWorktreePath.trim()) return;
    operationError = null;
    try {
      await invoke("git_worktree_create", {
        path: workspacePath,
        name: newWorktreeName.trim(),
        targetPath: newWorktreePath.trim(),
        branch: null,
      });
      newWorktreeName = "";
      newWorktreePath = "";
      showCreateWorktree = false;
      await loadAll();
    } catch (e) {
      operationError = String(e);
    }
  }

  async function removeWorktree(name: string) {
    operationError = null;
    try {
      await invoke("git_worktree_remove", { path: workspacePath, name });
      await loadAll();
    } catch (e) {
      operationError = String(e);
    }
  }

  function openWorktreeAsWorkspace(wt: GitWorktree) {
    layoutEngine.openGit(wt.path);
  }

  function statusIcon(s: string) {
    const map: Record<string, string> = {
      modified: "M",
      added: "A",
      deleted: "D",
      renamed: "R",
      untracked: "?",
    };
    return map[s] ?? "?";
  }

  function statusColor(s: string) {
    const map: Record<string, string> = {
      modified: "var(--accent)",
      added: "#22c55e",
      deleted: "#ef4444",
      renamed: "#eab308",
      untracked: "#6b7280",
    };
    return map[s] ?? "#6b7280";
  }

  function formatTime(ts: number) {
    const d = new Date(ts * 1000);
    return d.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      year:
        d.getFullYear() !== new Date().getFullYear() ? "numeric" : undefined,
    });
  }
</script>

<div class="git-panel">
  {#if isLoading}
    <div class="loading">
      <RefreshCw size={14} />
      <span>Loading repository...</span>
    </div>
  {:else if error}
    <div class="error-state">
      <AlertCircle size={16} />
      <div>
        <p class="error-title">Not a Git repository</p>
        <p class="error-detail">{error}</p>
      </div>
    </div>
  {:else if status}
    <!-- Branch header -->
    <div class="section branch-section">
      <button
        class="branch-btn"
        onclick={() => (showBranchList = !showBranchList)}
      >
        <GitBranch size={13} />
        <span class="branch-name">{status.branch ?? "HEAD detached"}</span>
        {#if status.ahead > 0 || status.behind > 0}
          <span class="sync-badge">
            {#if status.ahead > 0}↑{status.ahead}{/if}
            {#if status.behind > 0}↓{status.behind}{/if}
          </span>
        {/if}
        <ChevronDown size={12} class="chevron" />
      </button>
      <button
        class="icon-btn"
        title="Refresh"
        onclick={() => loadAll()}
      >
        <RefreshCw size={13} />
      </button>
    </div>

    <!-- Branch list dropdown -->
    {#if showBranchList}
      <div class="branch-dropdown">
        <div class="branch-list-header">
          <span>Branches</span>
          <button
            class="text-btn"
            onclick={() => {
              showBranchList = false;
              showNewBranch = true;
            }}>+ New</button
          >
        </div>
        {#each branches.filter((b) => !b.is_remote) as branch}
          <button
            class="branch-item"
            class:active={branch.is_current}
            onclick={() => checkoutBranch(branch.name)}
          >
            {#if branch.is_current}
              <Check size={11} />
            {:else}
              <span class="spacer"></span>
            {/if}
            {branch.name}
          </button>
        {/each}
      </div>
    {/if}

    <!-- New branch form -->
    {#if showNewBranch}
      <div class="inline-form">
        <input
          type="text"
          placeholder="Branch name"
          bind:value={newBranchName}
          onkeydown={(e) => {
            if (e.key === "Enter") createBranch();
            if (e.key === "Escape") {
              showNewBranch = false;
              newBranchName = "";
            }
          }}
          class="form-input"
          autofocus
        />
        <button class="btn-primary" onclick={createBranch}>Create</button>
        <button
          class="btn-ghost"
          onclick={() => {
            showNewBranch = false;
            newBranchName = "";
          }}>Cancel</button
        >
      </div>
    {/if}

    {#if operationError}
      <div class="op-error">
        <AlertCircle size={12} />
        {operationError}
      </div>
    {/if}

    <!-- Staged files -->
    <div class="section-header">
      <span>Staged Changes ({status.staged.length})</span>
      {#if status.staged.length > 0}
        <button class="text-btn" onclick={unstageAll} title="Unstage all"
          >–</button
        >
      {/if}
    </div>
    {#each status.staged as file}
      <div class="file-row">
        <span class="status-badge" style="color: {statusColor(file.status)}">
          {statusIcon(file.status)}
        </span>
        <span class="file-path" title={file.path}>{file.path}</span>
        <div class="file-actions">
          <button
            class="icon-btn"
            title="Unstage"
            onclick={() => unstageFile(file.path)}
          >
            <Minus size={11} />
          </button>
        </div>
      </div>
    {/each}

    <!-- Changed files -->
    <div class="section-header">
      <span
        >Changes ({status.changed.length + status.untracked.length})</span
      >
      {#if status.changed.length + status.untracked.length > 0}
        <button class="text-btn" onclick={stageAll} title="Stage all">+</button
        >
      {/if}
    </div>
    {#each [...status.changed, ...status.untracked] as file}
      <div class="file-row">
        <span class="status-badge" style="color: {statusColor(file.status)}">
          {statusIcon(file.status)}
        </span>
        <span class="file-path" title={file.path}>{file.path}</span>
        <div class="file-actions">
          <button
            class="icon-btn"
            title="Stage"
            onclick={() => stageFile(file.path)}
          >
            <Plus size={11} />
          </button>
          {#if file.status !== "untracked"}
            <button
              class="icon-btn danger"
              title="Discard changes"
              onclick={() => discardFile(file.path)}
            >
              <RotateCcw size={11} />
            </button>
          {/if}
        </div>
      </div>
    {/each}

    <!-- Commit message -->
    <div class="commit-section">
      <textarea
        class="commit-input"
        placeholder="Commit message (required)"
        bind:value={commitMessage}
        rows={3}
        onkeydown={(e) => {
          if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) commitChanges();
        }}
      ></textarea>
      <button
        class="btn-primary commit-btn"
        disabled={!commitMessage.trim() ||
          status.staged.length === 0 ||
          isCommitting}
        onclick={commitChanges}
      >
        {isCommitting ? "Committing..." : `Commit (${status.staged.length} staged)`}
      </button>
    </div>

    <!-- Recent commits -->
    {#if commits.length > 0}
      <div class="section-header">
        <GitCommit size={12} />
        <span>Recent Commits</span>
      </div>
      <div class="commit-list">
        {#each commits as commit}
          <div class="commit-item" title={commit.message}>
            <span class="commit-hash">{commit.short_hash}</span>
            <span class="commit-msg">{commit.message}</span>
            <span class="commit-meta"
              >{commit.author} · {formatTime(commit.timestamp)}</span
            >
          </div>
        {/each}
      </div>
    {/if}

    <!-- Worktrees -->
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="section-header clickable" onclick={() => (showWorktrees = !showWorktrees)}>
      <Layers size={12} />
      <span>Worktrees ({worktrees.length})</span>
      {#if showWorktrees}
        <ChevronDown size={11} />
      {:else}
        <ChevronRight size={11} />
      {/if}
    </div>

    {#if showWorktrees}
      {#each worktrees as wt}
        <div class="worktree-item">
          <div class="worktree-info">
            <span class="wt-name">{wt.name}</span>
            {#if wt.branch}
              <span class="wt-branch">{wt.branch}</span>
            {/if}
            <span class="wt-path">{wt.path}</span>
          </div>
          <div class="worktree-actions">
            <button
              class="icon-btn"
              title="Open worktree as workspace"
              onclick={() => openWorktreeAsWorkspace(wt)}
            >
              <FolderOpen size={11} />
            </button>
            {#if !wt.is_main}
              <button
                class="icon-btn danger"
                title="Remove worktree"
                onclick={() => removeWorktree(wt.name)}
              >
                <Minus size={11} />
              </button>
            {/if}
          </div>
        </div>
      {/each}

      <button
        class="text-btn add-wt"
        onclick={() => (showCreateWorktree = !showCreateWorktree)}
      >
        + Add worktree
      </button>

      {#if showCreateWorktree}
        <div class="inline-form vertical">
          <input
            type="text"
            placeholder="Name (e.g. feature-x)"
            bind:value={newWorktreeName}
            class="form-input"
          />
          <input
            type="text"
            placeholder="Target path"
            bind:value={newWorktreePath}
            class="form-input"
          />
          <div class="form-actions">
            <button class="btn-primary" onclick={createWorktree}>Create</button>
            <button
              class="btn-ghost"
              onclick={() => {
                showCreateWorktree = false;
                newWorktreeName = "";
                newWorktreePath = "";
              }}>Cancel</button
            >
          </div>
        </div>
      {/if}
    {/if}
  {/if}
</div>

<style>
  .git-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow-y: auto;
    font-family: "JetBrains Mono", ui-monospace, monospace;
    font-size: 12px;
    color: var(--text);
    background: var(--bg);
    padding-bottom: 16px;
  }

  .loading,
  .error-state {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 16px;
    color: var(--text-secondary);
  }

  .error-title {
    font-weight: 600;
    margin: 0 0 2px;
    color: var(--text);
  }

  .error-detail {
    margin: 0;
    font-size: 11px;
    opacity: 0.7;
  }

  /* Branch section */
  .section {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
  }

  .branch-btn {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    background: none;
    border: none;
    color: var(--text);
    cursor: pointer;
    padding: 0;
    font-size: 12px;
    font-family: inherit;
    text-align: left;
  }

  .branch-btn:hover {
    color: var(--accent);
  }

  .branch-name {
    font-weight: 600;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sync-badge {
    font-size: 10px;
    color: var(--text-secondary);
    background: rgba(59, 130, 246, 0.1);
    padding: 0 4px;
    border-radius: 3px;
  }

  .branch-dropdown {
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    max-height: 200px;
    overflow-y: auto;
  }

  .branch-list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 10px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border);
  }

  .branch-item {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 5px 10px;
    background: none;
    border: none;
    color: var(--text);
    cursor: pointer;
    font-size: 12px;
    font-family: inherit;
    text-align: left;
  }

  .branch-item:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .branch-item.active {
    color: var(--accent);
  }

  .spacer {
    width: 11px;
  }

  /* Section headers */
  .section-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 8px 4px;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-secondary);
    font-weight: 600;
    user-select: none;
  }

  .section-header.clickable {
    cursor: pointer;
  }

  .section-header .text-btn {
    margin-left: auto;
  }

  /* File rows */
  .file-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 3px 8px;
    border-radius: 3px;
    margin: 0 4px;
  }

  .file-row:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .file-row:hover .file-actions {
    opacity: 1;
  }

  .status-badge {
    font-size: 11px;
    font-weight: 700;
    width: 14px;
    text-align: center;
    flex-shrink: 0;
  }

  .file-path {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }

  .file-actions {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.1s;
    flex-shrink: 0;
  }

  /* Commit section */
  .commit-section {
    padding: 8px;
    border-top: 1px solid var(--border);
    margin-top: 4px;
  }

  .commit-input {
    width: 100%;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    font-family: inherit;
    font-size: 12px;
    padding: 6px 8px;
    resize: vertical;
    box-sizing: border-box;
    line-height: 1.5;
  }

  .commit-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .commit-btn {
    width: 100%;
    margin-top: 6px;
  }

  /* Commit list */
  .commit-list {
    padding: 0 4px;
  }

  .commit-item {
    padding: 6px 6px;
    border-radius: 3px;
    display: grid;
    grid-template-columns: 52px 1fr;
    grid-template-rows: auto auto;
    gap: 0 6px;
    cursor: default;
  }

  .commit-item:hover {
    background: rgba(255, 255, 255, 0.04);
  }

  .commit-hash {
    font-size: 11px;
    color: var(--accent);
    font-family: "JetBrains Mono", monospace;
    grid-row: 1;
    grid-column: 1;
  }

  .commit-msg {
    grid-row: 1;
    grid-column: 2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }

  .commit-meta {
    grid-row: 2;
    grid-column: 1 / -1;
    font-size: 10px;
    color: var(--text-secondary);
    margin-top: 1px;
  }

  /* Worktrees */
  .worktree-item {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 4px 8px;
    margin: 0 4px;
  }

  .worktree-actions {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
    align-items: center;
  }

  .worktree-info {
    flex: 1;
    min-width: 0;
  }

  .wt-name {
    font-weight: 600;
    display: block;
  }

  .wt-branch {
    font-size: 11px;
    color: var(--accent);
    display: block;
  }

  .wt-path {
    font-size: 10px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: block;
  }

  .add-wt {
    margin: 4px 8px;
  }

  /* Inline forms */
  .inline-form {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 6px 8px;
    flex-wrap: wrap;
  }

  .inline-form.vertical {
    flex-direction: column;
    align-items: stretch;
  }

  .form-input {
    flex: 1;
    min-width: 0;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    font-family: inherit;
    font-size: 12px;
    padding: 4px 8px;
  }

  .form-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .form-actions {
    display: flex;
    gap: 6px;
  }

  /* Shared button styles */
  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 3px;
    background: none;
    border: none;
    border-radius: 3px;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .icon-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    color: var(--text);
  }

  .icon-btn.danger:hover {
    color: #ef4444;
  }

  .text-btn {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font-size: 12px;
    font-family: inherit;
    padding: 2px 4px;
  }

  .text-btn:hover {
    text-decoration: underline;
  }

  .btn-primary {
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 4px;
    padding: 5px 12px;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn-primary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-primary:hover:not(:disabled) {
    filter: brightness(1.1);
  }

  .btn-ghost {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    padding: 4px 10px;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
  }

  .btn-ghost:hover {
    background: rgba(255, 255, 255, 0.06);
  }

  .op-error {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: rgba(239, 68, 68, 0.08);
    color: #f87171;
    font-size: 11px;
    border-left: 2px solid #ef4444;
    margin: 4px 8px;
    border-radius: 0 4px 4px 0;
  }
</style>
