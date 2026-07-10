<script lang="ts">
  import { invoke as tauriInvoke } from "@tauri-apps/api/core";
  import { webSocketClient } from "@runyard/common";
  import { onMount, onDestroy } from "svelte";

  async function invoke<T>(cmd: string, args?: any): Promise<T> {
    if (webSocketClient.status === "connected") {
      return webSocketClient.invoke<T>(cmd, args);
    } else {
      return tauriInvoke<T>(cmd, args);
    }
  }

  const WORKSPACE_PATH = "../../";

  let content = $state("");
  let savedContent = $state("");
  let isDirty = $derived(content !== savedContent);
  let showPreview = $state(false);
  let saveStatus = $state<"saved" | "edited" | "idle">("idle");

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let savedStatusTimer: ReturnType<typeof setTimeout> | null = null;

  function simpleMarkdown(raw: string): string {
    let html = raw
      // Escape HTML entities first
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      // Headings
      .replace(/^### (.+)$/gm, "<h3>$1</h3>")
      .replace(/^## (.+)$/gm, "<h2>$1</h2>")
      .replace(/^# (.+)$/gm, "<h1>$1</h1>")
      // Bold + italic (order matters: bold first)
      .replace(/\*\*\*(.+?)\*\*\*/g, "<strong><em>$1</em></strong>")
      .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
      .replace(/\*(.+?)\*/g, "<em>$1</em>")
      // Inline code
      .replace(/`([^`]+)`/g, "<code>$1</code>")
      // Unordered list items
      .replace(/^- (.+)$/gm, "<li>$1</li>")
      // Wrap consecutive <li> in <ul>
      .replace(/(<li>.*<\/li>\n?)+/g, (match) => `<ul>${match}</ul>`)
      // Newlines to <br> (skip lines that are already block elements)
      .replace(/(?<!>)\n(?!<)/g, "<br>");
    return html;
  }

  let previewHtml = $derived(simpleMarkdown(content));

  async function save() {
    if (!isDirty) return;
    try {
      await invoke("note_save", { workspacePath: WORKSPACE_PATH, content });
      savedContent = content;
      saveStatus = "saved";
      if (savedStatusTimer) clearTimeout(savedStatusTimer);
      savedStatusTimer = setTimeout(() => {
        saveStatus = isDirty ? "edited" : "idle";
      }, 2000);
    } catch (e) {
      console.error("[NotesPanel] Failed to save note", e);
    }
  }

  function onInput() {
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      save();
    }, 2000);
    // Reflect "edited" state immediately
    if (saveStatus === "saved") saveStatus = "edited";
  }

  onMount(async () => {
    try {
      const note = await invoke<{ id: string; workspace_path: string; content: string; created_at: number; updated_at: number } | null>(
        "note_load",
        { workspacePath: WORKSPACE_PATH }
      );
      if (note) {
        content = note.content;
        savedContent = note.content;
      }
    } catch (e) {
      console.error("[NotesPanel] Failed to load note", e);
    }
  });

  onDestroy(() => {
    if (debounceTimer) clearTimeout(debounceTimer);
    if (savedStatusTimer) clearTimeout(savedStatusTimer);
    // Save on unmount if dirty
    if (isDirty) {
      invoke("note_save", { workspacePath: WORKSPACE_PATH, content }).catch((e) => {
        console.error("[NotesPanel] Failed to save note on unmount", e);
      });
    }
  });

  $effect(() => {
    if (isDirty && saveStatus !== "saved") {
      saveStatus = "edited";
    }
  });
</script>

<div class="notes-panel">
  <!-- Header -->
  <div class="panel-header">
    <span class="panel-title">NOTES</span>
    <div class="header-right">
      {#if saveStatus === "saved"}
        <span class="status-saved">Saved</span>
      {:else if saveStatus === "edited" && isDirty}
        <span class="status-edited">Edited</span>
      {/if}
      <button
        class="ghost-btn"
        class:active-toggle={showPreview}
        onclick={() => { showPreview = !showPreview; }}
      >
        {showPreview ? "Edit" : "Preview"}
      </button>
      {#if isDirty}
        <button class="ghost-btn save-btn" onclick={save}>Save</button>
      {/if}
    </div>
  </div>

  <!-- Content area -->
  <div class="content-area">
    {#if showPreview}
      <div class="preview-area">
        {@html previewHtml}
      </div>
    {:else}
      <textarea
        class="editor-textarea"
        bind:value={content}
        oninput={onInput}
        placeholder="Start writing your notes…"
        spellcheck="false"
      ></textarea>
    {/if}
  </div>
</div>

<style>
  .notes-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
  }

  /* Header */
  .panel-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--bg-secondary);
  }

  .panel-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    text-transform: uppercase;
    flex: 1;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .status-saved {
    font-size: 11px;
    color: var(--text-success);
  }

  .status-edited {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .ghost-btn {
    font-size: 11px;
    color: var(--text-secondary);
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    padding: 2px 8px;
    cursor: pointer;
    font-family: var(--font-sans);
    white-space: nowrap;
  }

  .ghost-btn:hover {
    color: var(--text);
    border-color: var(--border-active);
    background: var(--bg-elevated);
  }

  .ghost-btn.active-toggle {
    border-color: var(--accent);
    color: var(--accent);
  }

  .save-btn {
    border-color: var(--accent);
    color: var(--accent);
  }

  .save-btn:hover {
    background: var(--accent);
    color: var(--bg);
  }

  /* Content area */
  .content-area {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .editor-textarea {
    flex: 1;
    width: 100%;
    height: 100%;
    background: var(--bg);
    color: var(--text);
    border: none;
    outline: none;
    resize: none;
    padding: 12px;
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 1.6;
    box-sizing: border-box;
  }

  .editor-textarea::placeholder {
    color: var(--text-tertiary);
  }

  .preview-area {
    flex: 1;
    overflow-y: auto;
    padding: 12px 14px;
    font-family: var(--font-sans);
    font-size: 13px;
    color: var(--text);
    line-height: 1.6;
  }

  /* Preview markdown styles */
  .preview-area :global(h1) {
    font-size: 18px;
    font-weight: 700;
    color: var(--text);
    margin: 0 0 8px;
    padding-bottom: 4px;
    border-bottom: 1px solid var(--border);
  }

  .preview-area :global(h2) {
    font-size: 15px;
    font-weight: 700;
    color: var(--text);
    margin: 12px 0 6px;
  }

  .preview-area :global(h3) {
    font-size: 13px;
    font-weight: 700;
    color: var(--text);
    margin: 10px 0 4px;
  }

  .preview-area :global(strong) {
    font-weight: 700;
    color: var(--text);
  }

  .preview-area :global(em) {
    font-style: italic;
    color: var(--text-secondary);
  }

  .preview-area :global(code) {
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    padding: 1px 4px;
    color: var(--accent);
  }

  .preview-area :global(ul) {
    margin: 4px 0;
    padding-left: 18px;
  }

  .preview-area :global(li) {
    margin: 2px 0;
    color: var(--text-secondary);
  }
</style>
