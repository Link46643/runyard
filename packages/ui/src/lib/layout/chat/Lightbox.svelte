<script lang="ts">
  import { X } from "lucide-svelte";

  let { src, onClose }: { src: string | null; onClose: () => void } = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if src}
  <div class="lightbox-overlay" onclick={onClose} role="presentation">
    <button class="lightbox-close" onclick={onClose} title="Close" aria-label="Close">
      <X size={20} strokeWidth={1.5} />
    </button>
    <img class="lightbox-image" {src} alt="" onclick={(e) => e.stopPropagation()} />
  </div>
{/if}

<style>
  .lightbox-overlay {
    position: fixed;
    inset: 0;
    background: var(--bg-overlay);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: zoom-out;
  }
  .lightbox-image {
    max-width: 90vw;
    max-height: 90vh;
    object-fit: contain;
    cursor: default;
  }
  .lightbox-close {
    position: absolute;
    top: var(--space-5);
    right: var(--space-5);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    width: 36px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text);
    cursor: pointer;
  }
</style>
