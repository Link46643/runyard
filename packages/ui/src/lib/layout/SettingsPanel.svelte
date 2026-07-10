<script lang="ts">
  import { onMount } from "svelte";
  import { settingsStore } from "./settingsStore.svelte.js";
  import type { LspLanguageConfig } from "@runyard/common";

  let activeSection = $state<"editor" | "terminal" | "appearance" | "lsp" | "connections">("editor");
  let connections = $state<any[]>([]);

  function loadConnections() {
    try {
      connections = JSON.parse(localStorage.getItem("runyard:connections") || "[]");
    } catch {
      connections = [];
    }
  }

  onMount(async () => {
    loadConnections();
    if (!settingsStore.loaded) {
      await settingsStore.load();
    }
  });

  // Helper to update a nested setting and auto-save
  async function set<K extends keyof typeof settingsStore.settings>(
    section: K,
    key: string,
    value: unknown
  ) {
    (settingsStore.settings[section] as any)[key] = value;
    await settingsStore.save();
  }

  const sections = [
    { id: "editor", label: "Editor" },
    { id: "terminal", label: "Terminal" },
    { id: "appearance", label: "Appearance" },
    { id: "lsp", label: "Language Servers" },
    { id: "connections", label: "Connections" },
  ] as const;

  const languages = [
    { id: "typescript", label: "TypeScript / JavaScript", server: "typescript-language-server" },
    { id: "python", label: "Python", server: "basedpyright / pyright" },
    { id: "rust", label: "Rust", server: "rust-analyzer" },
    { id: "go", label: "Go", server: "gopls" },
  ] as const;
</script>

<div class="settings-panel">
  <!-- Sidebar navigation -->
  <nav class="settings-nav">
    <div class="nav-label">Settings</div>
    {#each sections as sec}
      <button
        class="nav-item"
        class:active={activeSection === sec.id}
        onclick={() => (activeSection = sec.id)}
      >
        {sec.label}
      </button>
    {/each}
  </nav>

  <!-- Content area -->
  <div class="settings-content">
    {#if activeSection === "editor"}
      <h2 class="section-title">Editor</h2>

      <div class="field">
        <label for="s-font-size">Font Size</label>
        <div class="field-row">
          <input
            id="s-font-size"
            type="range"
            min="10"
            max="24"
            step="1"
            value={settingsStore.settings.editor.font_size}
            oninput={(e) =>
              set("editor", "font_size", Number((e.target as HTMLInputElement).value))}
            class="range-input"
          />
          <span class="value-badge">{settingsStore.settings.editor.font_size}px</span>
        </div>
      </div>

      <div class="field">
        <label for="s-tab-size">Tab Size</label>
        <div class="field-row">
          <select
            id="s-tab-size"
            value={settingsStore.settings.editor.tab_size}
            onchange={(e) =>
              set("editor", "tab_size", Number((e.target as HTMLSelectElement).value))}
            class="select-input"
          >
            <option value={2}>2 spaces</option>
            <option value={4}>4 spaces</option>
            <option value={8}>8 spaces</option>
          </select>
        </div>
      </div>

      <div class="field toggle">
        <div>
          <label>Line Wrap</label>
          <p class="field-desc">Wrap long lines in the editor</p>
        </div>
        <label class="toggle-switch">
          <input
            type="checkbox"
            checked={settingsStore.settings.editor.line_wrap}
            onchange={(e) =>
              set("editor", "line_wrap", (e.target as HTMLInputElement).checked)}
          />
          <span class="slider"></span>
        </label>
      </div>

      <div class="field toggle">
        <div>
          <label>Format on Save</label>
          <p class="field-desc">Run language server formatting when saving</p>
        </div>
        <label class="toggle-switch">
          <input
            type="checkbox"
            checked={settingsStore.settings.editor.format_on_save}
            onchange={(e) =>
              set("editor", "format_on_save", (e.target as HTMLInputElement).checked)}
          />
          <span class="slider"></span>
        </label>
      </div>

      <div class="field toggle">
        <div>
          <label>Vim Mode</label>
          <p class="field-desc">Enable Vim keybindings (stub — coming soon)</p>
        </div>
        <label class="toggle-switch">
          <input
            type="checkbox"
            checked={settingsStore.settings.editor.vim_mode}
            onchange={(e) =>
              set("editor", "vim_mode", (e.target as HTMLInputElement).checked)}
          />
          <span class="slider"></span>
        </label>
      </div>

      <div class="field toggle">
        <div>
          <label>Code Folding</label>
          <p class="field-desc">Show fold triangles in the gutter and enable fold shortcuts</p>
        </div>
        <label class="toggle-switch">
          <input
            type="checkbox"
            checked={settingsStore.settings.editor.show_fold_gutter}
            onchange={(e) =>
              set("editor", "show_fold_gutter", (e.target as HTMLInputElement).checked)}
          />
          <span class="slider"></span>
        </label>
      </div>

      <div class="field toggle">
        <div>
          <label>Minimap</label>
          <p class="field-desc">Show a scaled document overview on the right side of the editor</p>
        </div>
        <label class="toggle-switch">
          <input
            type="checkbox"
            checked={settingsStore.settings.editor.show_minimap}
            onchange={(e) =>
              set("editor", "show_minimap", (e.target as HTMLInputElement).checked)}
          />
          <span class="slider"></span>
        </label>
      </div>

    {:else if activeSection === "terminal"}
      <h2 class="section-title">Terminal</h2>

      <div class="field">
        <label for="t-font-size">Font Size</label>
        <div class="field-row">
          <input
            id="t-font-size"
            type="range"
            min="10"
            max="22"
            step="1"
            value={settingsStore.settings.terminal.font_size}
            oninput={(e) =>
              set("terminal", "font_size", Number((e.target as HTMLInputElement).value))}
            class="range-input"
          />
          <span class="value-badge">{settingsStore.settings.terminal.font_size}px</span>
        </div>
      </div>

      <div class="field">
        <label for="t-shell">Default Shell</label>
        <div class="field-row">
          <input
            id="t-shell"
            type="text"
            placeholder="Default (auto-detect from $SHELL)"
            value={settingsStore.settings.terminal.default_shell ?? ""}
            oninput={(e) => {
              const v = (e.target as HTMLInputElement).value;
              set("terminal", "default_shell", v || null);
            }}
            class="text-input"
          />
        </div>
        <p class="field-desc">Full path to shell executable, e.g. /bin/zsh</p>
      </div>

      <div class="field">
        <label for="t-scrollback">Scrollback Buffer</label>
        <div class="field-row">
          <select
            id="t-scrollback"
            value={settingsStore.settings.terminal.scrollback_limit}
            onchange={(e) =>
              set(
                "terminal",
                "scrollback_limit",
                Number((e.target as HTMLSelectElement).value)
              )}
            class="select-input"
          >
            <option value={500}>500 lines</option>
            <option value={1000}>1,000 lines</option>
            <option value={5000}>5,000 lines</option>
            <option value={10000}>10,000 lines</option>
          </select>
        </div>
      </div>

    {:else if activeSection === "appearance"}
      <h2 class="section-title">Appearance</h2>

      <div class="field">
        <label>Theme</label>
        <div class="theme-toggle">
          {#each ["dark", "light"] as t}
            <button
              class="theme-btn"
              class:active={settingsStore.settings.appearance.theme === t}
              onclick={() => set("appearance", "theme", t)}
            >
              {t === "dark" ? "Dark" : "Light"}
            </button>
          {/each}
        </div>
      </div>

      <div class="field">
        <label for="a-font">Font Family</label>
        <div class="field-row">
          <select
            id="a-font"
            value={settingsStore.settings.appearance.font_family}
            onchange={(e) =>
              set(
                "appearance",
                "font_family",
                (e.target as HTMLSelectElement).value
              )}
            class="select-input"
          >
            <option value="JetBrains Mono">JetBrains Mono</option>
            <option value="Fira Code">Fira Code</option>
            <option value="Cascadia Code">Cascadia Code</option>
            <option value="Inconsolata">Inconsolata</option>
            <option value="monospace">System Monospace</option>
          </select>
        </div>
      </div>

    {:else if activeSection === "lsp"}
      <h2 class="section-title">Language Servers</h2>
      <p class="section-desc">
        Runyard auto-detects language servers from your PATH. Configure per-language
        settings below.
      </p>

      {#each languages as lang}
        {@const config = settingsStore.settings.lsp[lang.id as keyof typeof settingsStore.settings.lsp] as LspLanguageConfig}
        <div class="lsp-entry">
          <div class="lsp-header">
            <div class="lsp-name">{lang.label}</div>
            <label class="toggle-switch small">
              <input
                type="checkbox"
                checked={config.enabled}
                onchange={(e) => {
                  (settingsStore.settings.lsp[lang.id as keyof typeof settingsStore.settings.lsp] as LspLanguageConfig).enabled = (e.target as HTMLInputElement).checked;
                  settingsStore.save();
                }}
              />
              <span class="slider"></span>
            </label>
          </div>
          <p class="lsp-server-hint">Server: {lang.server}</p>
          {#if config.enabled}
            <div class="field">
              <label>Custom path (optional)</label>
              <input
                type="text"
                placeholder="Auto-detect from PATH"
                value={config.path_override ?? ""}
                oninput={(e) => {
                  const v = (e.target as HTMLInputElement).value;
                  (settingsStore.settings.lsp[lang.id as keyof typeof settingsStore.settings.lsp] as LspLanguageConfig).path_override = v || null;
                  settingsStore.save();
                }}
                class="text-input"
              />
            </div>
          {/if}
        </div>
      {/each}
    {:else if activeSection === "connections"}
      <h2 class="section-title">Remote Connections</h2>
      <p class="section-desc">
        Configure host connections to remote Sub-services.
      </p>

      <div class="connections-list" style="margin-bottom: 24px; display: flex; flex-direction: column; gap: 12px;">
        {#if connections.length === 0}
          <div style="opacity: 0.5; font-style: italic;">No remote connections configured.</div>
        {:else}
          {#each connections as conn, i}
            <div style="border: 1px solid var(--border); padding: 12px; display: flex; justify-content: space-between; align-items: center; border-radius: 4px; background: var(--bg-secondary);">
              <div>
                <div style="font-weight: bold;">{conn.name}</div>
                <div style="font-size: 11px; opacity: 0.8; font-family: var(--font-mono);">ws://{conn.host}:{conn.port}</div>
              </div>
              <div style="display: flex; gap: 8px;">
                <button
                  style="padding: 4px 8px; font-size: 11px; cursor: pointer; background: var(--accent); color: white; border: none; border-radius: 3px;"
                  onclick={() => {
                    localStorage.setItem("runyard:token", conn.token);
                    window.location.search = `?token=${encodeURIComponent(conn.token)}`;
                  }}
                >
                  Connect
                </button>
                <button
                  style="padding: 4px 8px; font-size: 11px; cursor: pointer; background: transparent; border: 1px solid var(--border); border-radius: 3px; color: var(--text-secondary);"
                  onclick={() => {
                    const next = connections.filter((_: any, idx: number) => idx !== i);
                    localStorage.setItem("runyard:connections", JSON.stringify(next));
                    loadConnections();
                  }}
                >
                  Delete
                </button>
              </div>
            </div>
          {/each}
        {/if}
      </div>

      <div style="border-top: 1px solid var(--border); padding-top: 16px;">
        <h3 style="font-size: 14px; font-weight: 600; margin-bottom: 12px;">Add New Connection</h3>
        
        <div style="display: flex; flex-direction: column; gap: 8px;">
          <input type="text" placeholder="Connection Name (e.g. My Remote Server)" id="conn-name" class="text-input" />
          <div style="display: flex; gap: 8px;">
            <input type="text" placeholder="Host (e.g. localhost)" id="conn-host" class="text-input" style="flex: 2;" />
            <input type="number" placeholder="Port" id="conn-port" class="text-input" style="flex: 1;" value="7820" />
          </div>
          <input type="password" placeholder="Auth Token" id="conn-token" class="text-input" />
          
          <button
            style="align-self: flex-start; padding: 6px 12px; background: var(--accent); color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 600;"
            onclick={() => {
              const nameEl = document.getElementById("conn-name") as HTMLInputElement;
              const hostEl = document.getElementById("conn-host") as HTMLInputElement;
              const portEl = document.getElementById("conn-port") as HTMLInputElement;
              const tokenEl = document.getElementById("conn-token") as HTMLInputElement;
              
              if (!nameEl.value || !hostEl.value || !tokenEl.value) {
                 alert("Please fill in Name, Host, and Token");
                 return;
              }

              const newConn = {
                name: nameEl.value,
                host: hostEl.value,
                port: Number(portEl.value) || 7820,
                token: tokenEl.value
              };

              const current = (() => {
                try {
                  return JSON.parse(localStorage.getItem("runyard:connections") || "[]");
                } catch {
                  return [];
                }
              })();

              current.push(newConn);
              localStorage.setItem("runyard:connections", JSON.stringify(current));
              
              nameEl.value = "";
              hostEl.value = "";
              portEl.value = "7820";
              tokenEl.value = "";
              
              loadConnections();
            }}
          >
            Save Connection
          </button>
        </div>
      </div>

      <div style="border-top: 1px solid var(--border); padding-top: 16px; margin-top: 16px;">
        <h3 style="font-size: 14px; font-weight: 600; margin-bottom: 12px;">SSH Bootstrap Remote Server</h3>
        <p style="font-size: 11px; opacity: 0.8; margin-bottom: 8px;">Deploys the Subservice backend to a remote Linux environment automatically via SSH.</p>
        <div style="display: flex; flex-direction: column; gap: 8px;">
          <div style="display: flex; gap: 8px;">
            <input type="text" placeholder="SSH Host" id="ssh-host" class="text-input" style="flex: 2;" />
            <input type="number" placeholder="SSH Port" id="ssh-port" class="text-input" style="flex: 1;" value="22" />
          </div>
          <input type="text" placeholder="SSH Username" id="ssh-user" class="text-input" />
          <input type="text" placeholder="Subservice Port (default 7820)" id="bootstrap-port" class="text-input" value="7820" />
          <input type="password" placeholder="Subservice Token (auto-generated if empty)" id="bootstrap-token" class="text-input" />

          <button
            style="align-self: flex-start; padding: 6px 12px; background: var(--accent-secondary, #8b5cf6); color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 600;"
            onclick={async () => {
              const hostEl = document.getElementById("ssh-host") as HTMLInputElement;
              const portEl = document.getElementById("ssh-port") as HTMLInputElement;
              const userEl = document.getElementById("ssh-user") as HTMLInputElement;
              const bootPortEl = document.getElementById("bootstrap-port") as HTMLInputElement;
              const bootTokenEl = document.getElementById("bootstrap-token") as HTMLInputElement;

              if (!hostEl.value || !userEl.value) {
                alert("Please fill in SSH Host and Username");
                return;
              }

              const token = bootTokenEl.value || "ry_tok_generated";

              try {
                // Call rust tauri command for boot
                const res = await invoke<string>("ssh_bootstrap", {
                  host: hostEl.value,
                  port: Number(portEl.value) || 22,
                  username: userEl.value,
                  token
                });
                alert(res);
              } catch(e) {
                alert(`SSH Bootstrap failed: ${e}`);
              }
            }}
          >
            Run SSH Bootstrap
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings-panel {
    display: flex;
    height: 100%;
    background: var(--bg);
    color: var(--text);
    font-family: "Google Sans Flex Variable", "JetBrains Mono", sans-serif;
    font-size: 13px;
    overflow: hidden;
  }

  /* ── Sidebar nav ── */
  .settings-nav {
    width: 160px;
    flex-shrink: 0;
    background: var(--sidebar-bg, var(--bg-secondary));
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    padding: 12px 0;
    gap: 1px;
    overflow-y: auto;
  }

  .nav-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
    padding: 0 14px 8px;
    font-weight: 600;
  }

  .nav-item {
    background: none;
    border: none;
    text-align: left;
    padding: 7px 14px;
    font-size: 13px;
    font-family: inherit;
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: 0;
    transition: background 0.1s, color 0.1s;
  }

  .nav-item:hover {
    background: rgba(255, 255, 255, 0.05);
    color: var(--text);
  }

  .nav-item.active {
    background: rgba(59, 130, 246, 0.1);
    color: var(--accent);
    font-weight: 600;
  }

  /* ── Content ── */
  .settings-content {
    flex: 1;
    overflow-y: auto;
    padding: 24px 28px;
    max-width: 600px;
  }

  .section-title {
    font-size: 16px;
    font-weight: 700;
    margin: 0 0 20px;
    color: var(--text);
  }

  .section-desc {
    margin: -12px 0 20px;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.5;
  }

  /* ── Field ── */
  .field {
    margin-bottom: 20px;
  }

  .field label {
    display: block;
    font-weight: 600;
    margin-bottom: 6px;
    color: var(--text);
    font-size: 13px;
  }

  .field.toggle {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
    padding: 12px 0;
    border-bottom: 1px solid var(--border);
    margin-bottom: 0;
  }

  .field.toggle label {
    margin-bottom: 2px;
  }

  .field-desc {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 2px 0 0;
  }

  .field-row {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .value-badge {
    font-size: 12px;
    font-family: "JetBrains Mono", monospace;
    color: var(--accent);
    min-width: 36px;
    text-align: right;
  }

  /* Inputs */
  .range-input {
    flex: 1;
    accent-color: var(--accent);
  }

  .select-input,
  .text-input {
    flex: 1;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 5px;
    color: var(--text);
    font-family: inherit;
    font-size: 13px;
    padding: 6px 10px;
  }

  .select-input:focus,
  .text-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  /* Theme toggle */
  .theme-toggle {
    display: flex;
    gap: 8px;
  }

  .theme-btn {
    padding: 6px 18px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: none;
    color: var(--text-secondary);
    font-size: 13px;
    font-family: inherit;
    cursor: pointer;
    transition: all 0.15s;
  }

  .theme-btn.active {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }

  .theme-btn:hover:not(.active) {
    background: rgba(255, 255, 255, 0.06);
    color: var(--text);
  }

  /* Toggle switch */
  .toggle-switch {
    position: relative;
    display: inline-block;
    width: 40px;
    height: 22px;
    flex-shrink: 0;
  }

  .toggle-switch.small {
    width: 34px;
    height: 18px;
  }

  .toggle-switch input {
    opacity: 0;
    width: 0;
    height: 0;
    position: absolute;
  }

  .slider {
    position: absolute;
    inset: 0;
    background: var(--border);
    border-radius: 999px;
    cursor: pointer;
    transition: background 0.2s;
  }

  .slider::before {
    content: "";
    position: absolute;
    width: 16px;
    height: 16px;
    left: 3px;
    bottom: 3px;
    background: white;
    border-radius: 50%;
    transition: transform 0.2s;
  }

  .toggle-switch.small .slider::before {
    width: 12px;
    height: 12px;
    left: 3px;
    bottom: 3px;
  }

  input:checked + .slider {
    background: var(--accent);
  }

  input:checked + .slider::before {
    transform: translateX(18px);
  }

  .toggle-switch.small input:checked + .slider::before {
    transform: translateX(16px);
  }

  /* LSP entries */
  .lsp-entry {
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 14px 16px;
    margin-bottom: 12px;
  }

  .lsp-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 4px;
  }

  .lsp-name {
    font-weight: 600;
    font-size: 13px;
  }

  .lsp-server-hint {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 0 0 8px;
  }

  .lsp-entry .field {
    margin-top: 10px;
    margin-bottom: 0;
  }
</style>
