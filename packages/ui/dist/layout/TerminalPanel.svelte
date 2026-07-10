<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { UnlistenFn } from "@tauri-apps/api/event";
  import { settingsStore } from "./settingsStore.svelte.js";
  import { layoutEngine } from "./layoutStore.svelte.js";
  import type { TerminalSessionInfo } from "@runyard/common";

  let {
    terminalId,
    cwd,
    onExit,
  }: {
    terminalId: string;
    cwd?: string;
    onExit?: () => void;
  } = $props();

  let container: HTMLDivElement;
  let terminal: any = null;
  let fitAddon: any = null;
  let unlistenOutput: UnlistenFn | null = null;
  let unlistenExit: UnlistenFn | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let exited = $state(false);
  let exitCode = $state<number | null>(null);
  let isLoading = $state(true);

  // Global terminal instance cache: survive tab switches
  const INSTANCE_CACHE = (window as any).__runyard_terminals ??= new Map<string, any>();

  onMount(async () => {
    // Dynamic import keeps xterm out of the initial bundle
    const { Terminal } = await import("@xterm/xterm");
    const { FitAddon } = await import("@xterm/addon-fit");

    const fontSize = settingsStore.settings.terminal.font_size || 13;
    const scrollback = settingsStore.settings.terminal.scrollback_limit || 5000;
    // Use font family from settings if set, otherwise fall back to default mono stack
    const settingsFontFamily = settingsStore.settings.appearance?.font_family;
    const fontFamily = settingsFontFamily && settingsFontFamily !== "JetBrains Mono"
      ? settingsFontFamily
      : "JetBrains Mono, ui-monospace, Menlo, monospace";

    // Reuse cached instance if available (tab switch)
    let inst = INSTANCE_CACHE.get(terminalId);
    if (!inst) {
      const term = new Terminal({
        cursorBlink: true,
        fontFamily,
        fontSize,
        scrollback,
        theme: {
          background: "#000000",
          foreground: "#e5e7eb",
          cursor: "#e5e7eb",
          selectionBackground: "#3b82f6aa",
          black: "#000000",
          brightBlack: "#6b7280",
          red: "#ef4444",
          brightRed: "#f87171",
          green: "#22c55e",
          brightGreen: "#4ade80",
          yellow: "#eab308",
          brightYellow: "#facc15",
          blue: "#3b82f6",
          brightBlue: "#60a5fa",
          magenta: "#a855f7",
          brightMagenta: "#c084fc",
          cyan: "#06b6d4",
          brightCyan: "#22d3ee",
          white: "#e5e7eb",
          brightWhite: "#ffffff",
        },
        allowProposedApi: true,
        windowsPty: undefined,
      });
      const fit = new FitAddon();
      term.loadAddon(fit);
      inst = { terminal: term, fitAddon: fit };
      INSTANCE_CACHE.set(terminalId, inst);
    }

    terminal = inst.terminal;
    fitAddon = inst.fitAddon;

    // Attach/detach to DOM
    terminal.open(container);
    fitAddon.fit();
    isLoading = false;

    // Update the tab title when the shell reports its current process via OSC sequences
    // (e.g. zsh/bash with "precmd" hooks emit \e]0;title\a — xterm.js parses these natively)
    terminal.onTitleChange((title: string) => {
      if (title && title.trim()) {
        layoutEngine.setTabTitle(`terminal:${terminalId}`, title.trim());
      }
    });

    // Handle user input → write to PTY
    terminal.onData((data: string) => {
      if (exited) return;
      invoke("terminal_write", { id: terminalId, data }).catch(console.error);
    });

    // Listen for PTY output
    unlistenOutput = await listen<{ id: string; data: string }>(
      "terminal:output",
      (event) => {
        if (event.payload.id === terminalId) {
          terminal.write(event.payload.data);
        }
      }
    );

    // Listen for PTY exit
    unlistenExit = await listen<{ id: string; exit_code: number }>(
      "terminal:exit",
      (event) => {
        if (event.payload.id === terminalId) {
          exited = true;
          exitCode = event.payload.exit_code;
          terminal.write("\r\n\x1b[90m[Process exited]\x1b[0m\r\n");
          onExit?.();
        }
      }
    );

    // Resize when container changes
    resizeObserver = new ResizeObserver(() => {
      if (!fitAddon || !terminal) return;
      // Small delay to let the layout settle
      requestAnimationFrame(() => {
        try {
          fitAddon.fit();
          invoke("terminal_resize", {
            id: terminalId,
            cols: terminal.cols,
            rows: terminal.rows,
          }).catch(() => {});
        } catch {}
      });
    });
    resizeObserver.observe(container);

    // Initial focus
    terminal.focus();
  });

  onDestroy(() => {
    unlistenOutput?.();
    unlistenExit?.();
    resizeObserver?.disconnect();
    // NOTE: We deliberately do NOT dispose the xterm instance here —
    // the instance stays in INSTANCE_CACHE and will be reattached if the tab reopens.
    // The PTY itself is closed when the tab is removed from the layout (handled externally).
  });

  /** Called by parent when tab is actually closed (not just switched away). */
  export function destroyTerminal() {
    const inst = INSTANCE_CACHE.get(terminalId);
    if (inst) {
      inst.terminal.dispose();
      INSTANCE_CACHE.delete(terminalId);
    }
    invoke("terminal_close", { id: terminalId }).catch(() => {});
  }
</script>

<div class="terminal-wrapper">
  {#if isLoading}
    <div class="terminal-loading">Starting terminal...</div>
  {/if}

  <div
    bind:this={container}
    class="terminal-container"
    class:hidden={isLoading}
  ></div>

  {#if exited}
    <div class="terminal-exit-badge">
      Terminal exited{exitCode !== null ? ` (code ${exitCode})` : ""}
    </div>
  {/if}
</div>

<style>
  .terminal-wrapper {
    position: relative;
    width: 100%;
    height: 100%;
    background: #000;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .terminal-container {
    flex: 1;
    width: 100%;
    min-height: 0;
    padding: 4px 8px;
    box-sizing: border-box;
  }

  .terminal-container.hidden {
    visibility: hidden;
  }

  .terminal-loading {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #6b7280;
    font-size: 13px;
    font-family: "JetBrains Mono", monospace;
  }

  .terminal-exit-badge {
    position: absolute;
    bottom: 8px;
    right: 12px;
    background: rgba(239, 68, 68, 0.15);
    color: #f87171;
    border: 1px solid rgba(239, 68, 68, 0.3);
    border-radius: 4px;
    padding: 2px 8px;
    font-size: 11px;
    font-family: "JetBrains Mono", monospace;
    pointer-events: none;
  }

  /* xterm.js global overrides */
  :global(.xterm) {
    height: 100%;
    padding: 4px 0;
  }
  :global(.xterm-viewport) {
    overflow-y: auto !important;
  }
  :global(.xterm-screen) {
    padding: 0;
  }
</style>
