<script lang="ts">
  import type { ThinkingBlock } from "@runyard/common";
  import { ChevronRight } from "lucide-svelte";

  let { block }: { block: ThinkingBlock } = $props();
  let expanded = $state(false);
</script>

<div class="thinking-block">
  <button class="thinking-header" onclick={() => (expanded = !expanded)} aria-expanded={expanded}>
    <ChevronRight size={12} strokeWidth={2} class={expanded ? "chevron open" : "chevron"} />
    <span>Thinking{block.token_count ? ` (~${block.token_count.toLocaleString()} reasoning tokens)` : "..."}</span>
  </button>
  {#if expanded}
    <div class="thinking-content">{block.thought}</div>
  {/if}
</div>

<style>
  .thinking-block {
    font-size: var(--text-sm);
    color: var(--text-tertiary);
  }
  .thinking-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    background: none;
    border: none;
    color: var(--text-tertiary);
    font-size: var(--text-sm);
    font-family: var(--font-sans);
    cursor: pointer;
    padding: var(--space-1) 0;
  }
  .chevron {
    transition: transform 100ms ease;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .thinking-content {
    margin-top: var(--space-2);
    padding: var(--space-3);
    background: var(--bg-tertiary);
    border-radius: var(--radius-0);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--text-secondary);
    max-height: 300px;
    overflow-y: auto;
    white-space: pre-wrap;
  }
</style>
