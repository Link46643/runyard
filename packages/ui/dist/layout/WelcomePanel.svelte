<script lang="ts">
</script>

<div class="welcome-panel">
  <div class="hero">
    <h1>Runyard</h1>
    <p>A performance-first, agentic IDE.</p>
  </div>
  
  <div class="shortcuts">
    <div class="shortcut">
      <span class="label">Open File</span>
      <span class="key">Ctrl + O</span>
    </div>
    <div class="shortcut">
      <span class="label">Settings</span>
      <span class="key">Ctrl + ,</span>
    </div>
  </div>
</div>

<style>
  .welcome-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    background-color: var(--bg);
    color: var(--text-secondary);
    font-family: inherit;
  }

  .hero {
    text-align: center;
    margin-bottom: 48px;
  }

  h1 {
    font-size: 64px;
    font-weight: 800;
    color: var(--text);
    margin: 0 0 12px 0;
    letter-spacing: -2px;
    background: linear-gradient(135deg, var(--text) 0%, var(--text-secondary) 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
  }

  p {
    font-size: 20px;
    margin: 0;
    opacity: 0.8;
  }

  .shortcuts {
    display: flex;
    flex-direction: column;
    gap: 12px;
    width: 320px;
  }

  .shortcut {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 16px;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    transition: border-color 0.2s;
  }
  
  .shortcut:hover {
    border-color: var(--accent);
  }

  .label {
    font-size: 13px;
  }

  .key {
    font-size: 11px;
    background-color: var(--bg);
    padding: 2px 6px;
    border-radius: 4px;
    border: 1px solid var(--border);
    font-family: inherit;
  }
</style>
