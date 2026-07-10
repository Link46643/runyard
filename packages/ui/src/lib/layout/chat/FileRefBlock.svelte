<script lang="ts">
  import type { FileRefBlock } from "@runyard/common";
  import { FileCode } from "lucide-svelte";

  let { block, onOpen }: { block: FileRefBlock; onOpen?: (path: string) => void } = $props();

  let showPreview = $state(false);
  let previewLines = $state<string[] | null>(null);
  let previewError = $state<string | null>(null);
  let hoverTimer: ReturnType<typeof setTimeout> | null = null;

  function shortName(path: string): string {
    const parts = path.split("/");
    return parts[parts.length - 1] || path;
  }

  async function loadPreview() {
    if (previewLines || previewError) return; // cache once loaded
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const content = await invoke<string>("fs_read", { path: block.filepath });
      previewLines = content.split("\n").slice(0, 10);
    } catch (e) {
      previewError = "Could not read file";
    }
  }

  function handleMouseEnter() {
    hoverTimer = setTimeout(() => {
      showPreview = true;
      loadPreview();
    }, 300);
  }

  function handleMouseLeave() {
    if (hoverTimer) clearTimeout(hoverTimer);
    showPreview = false;
  }
</script>

<span class="file-ref-wrapper" onmouseenter={handleMouseEnter} onmouseleave={handleMouseLeave} role="presentation">
  <button class="file-ref-chip" onclick={() => onOpen?.(block.filepath)} title={block.filepath}>
    <FileCode size={14} strokeWidth={1.5} />
    <span class="filename">{shortName(block.filepath)}</span>
    <span class="filepath">{block.filepath}</span>
  </button>
  {#if showPreview}
    <div class="file-preview">
      <div class="file-preview-path">{block.filepath}</div>
      {#if previewError}
        <div class="file-preview-error">{previewError}</div>
      {:else if previewLines}
        <pre class="file-preview-content">{previewLines.join("\n")}</pre>
      {:else}
        <div class="file-preview-loading">Loading...</div>
      {/if}
    </div>
  {/if}
</span>

<style>
  .file-ref-wrapper {
    position: relative;
    display: inline-block;
  }
  .file-ref-chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    padding: 2px 8px;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--text);
    cursor: pointer;
    max-width: 100%;
    overflow: hidden;
  }
  .file-ref-chip:hover {
    border-color: var(--border-secondary);
    background: var(--bg-elevated);
  }
  .filename {
    font-weight: 500;
    flex-shrink: 0;
  }
  .filepath {
    color: var(--text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file-preview {
    position: absolute;
    bottom: calc(100% + 4px);
    left: 0;
    z-index: 100;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-2);
    box-shadow: var(--shadow-1);
    padding: var(--space-2) var(--space-3);
    min-width: 280px;
    max-width: 480px;
  }
  .file-preview-path {
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text-secondary);
    margin-bottom: var(--space-2);
    border-bottom: 1px solid var(--border);
    padding-bottom: var(--space-2);
  }
  .file-preview-content {
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    color: var(--text);
    white-space: pre;
    overflow-x: auto;
  }
  .file-preview-loading,
  .file-preview-error {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
  }
</style>
