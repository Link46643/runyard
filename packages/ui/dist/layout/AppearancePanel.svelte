<script lang="ts">
  import { theme } from "../themeStore.svelte.js";
</script>

<div class="appearance-panel">
  <h3>Appearance</h3>
  
  <div class="section">
    <span class="label">Theme</span>
    <div class="toggle-group">
      <button 
        class:active={theme.current === "light"} 
        onclick={() => theme.set("light")}
      >
        Light
      </button>
      <button 
        class:active={theme.current === "dark"} 
        onclick={() => theme.set("dark")}
      >
        Dark
      </button>
    </div>
  </div>

  <p class="hint">More options coming soon...</p>
</div>

<style>
  .appearance-panel {
    padding: 24px;
    max-width: 600px;
    margin: 0 auto;
    font-family: inherit;
  }

  h3 {
    margin-top: 0;
    margin-bottom: 24px;
    font-size: 20px;
    font-weight: 500;
  }

  .section {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 0;
    border-bottom: 1px solid var(--border);
  }

  .label {
    font-size: 14px;
    color: var(--text);
  }

  .toggle-group {
    display: flex;
    background-color: var(--bg-secondary);
    padding: 4px;
    border-radius: 8px;
    border: 1px solid var(--border);
  }

  button {
    background: none;
    border: none;
    padding: 6px 16px;
    font-size: 13px;
    border-radius: 4px;
    cursor: pointer;
    color: var(--text-secondary);
    transition: all 0.2s;
  }

  button.active {
    background-color: var(--bg);
    color: var(--text);
    box-shadow: 0 1px 3px rgba(0,0,0,0.1);
  }

  .hint {
    margin-top: 32px;
    font-size: 12px;
    color: var(--text-secondary);
    font-style: italic;
  }
</style>
