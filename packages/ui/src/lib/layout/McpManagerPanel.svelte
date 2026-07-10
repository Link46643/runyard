<script lang="ts">
  // 1. Imports
  import { onMount, onDestroy } from "svelte";
  import {
    Plus,
    Upload,
    Download,
    Server,
    Pencil,
    Trash2,
  } from "lucide-svelte";
  import { invoke as tauriInvoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { webSocketClient } from "@runyard/common";
  import type { McpServerConfig, McpEnvVar, McpAuth } from "@runyard/common";

  // 2. Types
  type DialogMode = "add" | "edit" | null;
  type TransportKind = "stdio" | "http" | "websocket";
  type AuthKind = "none" | "bearer" | "api_key";
  type DialogTab = "manual" | "registry";

  interface EnvVarRow {
    key: string;
    value: string;
    is_secret: boolean;
  }

  interface RegistryServer {
    id: string;
    name: string;
    description: string;
    transport: TransportKind;
    command: string;
    envVars?: string[];
  }

  // ── Registry curated list ──────────────────────────────────────────────────
  const POPULAR_MCP_SERVERS: RegistryServer[] = [
    { id: "filesystem", name: "Filesystem", description: "Local filesystem read/write access. Configurable allowed directories.", transport: "stdio", command: "npx -y @modelcontextprotocol/server-filesystem {DIRECTORY}" },
    { id: "github", name: "GitHub", description: "GitHub API: repos, issues, PRs, search, file contents.", transport: "stdio", command: "npx -y @modelcontextprotocol/server-github", envVars: ["GITHUB_PERSONAL_ACCESS_TOKEN"] },
    { id: "brave-search", name: "Brave Search", description: "Web and local search via Brave Search API.", transport: "stdio", command: "npx -y @modelcontextprotocol/server-brave-search", envVars: ["BRAVE_API_KEY"] },
    { id: "slack", name: "Slack", description: "Slack workspace integration: channels, messages, users.", transport: "stdio", command: "npx -y @modelcontextprotocol/server-slack", envVars: ["SLACK_BOT_TOKEN", "SLACK_TEAM_ID"] },
    { id: "postgres", name: "PostgreSQL", description: "Read-only database access and schema inspection.", transport: "stdio", command: "npx -y @modelcontextprotocol/server-postgres", envVars: ["DATABASE_URL"] },
    { id: "google-maps", name: "Google Maps", description: "Geocoding, directions, place search.", transport: "stdio", command: "npx -y @modelcontextprotocol/server-google-maps", envVars: ["GOOGLE_MAPS_API_KEY"] },
    { id: "fetch", name: "Fetch (HTTP)", description: "Fetch any URL, convert HTML to markdown.", transport: "stdio", command: "npx -y @modelcontextprotocol/server-fetch" },
    { id: "puppeteer", name: "Puppeteer", description: "Browser automation, screenshots, web scraping.", transport: "stdio", command: "npx -y @modelcontextprotocol/server-puppeteer" },
    { id: "sqlite", name: "SQLite", description: "SQLite database access for local development.", transport: "stdio", command: "npx -y @modelcontextprotocol/server-sqlite --db-path {DB_PATH}" },
    { id: "linear", name: "Linear", description: "Linear issue tracker: teams, issues, projects, cycles.", transport: "stdio", command: "npx -y @modelcontextprotocol/server-linear", envVars: ["LINEAR_API_KEY"] },
  ];

  // Dual-mode invoke (Tauri or WebSocket)
  async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    if (webSocketClient.status === "connected") {
      return webSocketClient.invoke<T>(cmd, args);
    } else {
      return tauriInvoke<T>(cmd, args);
    }
  }

  // 3. Props (none — top-level panel)

  // 4. State
  let servers = $state<McpServerConfig[]>([]);
  // 1.8.6: health status per server name, updated from ACP session_info_update events.
  // Agents report MCP server health in session/update session_info_update payloads.
  let serverHealth = $state<Record<string, "active" | "error" | "unknown">>({});
  let isLoading = $state(false);
  let error = $state<string | null>(null);

  let dialogMode = $state<DialogMode>(null);
  let editingServer = $state<McpServerConfig | null>(null);

  // Form fields
  let formName = $state("");
  let formTransport = $state<TransportKind>("stdio");
  let formCommand = $state("");
  let formUrl = $state("");
  let formAuthKind = $state<AuthKind>("none");
  let formAuthToken = $state("");
  let formEnvVars = $state<EnvVarRow[]>([]);
  let formIsGlobal = $state(true);

  // Validation errors
  let formErrors = $state<{ command?: string; url?: string; name?: string }>({});

  // Delete confirmation: stores id with confirmation open
  let deleteConfirmId = $state<string | null>(null);

  // Import file input ref
  let importFileInput = $state<HTMLInputElement | null>(null);

  // Dialog tab state (manual / registry) — only for "add" mode
  let dialogTab = $state<DialogTab>("manual");

  // Validate format feedback
  let validateMsg = $state<{ ok: boolean; text: string } | null>(null);

  // 5. Derived
  let canSubmit = $derived(
    formName.trim().length > 0 &&
    Object.keys(formErrors).length === 0
  );

  // 6. Effects (none needed)

  // 7. Functions

  // ── Status dot color ──────────────────────────────────────────────────────
  function statusDotColor(server: McpServerConfig): string {
    if (!server.is_active) return "var(--text-tertiary)";
    if (server.is_global) return "var(--text-success)";
    return "var(--accent-warning)";
  }

  // ── Validation ────────────────────────────────────────────────────────────
  function validateForm(): boolean {
    const errs: typeof formErrors = {};
    if (!formName.trim()) {
      errs.name = "Name is required.";
    }
    if (formTransport === "stdio") {
      if (!formCommand.trim()) {
        errs.command = "Command is required for stdio transport.";
      }
    } else {
      const trimmed = formUrl.trim();
      if (!trimmed) {
        errs.url = "URL is required.";
      } else if (
        formTransport === "http" && !trimmed.startsWith("http://") && !trimmed.startsWith("https://")
      ) {
        errs.url = "URL must start with http:// or https://";
      } else if (
        formTransport === "websocket" && !trimmed.startsWith("ws://") && !trimmed.startsWith("wss://")
      ) {
        errs.url = "URL must start with ws:// or wss://";
      }
    }
    formErrors = errs;
    return Object.keys(errs).length === 0;
  }

  // ── Load ──────────────────────────────────────────────────────────────────
  async function loadServers() {
    isLoading = true;
    error = null;
    try {
      servers = await invoke<McpServerConfig[]>("mcp_server_list");
    } catch (e) {
      error = String(e);
    } finally {
      isLoading = false;
    }
  }

  // ── Toggle active ─────────────────────────────────────────────────────────
  async function toggleActive(server: McpServerConfig) {
    error = null;
    try {
      await invoke("mcp_server_set_active", { id: server.id, isActive: !server.is_active });
      servers = servers.map((s) =>
        s.id === server.id ? { ...s, is_active: !s.is_active } : s
      );
    } catch (e) {
      error = String(e);
    }
  }

  // ── Delete ────────────────────────────────────────────────────────────────
  function openDeleteConfirm(id: string) {
    deleteConfirmId = id;
  }

  function closeDeleteConfirm() {
    deleteConfirmId = null;
  }

  async function confirmDelete(server: McpServerConfig) {
    error = null;
    try {
      await invoke("mcp_server_delete", { id: server.id });
      servers = servers.filter((s) => s.id !== server.id);
    } catch (e) {
      error = String(e);
    } finally {
      deleteConfirmId = null;
    }
  }

  // ── Dialog open / close ───────────────────────────────────────────────────
  function openAddDialog() {
    dialogMode = "add";
    editingServer = null;
    dialogTab = "manual";
    resetForm();
  }

  function openEditDialog(server: McpServerConfig) {
    dialogMode = "edit";
    editingServer = server;
    formName = server.name;
    formTransport = server.transport;
    formCommand = server.command ?? "";
    formUrl = server.url ?? "";
    formAuthKind = server.auth?.kind ?? "none";
    formAuthToken = server.auth?.token ?? "";
    formEnvVars = server.env_vars.map((e) => ({ ...e }));
    formIsGlobal = server.is_global;
    formErrors = {};
  }

  function closeDialog() {
    dialogMode = null;
    editingServer = null;
    resetForm();
  }

  function resetForm() {
    formName = "";
    formTransport = "stdio";
    formCommand = "";
    formUrl = "";
    formAuthKind = "none";
    formAuthToken = "";
    formEnvVars = [];
    formIsGlobal = true;
    formErrors = {};
    validateMsg = null;
  }

  // ── Registry: use a server ────────────────────────────────────────────────
  function useRegistryServer(server: RegistryServer) {
    formName = server.name;
    formTransport = server.transport;
    formCommand = server.command;
    formUrl = "";
    formAuthKind = "none";
    formAuthToken = "";
    formEnvVars = server.envVars
      ? server.envVars.map((key) => ({ key, value: "", is_secret: true }))
      : [];
    formIsGlobal = true;
    formErrors = {};
    validateMsg = null;
    dialogTab = "manual";
  }

  // ── Validate format ───────────────────────────────────────────────────────
  function validateFormat() {
    if (formTransport === "stdio") {
      if (!formCommand.trim()) {
        validateMsg = { ok: false, text: "Command is required for stdio transport." };
      } else if (!/^[a-zA-Z]/.test(formCommand.trim())) {
        validateMsg = { ok: false, text: "Command must start with a valid executable name." };
      } else {
        validateMsg = { ok: true, text: "Command format looks valid." };
      }
    } else {
      const trimmed = formUrl.trim();
      if (!trimmed) {
        validateMsg = { ok: false, text: "URL is required." };
      } else if (formTransport === "http" && !trimmed.startsWith("http://") && !trimmed.startsWith("https://")) {
        validateMsg = { ok: false, text: "URL must start with http:// or https://" };
      } else if (formTransport === "websocket" && !trimmed.startsWith("ws://") && !trimmed.startsWith("wss://")) {
        validateMsg = { ok: false, text: "URL must start with ws:// or wss://" };
      } else {
        validateMsg = { ok: true, text: "URL format looks valid." };
      }
    }
  }

  // ── Env vars ──────────────────────────────────────────────────────────────
  function addEnvVar() {
    formEnvVars = [...formEnvVars, { key: "", value: "", is_secret: false }];
  }

  function removeEnvVar(index: number) {
    formEnvVars = formEnvVars.filter((_, i) => i !== index);
  }

  function updateEnvVar(index: number, field: keyof EnvVarRow, val: string | boolean) {
    formEnvVars = formEnvVars.map((row, i) =>
      i === index ? { ...row, [field]: val } : row
    );
  }

  // ── Submit ────────────────────────────────────────────────────────────────
  async function submitForm() {
    if (!validateForm()) return;
    error = null;

    const auth: McpAuth | null =
      formAuthKind === "none"
        ? { kind: "none" }
        : { kind: formAuthKind, token: formAuthToken.trim() || null };

    const envVars: McpEnvVar[] = formEnvVars
      .filter((e) => e.key.trim())
      .map((e) => ({ key: e.key, value: e.value, is_secret: e.is_secret }));

    try {
      if (dialogMode === "edit" && editingServer) {
        const updated = await invoke<McpServerConfig>("mcp_server_update", {
          id: editingServer.id,
          name: formName.trim(),
          transport: formTransport,
          command: formTransport === "stdio" ? (formCommand.trim() || null) : null,
          url: formTransport !== "stdio" ? (formUrl.trim() || null) : null,
          envVars,
          auth,
          isGlobal: formIsGlobal,
          projectId: null,
        });
        servers = servers.map((s) => (s.id === editingServer!.id ? updated : s));
      } else {
        const created = await invoke<McpServerConfig>("mcp_server_create", {
          name: formName.trim(),
          transport: formTransport,
          command: formTransport === "stdio" ? (formCommand.trim() || null) : null,
          url: formTransport !== "stdio" ? (formUrl.trim() || null) : null,
          envVars,
          auth,
          isGlobal: formIsGlobal,
          projectId: null,
        });
        servers = [...servers, created];
      }
      closeDialog();
    } catch (e) {
      error = String(e);
    }
  }

  // ── Export ────────────────────────────────────────────────────────────────
  async function exportServer(server: McpServerConfig) {
    error = null;
    try {
      const json = await invoke<string>("mcp_server_export", { id: server.id });
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${server.name.replace(/\s+/g, "-")}.runyard-mcp.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      error = String(e);
    }
  }

  // ── Import ────────────────────────────────────────────────────────────────
  function triggerImport() {
    importFileInput?.click();
  }

  async function handleImportFile(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    const text = await file.text();
    input.value = "";
    error = null;
    try {
      const imported = await invoke<McpServerConfig>("mcp_server_import", { json: text });
      servers = [...servers, imported];
    } catch (e) {
      error = String(e);
    }
  }

  // 8. Lifecycle
  let _unlistenAcp: (() => void) | null = null;
  onMount(() => {
    loadServers();
    // 1.8.6: Listen for ACP session_info_update events dispatched as CustomEvent
    // by acpStore's event handler fallthrough (window.dispatchEvent).
    const handler = (e: Event) => {
      const payload = (e as CustomEvent).detail;
      if (payload?.mcp_servers) {
        // Agent reported MCP server health. Payload shape: { mcp_servers: [{name, status}] }
        for (const srv of payload.mcp_servers) {
          serverHealth = {
            ...serverHealth,
            [srv.name]: srv.status === "connected" ? "active" : srv.status === "error" ? "error" : "unknown",
          };
        }
      }
    };
    window.addEventListener("acp:session_info_update", handler);
    _unlistenAcp = () => window.removeEventListener("acp:session_info_update", handler);
  });
  onDestroy(() => {
    _unlistenAcp?.();
  });
</script>

<!-- Hidden import file input -->
<input
  type="file"
  accept=".json"
  style="display:none"
  bind:this={importFileInput}
  onchange={handleImportFile}
/>

<div class="mcp-manager-panel">
  <!-- Panel header -->
  <div class="panel-header">
    <span class="panel-title">MCP SERVERS</span>
    <div class="header-actions">
      <button class="btn-ghost btn-sm" onclick={triggerImport} title="Import server">
        <Upload size={12} />
        Import
      </button>
      <button class="btn-primary btn-sm" onclick={openAddDialog}>
        <Plus size={12} />
        Add server
      </button>
    </div>
  </div>

  <!-- Global error banner -->
  {#if error}
    <div class="global-error">{error}</div>
  {/if}

  <!-- Server list -->
  <div class="server-list">
    {#if isLoading}
      <div class="empty-state">
        <span class="loading-text">Loading servers...</span>
      </div>
    {:else if servers.length === 0}
      <div class="empty-state">
        <Server size={48} color="var(--text-tertiary)" />
        <p>No MCP servers configured.</p>
        <button class="btn-primary btn-sm" onclick={openAddDialog}>
          <Plus size={12} />
          Add server
        </button>
      </div>
    {:else}
      {#each servers as server (server.id)}
        {@const dotColor = statusDotColor(server)}

        <div class="server-row-wrapper">
          <!-- Main server row -->
          <div class="server-row">
            <div class="server-main">
              <span class="status-dot" style="background:{dotColor}"></span>
              <div class="server-info">
                <div class="server-name-line">
                  <span class="server-name">{server.name}</span>
                  <span class="badge transport-badge">
                    {server.transport === "websocket" ? "ws" : server.transport}
                  </span>
                  <span class="badge scope-badge">
                    {server.is_global ? "global" : "project"}
                  </span>
                </div>
                {#if server.command}
                  <span class="server-detail">{server.command}</span>
                {:else if server.url}
                  <span class="server-detail">{server.url}</span>
                {/if}
              </div>
            </div>

            <div class="server-actions">
              <button
                class="btn-ghost btn-sm"
                onclick={() => toggleActive(server)}
                title={server.is_active ? "Disable server" : "Enable server"}
              >
                {server.is_active ? "Disable" : "Enable"}
              </button>
              <button
                class="btn-icon"
                title="Edit server"
                onclick={() => openEditDialog(server)}
              >
                <Pencil size={12} />
              </button>
              <button
                class="btn-icon"
                title="Export server"
                onclick={() => exportServer(server)}
              >
                <Download size={12} />
              </button>
              <button
                class="btn-icon btn-icon-danger"
                title="Delete server"
                onclick={() => openDeleteConfirm(server.id)}
              >
                <Trash2 size={12} />
              </button>
            </div>
          </div>

          <!-- Inline delete confirmation -->
          {#if deleteConfirmId === server.id}
            <div class="delete-confirm">
              <span class="delete-msg">
                Remove <strong>{server.name}</strong>? This cannot be undone.
              </span>
              <div class="delete-actions">
                <button class="btn-danger btn-sm" onclick={() => confirmDelete(server)}>
                  Remove
                </button>
                <button class="btn-ghost btn-sm" onclick={closeDeleteConfirm}>
                  Cancel
                </button>
              </div>
            </div>
          {/if}
        </div>
      {/each}
    {/if}
  </div>
</div>

<!-- Add / Edit dialog -->
{#if dialogMode !== null}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="modal-backdrop" onclick={(e) => { if (e.target === e.currentTarget) closeDialog(); }}>
    <div class="modal" role="dialog" aria-modal="true">
      <div class="modal-header">
        <h2 class="modal-title">
          {dialogMode === "edit" ? "Edit MCP server" : "Add MCP server"}
        </h2>
        <button class="btn-icon" onclick={closeDialog} aria-label="Close">✕</button>
      </div>

      <!-- Tab bar — only on add mode -->
      {#if dialogMode === "add"}
        <div class="dialog-tabs">
          <button
            type="button"
            class="dialog-tab"
            class:dialog-tab--active={dialogTab === "manual"}
            onclick={() => { dialogTab = "manual"; }}
          >
            Manual
          </button>
          <button
            type="button"
            class="dialog-tab"
            class:dialog-tab--active={dialogTab === "registry"}
            onclick={() => { dialogTab = "registry"; }}
          >
            Registry
          </button>
        </div>
      {/if}

      <!-- Registry tab content -->
      {#if dialogMode === "add" && dialogTab === "registry"}
        <div class="registry-list">
          {#each POPULAR_MCP_SERVERS as srv (srv.id)}
            <div class="registry-card">
              <div class="registry-card-info">
                <span class="registry-card-name">{srv.name}</span>
                <span class="registry-card-desc">{srv.description}</span>
                {#if srv.envVars && srv.envVars.length > 0}
                  <div class="registry-card-envs">
                    {#each srv.envVars as ev (ev)}
                      <span class="badge env-badge">{ev}</span>
                    {/each}
                  </div>
                {/if}
              </div>
              <button
                type="button"
                class="btn-primary btn-sm"
                onclick={() => useRegistryServer(srv)}
              >
                Use
              </button>
            </div>
          {/each}
        </div>
      {:else}

      <form class="server-form" onsubmit={(e) => { e.preventDefault(); submitForm(); }}>
        <!-- Name -->
        <div class="form-row">
          <label class="form-label" for="mcp-name">Name</label>
          <input
            id="mcp-name"
            class="form-input"
            class:input-error={!!formErrors.name}
            type="text"
            placeholder="My MCP Server"
            bind:value={formName}
            required
          />
          {#if formErrors.name}
            <span class="field-error">{formErrors.name}</span>
          {/if}
        </div>

        <!-- Transport -->
        <div class="form-row">
          <label class="form-label" for="mcp-transport">Transport</label>
          <select
            id="mcp-transport"
            class="form-select"
            bind:value={formTransport}
            onchange={() => { formErrors = {}; }}
          >
            <option value="stdio">stdio</option>
            <option value="http">http</option>
            <option value="websocket">websocket</option>
          </select>
        </div>

        <!-- stdio: command -->
        {#if formTransport === "stdio"}
          <div class="form-row">
            <label class="form-label" for="mcp-command">Command</label>
            <input
              id="mcp-command"
              class="form-input"
              class:input-error={!!formErrors.command}
              type="text"
              placeholder="npx my-mcp-server --port 3000"
              bind:value={formCommand}
            />
            {#if formErrors.command}
              <span class="field-error">{formErrors.command}</span>
            {/if}
          </div>
        {:else}
          <!-- http/websocket: URL -->
          <div class="form-row">
            <label class="form-label" for="mcp-url">
              {formTransport === "http" ? "HTTP URL" : "WebSocket URL"}
            </label>
            <input
              id="mcp-url"
              class="form-input"
              class:input-error={!!formErrors.url}
              type="text"
              placeholder={formTransport === "http" ? "http://localhost:8080" : "ws://localhost:8080"}
              bind:value={formUrl}
            />
            {#if formErrors.url}
              <span class="field-error">{formErrors.url}</span>
            {/if}
          </div>
        {/if}

        <!-- Auth -->
        <div class="form-section">
          <div class="form-section-label">Authentication</div>
          <div class="form-row">
            <label class="form-label" for="mcp-auth-kind">Auth type</label>
            <select id="mcp-auth-kind" class="form-select" bind:value={formAuthKind}>
              <option value="none">None</option>
              <option value="bearer">Bearer token</option>
              <option value="api_key">API key</option>
            </select>
          </div>
          {#if formAuthKind !== "none"}
            <div class="form-row">
              <label class="form-label" for="mcp-auth-token">
                {formAuthKind === "bearer" ? "Bearer token" : "API key"}
              </label>
              <input
                id="mcp-auth-token"
                class="form-input"
                type="password"
                placeholder={formAuthKind === "bearer" ? "eyJ..." : "sk-..."}
                bind:value={formAuthToken}
              />
            </div>
          {/if}
        </div>

        <!-- Env vars -->
        <div class="env-section">
          <div class="env-header">
            <span class="form-section-label">Environment variables</span>
            <button type="button" class="btn-ghost btn-sm" onclick={addEnvVar}>
              <Plus size={11} />
              Add
            </button>
          </div>
          {#each formEnvVars as row, i}
            <div class="env-row">
              <input
                class="form-input env-key"
                type="text"
                placeholder="KEY"
                value={row.key}
                oninput={(e) => updateEnvVar(i, "key", (e.target as HTMLInputElement).value)}
              />
              <input
                class="form-input env-val"
                type={row.is_secret ? "password" : "text"}
                placeholder="value"
                value={row.value}
                oninput={(e) => updateEnvVar(i, "value", (e.target as HTMLInputElement).value)}
              />
              <label class="secret-label">
                <input
                  type="checkbox"
                  checked={row.is_secret}
                  onchange={(e) => updateEnvVar(i, "is_secret", (e.target as HTMLInputElement).checked)}
                />
                Secret
              </label>
              <button
                type="button"
                class="btn-icon btn-icon-danger"
                onclick={() => removeEnvVar(i)}
                aria-label="Remove env var"
              >
                <Trash2 size={11} />
              </button>
            </div>
          {/each}
        </div>

        <!-- Scope -->
        <div class="form-section">
          <div class="form-section-label">Scope</div>
          <div class="scope-options">
            <label class="scope-option">
              <input
                type="radio"
                name="mcp-scope"
                value={true}
                checked={formIsGlobal}
                onchange={() => { formIsGlobal = true; }}
              />
              <span class="scope-label">
                <span class="scope-title">Global (all projects)</span>
                <span class="scope-desc">Available in every workspace</span>
              </span>
            </label>
            <label class="scope-option">
              <input
                type="radio"
                name="mcp-scope"
                value={false}
                checked={!formIsGlobal}
                onchange={() => { formIsGlobal = false; }}
              />
              <span class="scope-label">
                <span class="scope-title">This project</span>
                <span class="scope-desc">Only available in the current workspace</span>
              </span>
            </label>
          </div>
        </div>

        <div class="form-footer">
          <button type="button" class="btn-ghost" onclick={closeDialog}>Cancel</button>
          <button type="button" class="btn-ghost" onclick={validateFormat}>
            Validate format
          </button>
          {#if validateMsg !== null}
            <span class="validate-msg" class:validate-ok={validateMsg.ok} class:validate-err={!validateMsg.ok}>
              {validateMsg.text}
            </span>
          {/if}
          <button type="submit" class="btn-primary" disabled={!canSubmit}>
            {dialogMode === "edit" ? "Save changes" : "Add server"}
          </button>
        </div>
      </form>
      {/if}
    </div>
  </div>
{/if}

<style>
  /* Root */
  .mcp-manager-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    font-size: 12px;
    color: var(--text);
    background: var(--bg);
    border: 1px solid var(--border);
  }

  /* Panel header */
  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .panel-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  /* Global error */
  .global-error {
    padding: 6px 10px;
    font-size: 11px;
    color: var(--text-error);
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-error);
    flex-shrink: 0;
  }

  /* Server list */
  .server-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 40px 16px;
    color: var(--text-tertiary);
    font-size: 12px;
    text-align: center;
  }

  .empty-state p {
    margin: 0;
    color: var(--text-secondary);
  }

  .loading-text {
    color: var(--text-secondary);
    font-size: 12px;
    padding: 16px;
  }

  /* Server rows */
  .server-row-wrapper {
    border-bottom: 1px solid var(--border-secondary);
  }

  .server-row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 10px;
  }

  .server-row:hover {
    background: var(--bg-secondary);
  }

  .server-main {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }

  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 9999px;
    flex-shrink: 0;
    margin-top: 4px;
  }

  .server-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .server-name-line {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .server-name {
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
  }

  .server-detail {
    font-size: 11px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--font-mono);
  }

  .server-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  /* Badges */
  .badge {
    background: var(--bg-tertiary);
    border-radius: var(--radius-1);
    font-size: 11px;
    font-weight: 500;
    padding: 2px 6px;
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .transport-badge {
    color: var(--text-tertiary);
    font-family: var(--font-mono);
  }

  .scope-badge {
    color: var(--text-tertiary);
  }

  /* Delete confirmation */
  .delete-confirm {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 10px 8px 24px;
    background: var(--bg-secondary);
    border-top: 1px solid var(--border-secondary);
  }

  .delete-msg {
    font-size: 11px;
    color: var(--text-secondary);
    flex: 1;
  }

  .delete-msg strong {
    color: var(--text);
    font-weight: 500;
  }

  .delete-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  /* Buttons */
  .btn-primary {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: var(--accent);
    color: var(--bg);
    border: none;
    border-radius: var(--radius-1);
    padding: 6px 12px;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .btn-primary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .btn-ghost {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    padding: 6px 12px;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn-ghost:hover {
    background: var(--bg-tertiary);
    color: var(--text);
  }

  .btn-danger {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: var(--accent-danger);
    color: var(--bg);
    border: none;
    border-radius: var(--radius-1);
    padding: 6px 12px;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn-danger:hover {
    filter: brightness(1.1);
  }

  .btn-sm {
    padding: 3px 8px;
    font-size: 11px;
  }

  .btn-icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    border-radius: var(--radius-1);
    color: var(--text-secondary);
    cursor: pointer;
    padding: 4px;
  }

  .btn-icon:hover {
    background: var(--bg-tertiary);
    color: var(--text);
  }

  .btn-icon-danger:hover {
    color: var(--accent-danger);
  }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: var(--bg-overlay);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--bg-elevated);
    border-radius: var(--radius-3);
    box-shadow: var(--shadow-2);
    padding: 24px;
    width: 100%;
    max-width: 560px;
    max-height: 90vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .modal-title {
    font-size: 16px;
    font-weight: 600;
    margin: 0;
    color: var(--text);
  }

  /* Forms */
  .server-form {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .form-row {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .form-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .form-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px;
    background: var(--bg-secondary);
    border-radius: var(--radius-2);
    border: 1px solid var(--border-secondary);
  }

  .form-section-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .form-input {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    color: var(--text);
    font-family: inherit;
    font-size: 12px;
    padding: 6px 8px;
    width: 100%;
    box-sizing: border-box;
  }

  .form-input:focus {
    outline: none;
    border-color: var(--border-active);
  }

  .form-input.input-error {
    border-color: var(--border-error);
  }

  .form-select {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    color: var(--text);
    font-family: inherit;
    font-size: 12px;
    padding: 6px 8px;
    width: 100%;
    box-sizing: border-box;
    cursor: pointer;
  }

  .form-select:focus {
    outline: none;
    border-color: var(--border-active);
  }

  .field-error {
    font-size: 11px;
    color: var(--text-error);
  }

  /* Env vars */
  .env-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .env-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .env-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .env-key {
    flex: 1;
    min-width: 0;
  }

  .env-val {
    flex: 2;
    min-width: 0;
  }

  .secret-label {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    cursor: pointer;
  }

  /* Scope options */
  .scope-options {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .scope-option {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    cursor: pointer;
    padding: 6px 8px;
    border-radius: var(--radius-2);
    border: 1px solid var(--border);
    background: var(--bg);
  }

  .scope-option:has(input:checked) {
    border-color: var(--border-active);
    background: var(--bg-tertiary);
  }

  .scope-label {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .scope-title {
    font-size: 12px;
    color: var(--text);
    font-weight: 500;
  }

  .scope-desc {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  /* Form footer */
  .form-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 4px;
    border-top: 1px solid var(--border);
    flex-wrap: wrap;
  }

  /* Validate feedback */
  .validate-msg {
    font-size: 11px;
    flex: 1;
    min-width: 0;
  }

  .validate-ok {
    color: var(--text-success);
  }

  .validate-err {
    color: var(--text-error);
  }

  /* Dialog tabs */
  .dialog-tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--border);
  }

  .dialog-tab {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-secondary);
    cursor: pointer;
    font-family: inherit;
    font-size: 12px;
    font-weight: 500;
    padding: 6px 14px;
    margin-bottom: -1px;
  }

  .dialog-tab:hover {
    color: var(--text);
  }

  .dialog-tab--active {
    border-bottom-color: var(--accent);
    color: var(--text);
  }

  /* Registry list */
  .registry-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 55vh;
    overflow-y: auto;
  }

  .registry-card {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: var(--radius-2);
    background: var(--bg-secondary);
  }

  .registry-card-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .registry-card-name {
    font-weight: 500;
    font-size: 12px;
    color: var(--text);
  }

  .registry-card-desc {
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  .registry-card-envs {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 2px;
  }

  .env-badge {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--text-tertiary);
    background: var(--bg-tertiary);
    border: 1px solid var(--border-secondary);
    padding: 1px 5px;
    border-radius: var(--radius-1);
  }
</style>
