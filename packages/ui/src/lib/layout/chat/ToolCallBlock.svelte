<script lang="ts">
  import type { ToolCallBlock, ToolResultBlock } from "@runyard/common";
  import { ChevronRight } from "lucide-svelte";

  let { block, result }: { block: ToolCallBlock; result?: ToolResultBlock } = $props();
  let expanded = $state(false);

  let status = $derived(result ? (result.is_error ? "failed" : "completed") : "running");

  function formatDuration(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  }
</script>

<div class="tool-call-block">
  <button class="tool-header" onclick={() => (expanded = !expanded)} aria-expanded={expanded}>
    <ChevronRight size={12} strokeWidth={2} class={expanded ? "chevron open" : "chevron"} />
    <span class="dot" class:running={status === "running"} class:completed={status === "completed"} class:failed={status === "failed"}></span>
    <span class="tool-name">{block.name}</span>
    <span class="tool-args-summary">{Object.keys(block.arguments).length} arg{Object.keys(block.arguments).length === 1 ? "" : "s"}</span>
    {#if result?.duration_ms !== undefined}
      <span class="tool-duration">Took {formatDuration(result.duration_ms)}</span>
    {/if}
  </button>
  {#if expanded}
    <pre class="tool-args">{JSON.stringify(block.arguments, null, 2)}</pre>
  {/if}
</div>

<style>
  .tool-call-block {
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
    background: var(--bg-secondary);
  }
  .tool-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
    background: none;
    border: none;
    padding: var(--space-2) var(--space-3);
    cursor: pointer;
    font-family: var(--font-sans);
    text-align: left;
  }
  .chevron {
    color: var(--text-tertiary);
    transition: transform 100ms ease;
    flex-shrink: 0;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: var(--radius-full);
    flex-shrink: 0;
    background: var(--text-tertiary);
  }
  .dot.running {
    background: var(--accent-warning);
  }
  .dot.completed {
    background: var(--accent-success);
  }
  .dot.failed {
    background: var(--accent-danger);
  }
  .tool-name {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--text);
  }
  .tool-args-summary {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
  }
  .tool-duration {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    margin-left: auto;
  }
  .tool-args {
    margin: 0;
    padding: var(--space-3);
    border-top: 1px solid var(--border);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--text-secondary);
    overflow-x: auto;
    max-height: 300px;
    overflow-y: auto;
  }
</style>
