<script lang="ts">
  import { onMount } from "svelte";
  import { StatusBar, CommandPalette, theme } from "@runyard/ui";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import "@fontsource-variable/google-sans-flex";
  import "@runyard/ui/global.css";

  let { children } = $props();

  let commandPaletteOpen = $state(false);

  onMount(() => {
    // Show window after mount to prevent white flash
    setTimeout(() => {
      getCurrentWindow().show();
    }, 100);

    // Global Ctrl+K / Cmd+K handler for command palette
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "k") {
        e.preventDefault();
        commandPaletteOpen = !commandPaletteOpen;
      }
      if (e.key === "Escape" && commandPaletteOpen) {
        commandPaletteOpen = false;
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });
</script>

<svelte:head>
  <title>Runyard</title>
</svelte:head>

<div class="app-container">
  <main class="content">
    {@render children()}
  </main>
  <StatusBar />
</div>

<CommandPalette bind:open={commandPaletteOpen} />

<style>
  .app-container {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    background-color: var(--bg);
    color: var(--text);
    position: absolute;
    top: 0;
    left: 0;
  }

  .content {
    flex: 1;
    display: flex;
    overflow: hidden;
    position: relative;
  }
</style>
