<script lang="ts">
  let { title, message, onConfirm, onCancel, confirmLabel = "OK", cancelLabel = "Cancel", show = $bindable(false), children } = $props<{
    title: string,
    message: string,
    onConfirm: () => void,
    onCancel?: () => void,
    confirmLabel?: string,
    cancelLabel?: string,
    show: boolean,
    children?: import("svelte").Snippet
  }>();

  function handleConfirm() {
    show = false;
    onConfirm();
  }

  function handleCancel() {
    show = false;
    if (onCancel) onCancel();
  }
</script>

{#if show}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div 
    class="modal-backdrop" 
    onclick={handleCancel}
    role="button"
    tabindex="-1"
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="modal-content" onclick={(e) => e.stopPropagation()}>
      <div class="modal-header">
        <h2>{title}</h2>
      </div>
      <div class="modal-body">
        <p>{message}</p>
        {#if children}
          {@render children()}
        {/if}
      </div>
      <div class="modal-footer">
        {#if onCancel}
          <button class="cancel" onclick={handleCancel}>{cancelLabel}</button>
        {/if}
        <button class="confirm" onclick={handleConfirm}>{confirmLabel}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(2px);
  }

  .modal-content {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 12px;
    width: 400px;
    max-width: 90vw;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: var(--text);
  }

  .modal-body {
    padding: 20px;
    color: var(--text-secondary);
    font-size: 14px;
    line-height: 1.5;
    word-break: break-word;
  }

  .modal-footer {
    padding: 12px 20px;
    background: var(--bg-secondary);
    display: flex;
    justify-content: flex-end;
    gap: 12px;
    border-top: 1px solid var(--border);
  }

  button {
    padding: 8px 16px;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.2s;
  }

  .confirm {
    background: var(--accent);
    color: white;
    border: none;
  }

  .confirm:hover {
    filter: brightness(1.1);
  }

  .cancel {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .cancel:hover {
    background: rgba(128, 128, 128, 0.1);
    color: var(--text);
  }
</style>
