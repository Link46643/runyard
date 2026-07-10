<script lang="ts">
  import type { ToolResultBlock } from "@runyard/common";

  let { block }: { block: ToolResultBlock } = $props();

  const TRUNCATE_AT = 500;
  let expanded = $state(false);
  let isTruncated = $derived(block.output.length > TRUNCATE_AT);
  let displayText = $derived(expanded || !isTruncated ? block.output : block.output.slice(0, TRUNCATE_AT));
</script>

<div class="tool-result-block" class:is-error={block.is_error}>
  <pre class="result-content">{displayText}{#if isTruncated && !expanded}...{/if}</pre>
  <div class="result-footer">
    {#if isTruncated}
      <button class="link-btn" onclick={() => (expanded = !expanded)}>
        {expanded ? "Show less" : `Show full output (${block.output.length.toLocaleString()} chars)`}
      </button>
    {/if}
  </div>
</div>

<style>
  .tool-result-block {
    border-left: 2px solid var(--border);
    margin-left: var(--space-3);
    padding-left: var(--space-3);
  }
  .tool-result-block.is-error {
    border-left-color: var(--border-error);
  }
  .result-content {
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .tool-result-block.is-error .result-content {
    color: var(--text-error);
  }
  .result-footer {
    margin-top: var(--space-1);
  }
  .link-btn {
    background: none;
    border: none;
    color: var(--text-link);
    font-size: var(--text-xs);
    cursor: pointer;
    padding: 0;
  }
</style>
