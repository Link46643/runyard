<script lang="ts">
  let { checked = $bindable(false), label, id } = $props<{
    checked?: boolean;
    label?: string;
    id?: string;
  }>();

  const internalId = id || Math.random().toString(36).substring(2, 9);
</script>

<div class="checkbox-container">
  <input 
    type="checkbox" 
    id={internalId} 
    bind:checked={checked} 
    class="sr-only" 
  />
  <label for={internalId} class="checkbox-label">
    <div class="checkbox-box" class:checked={checked}>
      {#if checked}
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="4" stroke-linecap="round" stroke-linejoin="round">
          <polyline points="20 6 9 17 4 12"></polyline>
        </svg>
      {/if}
    </div>
    {#if label}
      <span class="label-text">{label}</span>
    {/if}
  </label>
</div>

<style>
  .checkbox-container {
    display: inline-flex;
    align-items: center;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border-width: 0;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    user-select: none;
    font-size: 13px;
    color: var(--text-secondary);
    transition: color 0.2s;
  }

  .checkbox-label:hover {
    color: var(--text);
  }

  .checkbox-box {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    border: 1px solid var(--border);
    background-color: var(--bg);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
    flex-shrink: 0;
  }

  .checkbox-box.checked {
    background-color: var(--accent);
    border-color: var(--accent);
  }

  .checkbox-box svg {
    width: 10px;
    height: 10px;
    color: white;
  }

  .checkbox-label:hover .checkbox-box {
    border-color: var(--accent);
  }

  .label-text {
    line-height: 1;
  }
</style>
