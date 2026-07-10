<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { appStatus } from "./appStatusStore.svelte.js";
  import { lspStore } from "./lspStore.svelte.js";
  import { GitBranch, CircleDot } from "lucide-svelte";

  let fileEncoding = $state<string>("UTF-8");
  let connectionState = $state<string>("Local");

  // Derive active LSP languages and their statuses
  let lspBadges = $derived(
    Object.entries(lspStore.statuses)
      .filter(([, s]) => s.status !== "disconnected")
      .map(([lang, s]) => ({ lang: lang.slice(0, 2).toUpperCase(), status: s.status }))
  );

  function truncatePath(path: string | null) {
    if (!path) return "No file open";
    if (path.length > 50) {
      return "..." + path.slice(-47);
    }
    return path;
  }

  function lspStatusColor(status: string) {
    switch (status) {
      case "ready": return "#22c55e";
      case "starting": return "#eab308";
      case "error": return "#ef4444";
      default: return "#6b7280";
    }
  }

  onMount(async () => {
    try {
      const res = await invoke<string | null>("git_branch", { path: "../../" });
      if (res) {
        appStatus.updateGitBranch(res);
      }
    } catch (e) {
      console.error("Failed to fetch git branch", e);
      appStatus.updateGitBranch("no repo");
    }

    // Init LSP store event listener
    await lspStore.init();
  });
</script>

<div class="status-bar">
  <div class="left">
    <div class="item connection">{connectionState}</div>
    <div class="item git">
      <span class="icon"><GitBranch size={12} strokeWidth={2} /></span>
      {appStatus.gitBranch}
    </div>
    <div class="item path">{truncatePath(appStatus.activeFilePath)}</div>

    <!-- LSP status badges -->
    {#each lspBadges as badge}
      <div class="item lsp-badge" title="LSP: {badge.lang} ({badge.status})">
        <CircleDot size={10} color={lspStatusColor(badge.status)} />
        <span style="color: {lspStatusColor(badge.status)}">{badge.lang}</span>
      </div>
    {/each}
  </div>

  <div class="right">
    <div class="item cursor">
      {appStatus.cursorPosition.line}:{appStatus.cursorPosition.col}
    </div>
    <div class="item encoding">{fileEncoding}</div>
  </div>
</div>

<style>
  .status-bar {
    height: 24px;
    background-color: var(--statusbar-bg);
    color: var(--statusbar-text);
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0;
    font-family: "JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Monaco,
      Consolas, monospace;
    font-size: 11px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
    user-select: none;
    z-index: 100;
  }

  .left,
  .right {
    display: flex;
    align-items: center;
    height: 100%;
  }

  .item {
    padding: 0 10px;
    height: 100%;
    display: flex;
    align-items: center;
    cursor: default;
    transition: background 0.15s ease;
    gap: 4px;
  }

  .item:hover {
    background-color: rgba(128, 128, 128, 0.15);
  }

  .icon {
    display: flex;
    align-items: center;
    opacity: 0.8;
  }

  .connection {
    background-color: var(--accent);
    color: #ffffff;
    font-weight: 700;
    padding: 0 14px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .connection:hover {
    background-color: var(--accent);
    filter: contrast(1.1);
  }

  .git {
    font-weight: 500;
  }

  .path {
    opacity: 0.8;
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .lsp-badge {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.03em;
    padding: 0 8px;
  }
</style>
