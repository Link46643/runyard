<script lang="ts">
  import type { DiffBlock } from "@runyard/common";
  import { parseUnifiedDiff, pairHunkLines, type ParsedDiffHunk } from "./diffParser.js";
  import { Columns2, Rows2 } from "lucide-svelte";

  let { block }: { block: DiffBlock } = $props();

  let hunks = $state<ParsedDiffHunk[]>(parseUnifiedDiff(block.diff));
  let showUndo = $state(false);
  let sideBySide = $state(false);
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

  // Parse the @@ -N,M +P,Q @@ header to extract original start line (1-based).
  function parseHunkOrigStart(header: string): number {
    const m = header.match(/@@ -(\d+)(?:,\d+)? \+\d+(?:,\d+)? @@/);
    return m ? parseInt(m[1], 10) : 1;
  }

  // Apply accepted hunks to original file content using proper line-based patching.
  // Hunks are applied bottom-to-top so earlier hunk offsets don't affect later ones.
  function applyHunksToContent(original: string, accepted: ParsedDiffHunk[]): string {
    const fileLines = original.split("\n");

    // Sort accepted hunks by original start line, descending (bottom-to-top application)
    const sorted = [...accepted].sort((a, b) => {
      const aStart = parseHunkOrigStart(a.header ?? "");
      const bStart = parseHunkOrigStart(b.header ?? "");
      return bStart - aStart; // descending
    });

    for (const hunk of sorted) {
      const origStart = parseHunkOrigStart(hunk.header ?? "");
      // Count how many original (del + context) lines this hunk covers
      const origLineCount = hunk.lines.filter((l) => l.type !== "add").length;
      // Build replacement: context lines + additions (no deletions)
      const replacement = hunk.lines
        .filter((l) => l.type !== "del")
        .map((l) => l.content);
      // origStart is 1-based; splice is 0-based
      fileLines.splice(origStart - 1, origLineCount, ...replacement);
    }

    return fileLines.join("\n");
  }

  async function applyAccepted() {
    const acceptedHunks = hunks.filter((h) => h.status === "accepted");
    if (acceptedHunks.length === 0) return;
    if (!block.filepath) {
      console.warn("[DiffBlock] No filepath — cannot write to disk");
      return;
    }
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const original = await invoke<string>("fs_read", { path: block.filepath }).catch(() => "");
      const patched = applyHunksToContent(original, acceptedHunks);
      await invoke("fs_write", { path: block.filepath, contents: patched });
    } catch (e) {
      console.error("[DiffBlock] Failed to apply accepted hunks", e);
    }
  }

  // Write to disk immediately when an individual hunk is accepted
  async function acceptHunkAndWrite(id: string) {
    acceptHunk(id);
    // Gather all accepted hunks after this accept (state update is synchronous in Svelte 5)
    const nowAccepted = hunks
      .map((h) => (h.id === id ? { ...h, status: "accepted" as const } : h))
      .filter((h) => h.status === "accepted");
    if (!block.filepath || nowAccepted.length === 0) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const original = await invoke<string>("fs_read", { path: block.filepath }).catch(() => "");
      const patched = applyHunksToContent(original, nowAccepted);
      await invoke("fs_write", { path: block.filepath, contents: patched });
    } catch (e) {
      console.error("[DiffBlock] Failed to write hunk to disk", e);
    }
  }

  // Accept all and immediately write
  async function acceptAllAndWrite() {
    acceptAll();
    const allHunks = hunks.map((h) => ({ ...h, status: "accepted" as const }));
    if (!block.filepath || allHunks.length === 0) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const original = await invoke<string>("fs_read", { path: block.filepath }).catch(() => "");
      const patched = applyHunksToContent(original, allHunks);
      await invoke("fs_write", { path: block.filepath, contents: patched });
    } catch (e) {
      console.error("[DiffBlock] Failed to write all hunks to disk", e);
    }
  }

  let allDecided = $derived(hunks.every((h) => h.status !== "pending"));
</script>

<div class="diff-block">
  <div class="diff-header">
    <span class="filepath">{block.filepath}</span>
    <div class="spacer"></div>
    <button class="icon-toggle" onclick={() => (sideBySide = !sideBySide)} title={sideBySide ? "Unified view" : "Side-by-side view"}>
      {#if sideBySide}<Rows2 size={14} strokeWidth={1.5} />{:else}<Columns2 size={14} strokeWidth={1.5} />{/if}
    </button>
    <button class="ghost-btn" onclick={acceptAllAndWrite}>Accept all</button>
    <button class="ghost-btn" onclick={rejectAll}>Reject all</button>
    {#if allDecided}
      <button class="ghost-btn primary" onclick={applyAccepted}>Apply</button>
    {/if}
  </div>
  {#each hunks as hunk (hunk.id)}
    <div class="hunk" class:rejected={hunk.status === "rejected"}>
      {#if hunk.header}<div class="hunk-header">{hunk.header}</div>{/if}
      {#if sideBySide}
        <div class="hunk-lines side-by-side">
          {#each pairHunkLines(hunk.lines) as row, i (i)}
            <div class="sbs-row">
              <div class="diff-line" class:del={row.left?.type === "del"}>
                <span class="gutter">{row.left ? (row.left.type === "del" ? "-" : " ") : ""}</span>
                <span class="content">{row.left?.content ?? ""}</span>
              </div>
              <div class="diff-line" class:add={row.right?.type === "add"}>
                <span class="gutter">{row.right ? (row.right.type === "add" ? "+" : " ") : ""}</span>
                <span class="content">{row.right?.content ?? ""}</span>
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="hunk-lines">
          {#each hunk.lines as line, i (i)}
            <div class="diff-line" class:add={line.type === "add"} class:del={line.type === "del"}>
              <span class="gutter">{line.type === "add" ? "+" : line.type === "del" ? "-" : " "}</span>
              <span class="content">{line.content}</span>
            </div>
          {/each}
        </div>
      {/if}
      <div class="hunk-actions">
        {#if hunk.status === "pending"}
          <button class="ghost-btn small" onclick={() => acceptHunkAndWrite(hunk.id)}>Accept hunk</button>
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
  .icon-toggle {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    border-radius: var(--radius-1);
    padding: 3px 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
  }
  .icon-toggle:hover {
    color: var(--text);
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
  .side-by-side {
    display: flex;
    flex-direction: column;
  }
  .sbs-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
  }
  .sbs-row .diff-line:first-child {
    border-right: 1px solid var(--border);
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
