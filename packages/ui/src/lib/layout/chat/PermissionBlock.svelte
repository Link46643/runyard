<script lang="ts">
  import type { PermissionBlock } from "@runyard/common";

  let { block, onDecide }: { block: PermissionBlock; onDecide?: (approved: boolean) => void } = $props();
</script>

<div class="permission-block">
  <div class="permission-title">Permission required</div>
  <div class="permission-body">
    <div class="permission-row"><span class="label">Tool</span><code>{block.tool_id}</code></div>
    <div class="permission-row"><span class="label">Action</span><span>{block.action}</span></div>
  </div>
  {#if block.approved === null}
    <div class="permission-actions">
      <button class="btn-primary" onclick={() => onDecide?.(true)}>Approve</button>
      <button class="btn-secondary" onclick={() => onDecide?.(false)}>Deny</button>
    </div>
  {:else}
    <div class="permission-result" class:denied={!block.approved}>
      {block.approved ? "Approved" : "Denied"}
    </div>
  {/if}
</div>

<style>
  .permission-block {
    border: 1px solid var(--accent-warning);
    background: color-mix(in srgb, var(--accent-warning) 8%, var(--bg));
    border-radius: var(--radius-0);
    padding: var(--space-4);
  }
  .permission-title {
    font-size: var(--text-base);
    font-weight: 600;
    color: var(--text);
    margin-bottom: var(--space-3);
  }
  .permission-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin-bottom: var(--space-4);
  }
  .permission-row {
    display: flex;
    gap: var(--space-2);
    font-size: var(--text-base);
  }
  .permission-row .label {
    color: var(--text-secondary);
    min-width: 60px;
  }
  .permission-row code {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    background: var(--bg-tertiary);
    padding: 1px 4px;
    border-radius: var(--radius-1);
  }
  .permission-actions {
    display: flex;
    gap: var(--space-3);
  }
  .btn-primary, .btn-secondary {
    font-family: var(--font-sans);
    font-size: var(--text-base);
    padding: 6px 12px;
    border-radius: var(--radius-1);
    cursor: pointer;
  }
  .btn-primary {
    background: var(--accent);
    border: none;
    color: var(--text-inverse);
  }
  .btn-secondary {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text);
  }
  .permission-result {
    font-size: var(--text-sm);
    color: var(--text-success);
    font-weight: 500;
  }
  .permission-result.denied {
    color: var(--text-error);
  }
</style>
