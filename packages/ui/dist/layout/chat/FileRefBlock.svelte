<script lang="ts">
  import type { FileRefBlock } from "@runyard/common";
  import { FileCode } from "lucide-svelte";

  let { block, onOpen }: { block: FileRefBlock; onOpen?: (path: string) => void } = $props();

  function shortName(path: string): string {
    const parts = path.split("/");
    return parts[parts.length - 1] || path;
  }
</script>

<button class="file-ref-chip" onclick={() => onOpen?.(block.filepath)} title={block.filepath}>
  <FileCode size={14} strokeWidth={1.5} />
  <span class="filename">{shortName(block.filepath)}</span>
  <span class="filepath">{block.filepath}</span>
</button>

<style>
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
</style>
