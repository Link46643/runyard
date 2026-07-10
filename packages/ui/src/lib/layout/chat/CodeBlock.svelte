<script lang="ts">
  import type { CodeBlock } from "@runyard/common";
  import { Copy, Check, CornerDownLeft, Save, WrapText, Maximize2, Minimize2 } from "lucide-svelte";
  import { setupEditor } from "@runyard/editor";
  import { onDestroy } from "svelte";

  let { block, onExplain }: { block: CodeBlock; onExplain?: (code: string, language: string) => void } = $props();

  let copied = $state(false);
  let wordWrap = $state(false);
  let expanded = $state(false);
  let container: HTMLDivElement;
  let editorInstance: ReturnType<typeof setupEditor> | null = null;

  async function copyCode() {
    try {
      await navigator.clipboard.writeText(block.code);
      copied = true;
      setTimeout(() => (copied = false), 800);
    } catch (e) {
      console.error("[CodeBlock] Clipboard write failed", e);
    }
  }

  function insertAtCursor() {
    document.dispatchEvent(new CustomEvent("runyard:insert-at-cursor", { detail: { text: block.code } }));
  }

  async function applyToFile() {
    if (!block.filename) return;
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke("fs_write", { path: block.filename, contents: block.code });
    } catch (e) {
      console.error("[CodeBlock] Failed to write file", e);
    }
  }

  const lineCount = $derived(block.code.split("\n").length);

  function mountEditor() {
    if (editorInstance) {
      editorInstance.destroy();
      editorInstance = null;
    }
    if (!container) return;
    editorInstance = setupEditor({
      parent: container,
      doc: block.code,
      language: block.language,
      readOnly: true,
    });
  }

  $effect(() => {
    // Re-mount when code, language, or wrap mode changes (wrap needs a CSS
    // class on the scroller, handled separately, but code/language changes
    // need a fresh editor since setupEditor doesn't support re-detecting language).
    void block.code;
    void block.language;
    mountEditor();
  });

  onDestroy(() => {
    editorInstance?.destroy();
  });
</script>

<div class="code-block">
  <div class="code-header">
    <span class="lang">{block.language}</span>
    {#if block.filename}
      <span class="filename">{block.filename}</span>
    {/if}
    <div class="spacer"></div>
    <button class="icon-btn" onclick={() => (wordWrap = !wordWrap)} title="Toggle word wrap" aria-pressed={wordWrap}>
      <WrapText size={14} strokeWidth={1.5} />
    </button>
    {#if lineCount > 20}
      <button class="icon-btn" onclick={() => (expanded = !expanded)} title={expanded ? "Collapse" : "Expand"}>
        {#if expanded}<Minimize2 size={14} strokeWidth={1.5} />{:else}<Maximize2 size={14} strokeWidth={1.5} />{/if}
      </button>
    {/if}
    {#if onExplain}
      <button class="icon-btn" onclick={() => onExplain?.(block.code, block.language)} title="Explain this code">Explain</button>
    {/if}
    {#if block.filename}
      <button class="icon-btn" onclick={applyToFile} title="Apply to file">
        <Save size={14} strokeWidth={1.5} />
      </button>
    {/if}
    <button class="icon-btn" onclick={insertAtCursor} title="Insert at cursor">
      <CornerDownLeft size={14} strokeWidth={1.5} />
    </button>
    <button class="icon-btn" onclick={copyCode} title="Copy">
      {#if copied}<Check size={14} strokeWidth={1.5} />{:else}<Copy size={14} strokeWidth={1.5} />{/if}
    </button>
  </div>
  <div
    bind:this={container}
    class="code-content"
    class:wrap={wordWrap}
    class:expanded
  ></div>
</div>

<style>
  .code-block {
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
    background: var(--editor-bg);
    overflow: hidden;
  }
  .code-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: 4px 8px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    font-size: var(--text-xs);
    color: var(--text-secondary);
  }
  .lang {
    font-family: var(--font-mono);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .filename {
    font-family: var(--font-mono);
    color: var(--text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .icon-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 2px 4px;
    font-family: var(--font-sans);
    font-size: var(--text-xs);
  }
  .icon-btn:hover {
    color: var(--text);
  }
  .code-content {
    max-height: 400px;
    overflow: auto;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }
  .code-content.expanded {
    max-height: none;
  }
  .code-content :global(.cm-editor) {
    height: 100%;
  }
  .code-content :global(.cm-scroller) {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }
  .code-content.wrap :global(.cm-line) {
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
