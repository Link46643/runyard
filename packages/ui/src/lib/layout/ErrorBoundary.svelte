<script lang="ts">
  let { fallback }: { fallback?: string } = $props();
  let hasError = $state(false);
  let errorMessage = $state("");
</script>

<svelte:boundary onerror={(error) => { hasError = true; errorMessage = (error as any)?.message ?? "Rendering failed."; }}>
  {#if hasError}
    <div class="error-boundary">
      <span class="error-msg">{errorMessage}</span>
      <button class="reload-btn" onclick={() => { hasError = false; errorMessage = ""; }}>Reload panel</button>
    </div>
  {:else}
    <slot />
  {/if}
</svelte:boundary>

<style>
  .error-boundary {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-4);
    background: color-mix(in srgb, var(--accent-danger) 5%, transparent);
    border: 1px solid var(--border-error);
    border-radius: var(--radius-0);
  }

  .error-msg {
    font-size: 12px;
    color: var(--text-error);
    font-family: var(--font-mono);
    word-break: break-word;
  }

  .reload-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    border-radius: var(--radius-0);
    padding: 3px 10px;
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    cursor: pointer;
  }

  .reload-btn:hover {
    color: var(--text);
    border-color: var(--border-active);
  }
</style>
