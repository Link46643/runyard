<script lang="ts">
  import type { ContextSummaryBlock } from "@runyard/common";

  let { block }: { block: ContextSummaryBlock } = $props();
  let expanded = $state(false);
</script>

<div class="context-summary">
  <div class="divider"></div>
  <button class="summary-label" onclick={() => (expanded = !expanded)} aria-expanded={expanded}>
    {block.original_count} {block.original_count === 1 ? "message" : "messages"} summarized
  </button>
  {#if expanded}
    <div class="summary-text">{block.summary}</div>
  {/if}
</div>

<style>
  .context-summary {
    text-align: center;
    padding: var(--space-3) 0;
  }
  .divider {
    border-top: 1px dashed var(--border);
    margin-bottom: var(--space-2);
  }
  .summary-label {
    background: none;
    border: none;
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    color: var(--text-tertiary);
    cursor: pointer;
  }
  .summary-label:hover {
    color: var(--text-secondary);
  }
  .summary-text {
    margin-top: var(--space-2);
    font-size: var(--text-sm);
    color: var(--text-tertiary);
    text-align: left;
    padding: var(--space-3);
    background: var(--bg-tertiary);
  }
</style>
