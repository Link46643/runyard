<script lang="ts">
  import type { TextBlock } from "@runyard/common";
  import { renderMarkdownLite } from "./markdownLite.js";

  let { block }: { block: TextBlock } = $props();
  let html = $derived(renderMarkdownLite(block.text));
</script>

<div class="text-block">
  {@html html}
</div>

<style>
  .text-block {
    font-family: var(--font-sans);
    font-size: var(--text-md);
    color: var(--text);
    line-height: 1.6;
  }
  .text-block :global(p) {
    margin: 0 0 var(--space-3) 0;
  }
  .text-block :global(p:last-child) {
    margin-bottom: 0;
  }
  .text-block :global(h1),
  .text-block :global(h2),
  .text-block :global(h3),
  .text-block :global(h4),
  .text-block :global(h5),
  .text-block :global(h6) {
    font-weight: 600;
    margin: var(--space-4) 0 var(--space-2) 0;
  }
  .text-block :global(a) {
    color: var(--text-link);
    text-decoration: none;
  }
  .text-block :global(a:hover) {
    text-decoration: underline;
  }
  .text-block :global(code) {
    font-family: var(--font-mono);
    font-size: 0.9em;
    background: var(--bg-tertiary);
    padding: 1px 4px;
    border-radius: var(--radius-1);
  }
  .text-block :global(pre.md-code) {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    background: var(--editor-bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
    padding: var(--space-4);
    overflow-x: auto;
    margin: var(--space-3) 0;
  }
  .text-block :global(pre.md-code code) {
    background: none;
    padding: 0;
  }
  .text-block :global(ul),
  .text-block :global(ol) {
    margin: 0 0 var(--space-3) 0;
    padding-left: var(--space-6);
  }
  .text-block :global(li) {
    margin-bottom: var(--space-1);
  }
  .text-block :global(blockquote) {
    border-left: 2px solid var(--border);
    padding-left: var(--space-4);
    color: var(--text-secondary);
    margin: 0 0 var(--space-3) 0;
  }
  .text-block :global(table) {
    border-collapse: collapse;
    margin: var(--space-3) 0;
    font-size: var(--text-base);
  }
  .text-block :global(th),
  .text-block :global(td) {
    border: 1px solid var(--border);
    padding: var(--space-2) var(--space-3);
    text-align: left;
  }
  .text-block :global(th) {
    background: var(--bg-tertiary);
    font-weight: 600;
  }
</style>
