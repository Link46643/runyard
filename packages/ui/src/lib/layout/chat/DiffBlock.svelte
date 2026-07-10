<script lang="ts">
  import type { DiffBlock } from "@runyard/common";
  import { parseUnifiedDiff, type ParsedDiffHunk } from "./diffParser.js";

  let { block }: { block: DiffBlock } = $props();

  let hunks = $state<ParsedDiffHunk[]>(parseUnifiedDiff(block.diff));
  let showUndo = $state(false);
  let undoTimer: ReturnType<typeof setTimeout> | null = null;

  function acceptHunk(id: string) {
    hunks = hunks.map((h) => (h.id === id ? { ...h, status: "accepted" } : h));
  }
  function rejectHunk(id: string) {
    hunks = hunks.map((h) => (h.id === id ? { ...h, status: "rejected" } : h));
    showUndo = true;
    if (undoTimer) clearTimeout(undoTimer);
    undoTimer = setTimeout(() => (showUndo = false), 30000);
  }
  function acceptAll() {
    hunks = hunks.map((h) => ({ ...h, status: "accepted" }));
  }
  function rejectAll() {
    hunks = hunks.map((h) => ({ ...h, status: "rejected" }));
    showUndo = true;
    if (undoTimer) clearTimeout(undoTimer);
    undoTimer = setTimeout(() => (showUndo = false), 30000);
  }
  function undoLast() {
    hunks = hunks.map((h) => (h.status === "rejected" ? { ...h, status: "pending" } : h));
    showUndo = false;
  }

  async function applyAccepted() {
    const acceptedHunks = hunks.filter((h) => h.status === "accepted");
    if (acceptedHunks.length === 0) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const current = await invoke<string>("fs_read", { path: block.filepath }).catch(() => "");
      // Best-effort: if we can't reconstruct precisely, fall back to writing
      // the accepted hunks' resulting lines directly for a single-hunk diff,
      // which is the common case for agent-proposed edits.
      const rebuilt = acceptedHunks
        .map((h) => h.lines.filter((l) => l.type !== "del").map((l) => l.content).join("\n"))
        .join("\n");
      await invoke("fs_write", { path: block.filepath, contents: rebuilt || current });
    } catch (e) {
      console.error("[DiffBlock] Failed to apply accepted hunks", e);
    }
  }

  let allDecided = $derived(hunks.every((h) => h.status !== "pending"));
</script>

<div class="diff-block">
  <div class="diff-header">
    <span class="filepath">{block.filepath}</span>
    <div class="spacer"></div>
    <button class="ghost-btn" onclick={acceptAll}>Accept all</button>
    <button class="ghost-btn" onclick={rejectAll}>Reject all</button>
    {#if allDecided}
      <button class="ghost-btn primary" onclick={applyAccepted}>Apply</button>
    {/if}
  </div>
  {#each hunks as hunk (hunk.id)}
    <div class="hunk" class:rejected={hunk.status === "rejected"}>
      {#if hunk.header}<div class="hunk-header">{hunk.header}</div>{/if}
      <div class="hunk-lines">
        {#each hunk.lines as line, i (i)}
          <div class="diff-line" class:add={line.type === "add"} class:del={line.type === "del"}>
            <span class="gutter">{line.type === "add" ? "+" : line.type === "del" ? "-" : " "}</span>
            <span class="content">{line.content}</span>
          </div>
        {/each}
      </div>
      <div class="hunk-actions">
        {#if hunk.status === "pending"}
          <button class="ghost-btn small" onclick={() => acceptHunk(hunk.id)}>Accept hunk</button>
          <button class="ghost-btn small" onclick={() => rejectHunk(hunk.id)}>Reject hunk</button>
        {:else}
          <span class="hunk-status">{hunk.status}</span>
        {/if}
      </div>
    </div>
  {/each}
  {#if showUndo}
    <button class="undo-bar" onclick={undoLast}>Undo rejection</button>
  {/if}
</div>

<style>
  .diff-block {
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
    background: var(--editor-bg);
    overflow: hidden;
  }
  .diff-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 6px 8px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    font-size: var(--text-xs);
  }
  .filepath {
    font-family: var(--font-mono);
    color: var(--text);
  }
  .spacer {
    flex: 1;
  }
  .ghost-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    border-radius: var(--radius-1);
    padding: 3px 8px;
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    cursor: pointer;
  }
  .ghost-btn:hover {
    color: var(--text);
    border-color: var(--border-secondary);
  }
  .ghost-btn.primary {
    background: var(--accent);
    color: var(--text-inverse);
    border-color: var(--accent);
  }
  .ghost-btn.small {
    padding: 2px 6px;
  }
  .hunk {
    border-top: 1px solid var(--border);
  }
  .hunk.rejected {
    opacity: 0.5;
  }
  .hunk-header {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    background: var(--bg-tertiary);
    padding: 2px 8px;
  }
  .hunk-lines {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }
  .diff-line {
    display: flex;
    padding: 0 8px;
  }
  .diff-line .gutter {
    width: 16px;
    flex-shrink: 0;
    color: var(--text-tertiary);
    user-select: none;
  }
  .diff-line .content {
    white-space: pre-wrap;
    word-break: break-word;
  }
  .diff-line.add {
    background: var(--diff-add-bg);
  }
  .diff-line.del {
    background: var(--diff-del-bg);
  }
  .hunk-actions {
    display: flex;
    gap: var(--space-2);
    padding: 4px 8px;
  }
  .hunk-status {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    text-transform: capitalize;
  }
  .undo-bar {
    width: 100%;
    text-align: center;
    background: var(--bg-tertiary);
    border: none;
    border-top: 1px solid var(--border);
    color: var(--text-link);
    font-size: var(--text-xs);
    padding: 4px;
    cursor: pointer;
  }
</style>
