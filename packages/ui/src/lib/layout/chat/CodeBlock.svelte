<script lang="ts">
  import type { CodeBlock } from "@runyard/common";
  import { Copy, Check, CornerDownLeft, Save, WrapText } from "lucide-svelte";

  let { block, onExplain }: { block: CodeBlock; onExplain?: (code: string, language: string) => void } = $props();

  let copied = $state(false);
  let wordWrap = $state(false);

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
  <div class="code-content" class:wrap={wordWrap} class:tall={lineCount > 20}>
    <pre><code>{block.code}</code></pre>
  </div>
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
  }
  .code-content pre {
    margin: 0;
    padding: var(--space-4);
  }
  .code-content code {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--text);
    white-space: pre;
  }
  .code-content.wrap code {
    white-space: pre-wrap;
    word-break: break-word;
  }
</style>
