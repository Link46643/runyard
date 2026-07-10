<script lang="ts">
  import { FileDiff } from "lucide-svelte";
</script>

<div class="diff-viewer-panel">
  <div class="panel-header">
    <span class="panel-title">DIFF VIEWER</span>
  </div>
  <div class="placeholder-body">
    <FileDiff size={20} color="var(--text-tertiary)" />
    <span class="placeholder-text">Open a diff from the chat panel or git panel.</span>
  </div>
</div>

<style>
  .diff-viewer-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
  }

  .panel-header {
    display: flex;
    align-items: center;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--bg-secondary);
  }

  .panel-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    text-transform: uppercase;
  }

  .placeholder-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px;
  }

  .placeholder-text {
    font-size: 13px;
    color: var(--text-secondary);
    text-align: center;
  }
</style>
