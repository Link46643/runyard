<script lang="ts">
  let { message = "Loading..." }: { message?: string } = $props();
</script>

<div class="loading-panel">
  <div class="progress-bar-track">
    <div class="progress-bar-fill"></div>
  </div>
  <span class="loading-label">{message}</span>
</div>

<style>
  .loading-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    position: relative;
  }

  .progress-bar-track {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--border);
    overflow: hidden;
  }

  .progress-bar-fill {
    height: 100%;
    background: var(--accent);
    width: 0%;
    animation: load-progress 300ms ease-out forwards;
  }

  @keyframes load-progress {
    from { width: 0%; }
    to   { width: 70%; }
  }

  .loading-label {
    font-size: 13px;
    color: var(--text-secondary);
    font-family: var(--font-sans);
  }
</style>
