<script lang="ts">
  import { invoke as tauriInvoke } from "@tauri-apps/api/core";
  import { webSocketClient } from "@runyard/common";
  import { workspaceStore } from "../stores/workspaceStore.svelte.js";
  import { layoutEngine } from "./layoutStore.svelte.js";

  let { open = $bindable(false) }: { open?: boolean } = $props();

  async function invoke<T>(cmd: string, args?: any): Promise<T> {
    if (webSocketClient.status === "connected") {
      return webSocketClient.invoke<T>(cmd, args);
    } else {
      return tauriInvoke<T>(cmd, args);
    }
  }

  interface FileEntry {
    path: string;
    name: string;
    kind: "file" | "dir";
  }

  const SKIP_DIRS = new Set(["node_modules", ".git", "target", "dist", ".svelte-kit", "build", ".next", "__pycache__"]);

  let searchQuery = $state("");
  let selectedIndex = $state(0);
  let inputEl = $state<HTMLInputElement | undefined>(undefined);
  let fileList = $state<FileEntry[]>([]);
  let isScanning = $state(false);

  // Cache the last scanned workspace path so we only re-scan on workspace change
  let lastScannedPath = $state("");

  async function scanWorkspace(rootPath: string, depth = 0): Promise<FileEntry[]> {
    if (depth > 3) return [];
    try {
      const entries = await invoke<{ path: string; name: string; kind: string }[]>("fs_list", { path: rootPath });
      const results: FileEntry[] = [];
      const subdirPromises: Promise<FileEntry[]>[] = [];

      for (const entry of entries) {
        if (entry.kind === "dir") {
          if (!SKIP_DIRS.has(entry.name)) {
            subdirPromises.push(scanWorkspace(entry.path, depth + 1));
          }
        } else {
          results.push({ path: entry.path, name: entry.name, kind: "file" });
        }
      }

      const subResults = await Promise.all(subdirPromises);
      for (const sub of subResults) {
        results.push(...sub);
      }
      return results;
    } catch (e) {
      return [];
    }
  }

  async function refreshFileList() {
    const root = workspaceStore.currentPath;
    if (root === lastScannedPath) return;
    isScanning = true;
    try {
      fileList = await scanWorkspace(root);
      lastScannedPath = root;
    } finally {
      isScanning = false;
    }
  }

  // Fuzzy scoring: returns a score >= 0 (higher = better match), or -1 for no match
  function fuzzyScore(needle: string, haystack: string): number {
    if (!needle) return 0;
    const n = needle.toLowerCase();
    const h = haystack.toLowerCase();
    let score = 0;
    let j = 0;
    let consecutive = 0;
    for (let i = 0; i < h.length && j < n.length; i++) {
      if (h[i] === n[j]) {
        j++;
        consecutive++;
        score += consecutive; // bonus for consecutive matches
      } else {
        consecutive = 0;
      }
    }
    if (j < n.length) return -1; // not all needle chars matched
    return score;
  }

  let filteredResults = $derived.by(() => {
    const q = searchQuery.trim();
    if (!q) return fileList.slice(0, 50);
    const scored: { entry: FileEntry; score: number }[] = [];
    for (const entry of fileList) {
      // Score against the filename first, then full path
      const nameScore = fuzzyScore(q, entry.name);
      const pathScore = fuzzyScore(q, entry.path);
      const best = Math.max(nameScore, pathScore);
      if (best >= 0) {
        // Boost filename matches over path matches
        const boosted = nameScore >= 0 ? best + 100 : best;
        scored.push({ entry, score: boosted });
      }
    }
    scored.sort((a, b) => b.score - a.score);
    return scored.slice(0, 50).map((s) => s.entry);
  });

  $effect(() => {
    if (open) {
      selectedIndex = 0;
      searchQuery = "";
      // Trigger scan (no-op if already scanned this workspace)
      refreshFileList();
      setTimeout(() => inputEl?.focus(), 0);
    }
  });

  // Re-scan when workspace changes while overlay is open
  $effect(() => {
    const path = workspaceStore.currentPath;
    if (open && path !== lastScannedPath) {
      refreshFileList();
    }
  });

  function close() {
    open = false;
    searchQuery = "";
  }

  function openFile(entry: FileEntry) {
    close();
    layoutEngine.openEditor(entry.path, entry.name);
  }

  function handleKey(e: KeyboardEvent) {
    switch (e.key) {
      case "Escape":
        e.preventDefault();
        close();
        break;
      case "ArrowDown":
        e.preventDefault();
        selectedIndex = Math.min(selectedIndex + 1, filteredResults.length - 1);
        scrollSelectedIntoView();
        break;
      case "ArrowUp":
        e.preventDefault();
        selectedIndex = Math.max(selectedIndex - 1, 0);
        scrollSelectedIntoView();
        break;
      case "Enter":
        e.preventDefault();
        if (filteredResults[selectedIndex]) {
          openFile(filteredResults[selectedIndex]);
        }
        break;
    }
  }

  function scrollSelectedIntoView() {
    requestAnimationFrame(() => {
      const el = document.querySelector(".quick-open-item.selected");
      el?.scrollIntoView({ block: "nearest" });
    });
  }

  function truncatePath(path: string): string {
    const root = workspaceStore.currentPath.replace(/\/$/, "");
    if (path.startsWith(root)) {
      return path.slice(root.length).replace(/^\//, "");
    }
    return path;
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="quick-open-backdrop" onclick={close}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="quick-open-container" onclick={(e) => e.stopPropagation()}>
      <!-- Workspace breadcrumb -->
      <div class="quick-open-breadcrumb">
        <span class="breadcrumb-label">Workspace:</span>
        <span class="breadcrumb-path">{workspaceStore.currentPath}</span>
      </div>

      <!-- Search input -->
      <div class="quick-open-search">
        <svg class="search-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8" /><path d="m21 21-4.35-4.35" />
        </svg>
        <input
          bind:this={inputEl}
          bind:value={searchQuery}
          onkeydown={handleKey}
          oninput={() => (selectedIndex = 0)}
          class="quick-open-input"
          placeholder="Go to file..."
          autocomplete="off"
          spellcheck="false"
        />
        {#if searchQuery}
          <button class="clear-btn" onclick={() => (searchQuery = "")}>×</button>
        {:else}
          <kbd class="esc-hint">Esc</kbd>
        {/if}
      </div>

      <!-- Results -->
      <div class="quick-open-results">
        {#if isScanning}
          <div class="scanning-msg">Scanning workspace...</div>
        {:else if filteredResults.length === 0}
          <div class="no-results">
            {searchQuery ? `No files matching "${searchQuery}"` : "No files found"}
          </div>
        {:else}
          {#each filteredResults as entry, idx}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div
              class="quick-open-item"
              class:selected={selectedIndex === idx}
              onmouseenter={() => (selectedIndex = idx)}
              onclick={() => openFile(entry)}
            >
              <div class="item-content">
                <span class="item-name">{entry.name}</span>
                <span class="item-path">{truncatePath(entry.path)}</span>
              </div>
            </div>
          {/each}
        {/if}
      </div>

      <!-- Footer hint -->
      <div class="quick-open-footer">
        <span>↑↓ navigate</span>
        <span>↵ open</span>
        <span>Esc close</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .quick-open-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 10vh;
    animation: fade-in 0.1s ease;
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .quick-open-container {
    width: 640px;
    max-width: calc(100vw - 32px);
    background: var(--bg-elevated, var(--bg));
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow:
      0 4px 6px -1px rgba(0, 0, 0, 0.4),
      0 20px 60px -8px rgba(0, 0, 0, 0.6);
    overflow: hidden;
    animation: slide-down 0.12s ease;
    max-height: 60vh;
    display: flex;
    flex-direction: column;
  }

  @keyframes slide-down {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .quick-open-breadcrumb {
    padding: 8px 16px 0;
    font-size: 11px;
    color: var(--text-tertiary);
    font-family: var(--font-mono, "JetBrains Mono", monospace);
    display: flex;
    gap: 6px;
    align-items: baseline;
    overflow: hidden;
    flex-shrink: 0;
  }

  .breadcrumb-label {
    color: var(--text-secondary);
    font-weight: 600;
    flex-shrink: 0;
  }

  .breadcrumb-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .quick-open-search {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .search-icon {
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .quick-open-input {
    flex: 1;
    background: none;
    border: none;
    color: var(--text);
    font-size: 15px;
    font-family: var(--font-sans, system-ui, sans-serif);
    outline: none;
    min-width: 0;
  }

  .quick-open-input::placeholder {
    color: var(--text-secondary);
  }

  .clear-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 18px;
    cursor: pointer;
    line-height: 1;
    padding: 0 2px;
    flex-shrink: 0;
  }

  .clear-btn:hover {
    color: var(--text);
  }

  .esc-hint {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 11px;
    padding: 2px 6px;
    color: var(--text-secondary);
    font-family: var(--font-mono, "JetBrains Mono", monospace);
    flex-shrink: 0;
  }

  .quick-open-results {
    overflow-y: auto;
    padding: 6px 0;
    flex: 1;
    min-height: 0;
  }

  .scanning-msg,
  .no-results {
    padding: 24px 16px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .quick-open-item {
    display: flex;
    align-items: center;
    padding: 7px 16px;
    cursor: pointer;
    border-radius: 4px;
    margin: 0 6px;
  }

  .quick-open-item.selected {
    background: rgba(59, 130, 246, 0.12);
  }

  .quick-open-item:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .quick-open-item.selected:hover {
    background: rgba(59, 130, 246, 0.15);
  }

  .item-content {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .item-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .quick-open-item.selected .item-name {
    color: var(--accent);
  }

  .item-path {
    font-size: 11px;
    color: var(--text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-top: 1px;
    font-family: var(--font-mono, "JetBrains Mono", monospace);
  }

  .quick-open-footer {
    display: flex;
    gap: 16px;
    padding: 7px 16px;
    border-top: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }
</style>
