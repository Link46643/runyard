<script lang="ts">
  import type { ErrorBlock } from "@runyard/common";
  import { TriangleAlert } from "lucide-svelte";

  let { block }: { block: ErrorBlock } = $props();
</script>

<div class="error-block">
  <TriangleAlert size={14} strokeWidth={1.5} class="error-icon" />
  <div class="error-body">
    <div class="error-code">{block.code}</div>
    <div class="error-message">{block.message}</div>
  </div>
</div>

<style>
  .error-block {
    display: flex;
    gap: var(--space-3);
    align-items: flex-start;
    border: 1px solid var(--border-error);
    background: color-mix(in srgb, var(--accent-danger) 5%, var(--bg));
    border-radius: var(--radius-0);
    padding: var(--space-3) var(--space-4);
  }
  :global(.error-icon) {
    color: var(--text-error);
    flex-shrink: 0;
    margin-top: 2px;
  }
  .error-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-width: 0;
  }
  .error-code {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    color: var(--text-error);
    font-weight: 600;
  }
  .error-message {
    font-size: var(--text-base);
    color: var(--text);
  }
</style>
