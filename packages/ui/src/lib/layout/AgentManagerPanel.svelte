<script lang="ts">
  // 1. Imports
  import { onMount } from "svelte";
  import {
    Plus,
    Upload,
    Download,
    Plug,
    PlugZap,
    Pencil,
    Trash2,
    ChevronDown,
    ChevronUp,
    Filter,
    Cpu,
    Terminal,
    RefreshCw,
  } from "lucide-svelte";
  import { invoke as tauriInvoke } from "@tauri-apps/api/core";
  import { webSocketClient } from "@runyard/common";
  import { acpStore } from "../stores/acpStore.svelte.js";

  async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
    if (webSocketClient.status === "connected") return webSocketClient.invoke<T>(cmd, args);
    return tauriInvoke<T>(cmd, args);
  }
  import type {
    AcpAgentConfig,
    AcpTransportKind,
    AcpAgentEnvVar,
    DiscoveredAcpAgent,
    AcpRegistryAgent,
    AcpLogDirection,
  } from "@runyard/common";

  // 2. Type defs
  type DialogMode = "add" | "edit" | null;
  type AddTab = "manual" | "discover" | "registry";
  type LogFilter = "all" | "stdin" | "stdout" | "stderr";

  interface EnvVarRow {
    key: string;
    value: string;
    is_secret: boolean;
  }

  // 3. Props
  // (none - top-level panel)

  // 4. State
  let dialogMode = $state<DialogMode>(null);
  let editingAgent = $state<AcpAgentConfig | null>(null);
  let addTab = $state<AddTab>("manual");

  // Form fields
  let formName = $state("");
  let formAgentId = $state("");
  let formTransport = $state<AcpTransportKind>("stdio");
  let formExecutablePath = $state("");
  let formSpawnCommand = $state("");
  let formRemoteUrl = $state("");
  let formEnvVars = $state<EnvVarRow[]>([]);

  // Delete confirmation: stores the agent id that has confirmation open
  let deleteConfirmId = $state<string | null>(null);

  // Connecting state: agentRowId -> pending
  let connectingIds = $state<Set<string>>(new Set());

  // agentRowId -> connectionId mapping (local to panel)
  let agentToConnection = $state<Record<string, string>>({});

  // Log filter per connectionId
  let logFilters = $state<Record<string, LogFilter>>({});

  // Discover / registry
  let discoveredAgents = $state<DiscoveredAcpAgent[]>([]);
  let registryAgents = $state<AcpRegistryAgent[]>([]);
  let discoverLoading = $state(false);
  let registryLoading = $state(false);
  let discoverError = $state<string | null>(null);
  let registryError = $state<string | null>(null);
  let showGuide = $state(false);

  // Log scroll refs: connectionId -> element
  let logScrollEls = $state<Record<string, HTMLElement>>({});

  // Import file input ref
  let importFileInput = $state<HTMLInputElement | null>(null);

  // 5. Derived
  let connectedAgentIds = $derived(Object.keys(agentToConnection));

  // 6. Effects
  $effect(() => {
    // Auto-scroll log viewer when new lines arrive
    const expanded = acpStore.expandedConnectionId;
    if (!expanded) return;
    const logs = acpStore.logs[expanded];
    if (!logs) return;
    const el = logScrollEls[expanded];
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  });

  // 7. Functions

  // ── Status dot color ──────────────────────────────────────────────────────
  function statusDotColor(agent: AcpAgentConfig): string {
    const connId = agentToConnection[agent.id];
    if (!connId) return "var(--text-tertiary)";
    const connStatus = acpStore.connections[connId];
    if (!connStatus) return "var(--text-tertiary)";
    if (connStatus === "ready") return "var(--text-success)";
    if (connStatus === "initializing" || connStatus === "processing") return "var(--text-warning)";
    if (connStatus === "error") return "var(--text-error)";
    return "var(--text-tertiary)";
  }

  // ── Connect / disconnect ──────────────────────────────────────────────────
  async function handleConnect(agent: AcpAgentConfig) {
    connectingIds = new Set(connectingIds).add(agent.id);
    try {
      const connId = await acpStore.connect(agent.id);
      agentToConnection = { ...agentToConnection, [agent.id]: connId };
    } catch {
      // error surfaced via acpStore.error
    } finally {
      const next = new Set(connectingIds);
      next.delete(agent.id);
      connectingIds = next;
    }
  }

  async function handleDisconnect(agent: AcpAgentConfig) {
    const connId = agentToConnection[agent.id];
    if (!connId) return;
    try {
      await acpStore.disconnect(connId);
      const { [agent.id]: _, ...rest } = agentToConnection;
      agentToConnection = rest;
      if (acpStore.expandedConnectionId === connId) {
        acpStore.expandedConnectionId = null;
      }
    } catch {
      // error surfaced via acpStore.error
    }
  }

  // ── Log viewer toggle ─────────────────────────────────────────────────────
  function toggleLogs(connId: string) {
    if (acpStore.expandedConnectionId === connId) {
      acpStore.expandedConnectionId = null;
    } else {
      acpStore.expandedConnectionId = connId;
    }
  }

  function getLogFilter(connId: string): LogFilter {
    return logFilters[connId] ?? "all";
  }

  function setLogFilter(connId: string, f: LogFilter) {
    logFilters = { ...logFilters, [connId]: f };
  }

  function filteredLogs(connId: string) {
    const lines = acpStore.logs[connId] ?? [];
    const filter = getLogFilter(connId);
    if (filter === "all") return lines.slice(-200);
    return lines.filter((l) => l.direction === filter).slice(-200);
  }

  function logDirectionSymbol(dir: AcpLogDirection): string {
    if (dir === "stdin") return "→";
    if (dir === "stdout") return "←";
    return "!";
  }

  function logDirectionColor(dir: AcpLogDirection): string {
    if (dir === "stdin") return "var(--accent)";
    if (dir === "stdout") return "var(--text-secondary)";
    return "var(--text-warning)";
  }

  // ── Capabilities display ──────────────────────────────────────────────────
  function capabilityBadges(caps: Record<string, unknown> | null): string[] {
    if (!caps) return [];
    const badges: string[] = [];
    if (caps.terminal === true) badges.push("terminal");
    if (caps.fs && typeof caps.fs === "object") {
      const fs = caps.fs as Record<string, unknown>;
      if (fs.read_text_file === true) badges.push("fs:read");
      if (fs.write_text_file === true) badges.push("fs:write");
    }
    if (caps.session && typeof caps.session === "object") {
      const sess = caps.session as Record<string, unknown>;
      if (sess.list === true) badges.push("session:list");
      if (sess.resume === true) badges.push("session:resume");
      if (sess.close === true) badges.push("session:close");
    }
    return badges;
  }

  // ── Delete ────────────────────────────────────────────────────────────────
  function openDeleteConfirm(id: string) {
    deleteConfirmId = id;
  }

  function closeDeleteConfirm() {
    deleteConfirmId = null;
  }

  async function confirmDelete(agent: AcpAgentConfig) {
    try {
      await acpStore.deleteAgent(agent.id);
      const { [agent.id]: _, ...rest } = agentToConnection;
      agentToConnection = rest;
    } catch {
      // error surfaced via acpStore.error
    } finally {
      deleteConfirmId = null;
    }
  }

  // ── Dialog open / close ───────────────────────────────────────────────────
  function openAddDialog() {
    dialogMode = "add";
    editingAgent = null;
    addTab = "manual";
    resetForm();
    discoveredAgents = [];
    registryAgents = [];
    discoverError = null;
    registryError = null;
  }

  function openEditDialog(agent: AcpAgentConfig) {
    dialogMode = "edit";
    editingAgent = agent;
    formName = agent.name;
    formAgentId = agent.agent_id;
    formTransport = agent.transport;
    formExecutablePath = agent.executable_path ?? "";
    formSpawnCommand = agent.spawn_command ?? "";
    formRemoteUrl = agent.remote_url ?? "";
    formEnvVars = agent.env_vars.map((e) => ({ ...e }));
  }

  function closeDialog() {
    dialogMode = null;
    editingAgent = null;
    resetForm();
  }

  function resetForm() {
    formName = "";
    formAgentId = "";
    formTransport = "stdio";
    formExecutablePath = "";
    formSpawnCommand = "";
    formRemoteUrl = "";
    formEnvVars = [];
  }

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

  async function submitForm() {
    const agentId = formAgentId.trim();
    const envVars = formEnvVars.filter((e) => e.key.trim());

    // 1.6.8: Store secret env var values in the OS keychain.
    // The value saved to SQLite will be empty ("") for secret entries;
    // the real value is in the keychain, retrieved at launch time.
    for (const v of envVars) {
      if (v.is_secret && v.value) {
        try {
          await invoke("keychain_set", { agentId, key: v.key, value: v.value });
          // Replace value with sentinel so the DB never persists the secret.
          v.value = "";
        } catch (e) {
          console.warn("[AgentManagerPanel] keychain_set failed for", v.key, e);
          // Fall back to storing in DB if keychain is unavailable.
        }
      }
    }

    const payload = {
      name: formName.trim(),
      agent_id: agentId,
      transport: formTransport,
      executable_path: formTransport === "stdio" ? (formExecutablePath.trim() || null) : null,
      spawn_command: formTransport === "stdio" ? (formSpawnCommand.trim() || null) : null,
      remote_url: (formTransport === "http" || formTransport === "websocket") ? (formRemoteUrl.trim() || null) : null,
      env_vars: envVars,
    };

    try {
      if (dialogMode === "edit" && editingAgent) {
        await acpStore.updateAgent(editingAgent.id, payload);
      } else {
        await acpStore.createAgent(payload);
      }
      closeDialog();
    } catch {
      // error surfaced via acpStore.error
    }
  }

  // ── Discover tab ──────────────────────────────────────────────────────────
  async function runDiscover() {
    discoverLoading = true;
    discoverError = null;
    try {
      discoveredAgents = await acpStore.discoverAgents();
    } catch (e) {
      discoverError = String(e);
    } finally {
      discoverLoading = false;
    }
  }

  async function addDiscoveredAgent(d: DiscoveredAcpAgent) {
    try {
      await acpStore.createAgent({
        name: d.name,
        agent_id: d.agent_id,
        transport: "stdio",
        // Always use the recommended spawn command, never executable_path
        // alone - some agents (OpenCode) require a subcommand to run their
        // ACP server rather than their default interactive mode. Bug fixed:
        // this used to be spawn_command: null, which meant a discovered
        // OpenCode agent silently failed to speak ACP on connect.
        executable_path: d.executable_path,
        spawn_command: d.recommended_spawn_command,
        remote_url: null,
        env_vars: [],
      });
      closeDialog();
    } catch {
      // error surfaced via acpStore.error
    }
  }

  function addRegistryAgent(r: AcpRegistryAgent) {
    addTab = "manual";
    formName = r.name;
    formAgentId = r.id;
    formTransport = "stdio";
    if (r.id === "claude-acp" || r.id === "claude-code") {
      // claude itself has no ACP mode - the real entry point is the separate
      // claude-agent-acp adapter binary (npm: @agentclientprotocol/claude-agent-acp).
      formSpawnCommand = "npx -y @agentclientprotocol/claude-agent-acp@latest";
      formExecutablePath = "";
    } else if (r.id === "goose") {
      formSpawnCommand = "goose session --acp";
      formExecutablePath = "";
    } else if (r.id === "opencode") {
      formSpawnCommand = "opencode acp";
      formExecutablePath = "";
    } else if (r.id === "gemini-cli" || r.id === "gemini") {
      formSpawnCommand = "gemini --acp";
      formExecutablePath = "";
    } else if (r.id === "codex-acp" || r.id === "codex-cli" || r.id === "codex") {
      // codex itself has no ACP mode - the real entry point is the separate
      // codex-acp adapter binary (npm: @agentclientprotocol/codex-acp).
      formSpawnCommand = "npx -y @agentclientprotocol/codex-acp@latest";
      formExecutablePath = "";
    } else {
      formSpawnCommand = "";
      formExecutablePath = "";
    }
    formRemoteUrl = "";
    formEnvVars = [];
  }

  // ── Registry tab ──────────────────────────────────────────────────────────
  async function runFetchRegistry() {
    registryLoading = true;
    registryError = null;
    try {
      registryAgents = await acpStore.fetchRegistry();
    } catch (e) {
      registryError = String(e);
    } finally {
      registryLoading = false;
    }
  }

  // ── Export / import ───────────────────────────────────────────────────────
  async function exportAgent(agent: AcpAgentConfig) {
    try {
      const json = await acpStore.exportAgent(agent.id);
      const blob = new Blob([json], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${agent.name.replace(/\s+/g, "-")}.runyard-agent.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch {
      // error surfaced via acpStore.error
    }
  }

  function triggerImport() {
    importFileInput?.click();
  }

  async function handleImportFile(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    const text = await file.text();
    input.value = "";
    try {
      await acpStore.importAgent(text);
    } catch {
      // error surfaced via acpStore.error
    }
  }

  // ── Discover on tab switch ────────────────────────────────────────────────
  function handleTabSwitch(tab: AddTab) {
    addTab = tab;
    if (tab === "discover" && discoveredAgents.length === 0 && !discoverLoading) {
      runDiscover();
    }
    if (tab === "registry" && registryAgents.length === 0 && !registryLoading) {
      runFetchRegistry();
    }
  }

  // 8. Lifecycle
  onMount(() => {
    acpStore.loadAgents();
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

<div class="agent-manager-panel">
  <!-- Panel header -->
  <div class="panel-header">
    <span class="panel-title">AGENT MANAGER</span>
    <div class="header-actions">
      <button class="btn-ghost btn-sm" onclick={triggerImport} title="Import agent">
        <Upload size={12} />
        Import
      </button>
      <button class="btn-primary btn-sm" onclick={openAddDialog}>
        <Plus size={12} />
        Add agent
      </button>
    </div>
  </div>

  <!-- Global error -->
  {#if acpStore.error}
    <div class="global-error">{acpStore.error}</div>
  {/if}

  <!-- Agent list -->
  <div class="agent-list">
    {#if acpStore.isLoading}
      <div class="empty-state">
        <RefreshCw size={14} />
        <span>Loading agents...</span>
      </div>
    {:else if acpStore.agents.length === 0}
      <div class="empty-state">
        <Cpu size={16} />
        <p>No agents configured.</p>
        <button class="btn-primary btn-sm" onclick={openAddDialog}>
          <Plus size={12} />
          Add agent
        </button>
        <button class="btn-ghost btn-sm" onclick={() => { openAddDialog(); handleTabSwitch("discover"); }}>
          Discover agents
        </button>
      </div>
    {:else}
      {#each acpStore.agents as agent (agent.id)}
        {@const connId = agentToConnection[agent.id] ?? null}
        {@const isConnected = connId !== null && acpStore.connections[connId] !== undefined}
        {@const isConnecting = connectingIds.has(agent.id)}
        {@const sessionList = connId ? (acpStore.sessions[connId] ?? []) : []}
        {@const logExpanded = connId !== null && acpStore.expandedConnectionId === connId}
        {@const caps = capabilityBadges(agent.capabilities)}
        {@const dotColor = statusDotColor(agent)}

        <div class="agent-row-wrapper">
          <!-- Main agent row -->
          <div class="agent-row">
            <div class="agent-main">
              <span class="status-dot" style="background:{dotColor}"></span>
              <div class="agent-info">
                <div class="agent-name-line">
                  <span class="agent-name">{agent.name}</span>
                  <span class="badge transport-badge">{agent.transport}</span>
                  <span class="agent-id-text">{agent.agent_id}</span>
                </div>
                {#if caps.length > 0}
                  <div class="capability-badges">
                    {#each caps as cap}
                      <span class="badge cap-badge">{cap}</span>
                    {/each}
                  </div>
                {/if}
                {#if isConnected && connId}
                  <div class="connection-meta">
                    <span class="session-count">{sessionList.length} session{sessionList.length !== 1 ? "s" : ""}</span>
                    <button
                      class="text-link"
                      onclick={() => toggleLogs(connId)}
                    >
                      {logExpanded ? "Hide logs" : "View logs"}
                    </button>
                  </div>
                {/if}
              </div>
            </div>

            <div class="agent-actions">
              {#if isConnected && connId}
                <button class="btn-secondary btn-sm" onclick={() => handleDisconnect(agent)}>
                  <PlugZap size={11} />
                  Disconnect
                </button>
              {:else}
                <button
                  class="btn-primary btn-sm"
                  disabled={isConnecting}
                  onclick={() => handleConnect(agent)}
                >
                  <Plug size={11} />
                  {isConnecting ? "Connecting..." : "Connect"}
                </button>
              {/if}
              <button
                class="btn-icon"
                title="Export agent"
                onclick={() => exportAgent(agent)}
              >
                <Download size={12} />
              </button>
              <button
                class="btn-icon"
                title="Edit agent"
                onclick={() => openEditDialog(agent)}
              >
                <Pencil size={12} />
              </button>
              <button
                class="btn-icon btn-icon-danger"
                title="Delete agent"
                onclick={() => openDeleteConfirm(agent.id)}
              >
                <Trash2 size={12} />
              </button>
            </div>
          </div>

          <!-- Delete confirmation inline -->
          {#if deleteConfirmId === agent.id}
            <div class="delete-confirm">
              <span class="delete-msg">
                This will remove <strong>{agent.name}</strong> and disconnect any active sessions.
              </span>
              <div class="delete-actions">
                <button class="btn-danger btn-sm" onclick={() => confirmDelete(agent)}>
                  Remove
                </button>
                <button class="btn-ghost btn-sm" onclick={closeDeleteConfirm}>
                  Cancel
                </button>
              </div>
            </div>
          {/if}

          <!-- Log viewer -->
          {#if logExpanded && connId}
            {@const logLines = filteredLogs(connId)}
            {@const currentFilter = getLogFilter(connId)}
            <div class="log-viewer">
              <div class="log-header">
                <div class="log-filters">
                  {#each (["all", "stdin", "stdout", "stderr"] as LogFilter[]) as f}
                    <button
                      class="filter-btn"
                      class:filter-active={currentFilter === f}
                      onclick={() => setLogFilter(connId, f)}
                    >
                      {f}
                    </button>
                  {/each}
                </div>
                <button
                  class="btn-ghost btn-sm"
                  onclick={() => acpStore.clearLogs(connId)}
                >
                  Clear
                </button>
              </div>
              <div
                class="log-lines"
                bind:this={logScrollEls[connId]}
              >
                {#if logLines.length === 0}
                  <span class="log-empty">No log output.</span>
                {:else}
                  {#each logLines as entry}
                    <div class="log-entry">
                      <span
                        class="log-dir"
                        style="color:{logDirectionColor(entry.direction)}"
                      >{logDirectionSymbol(entry.direction)}</span>
                      <span
                        class="log-line"
                        title={entry.line}
                      >{entry.line.length > 120 ? entry.line.slice(0, 120) + "…" : entry.line}</span>
                    </div>
                  {/each}
                {/if}
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
          {dialogMode === "edit" ? "Edit agent" : "Add agent"}
        </h2>
        <button class="btn-icon" onclick={closeDialog} aria-label="Close">✕</button>
      </div>

      {#if dialogMode === "add"}
        <!-- Tabs -->
        <div class="dialog-tabs">
          {#each (["manual", "discover", "registry"] as AddTab[]) as tab}
            <button
              class="dialog-tab"
              class:tab-active={addTab === tab}
              onclick={() => handleTabSwitch(tab)}
            >
              {tab === "manual" ? "Manual" : tab === "discover" ? "Discover" : "Registry"}
            </button>
          {/each}
        </div>
      {/if}

      {#if dialogMode === "edit" || addTab === "manual"}
        <!-- Manual form -->
        <form class="agent-form" onsubmit={(e) => { e.preventDefault(); submitForm(); }}>
          <div class="form-row">
            <label class="form-label" for="f-name">Name</label>
            <input
              id="f-name"
              class="form-input"
              type="text"
              placeholder="My Agent"
              bind:value={formName}
              required
            />
          </div>
          <div class="form-row">
            <label class="form-label" for="f-agent-id">Agent ID</label>
            <input
              id="f-agent-id"
              class="form-input"
              type="text"
              placeholder="com.example.agent"
              bind:value={formAgentId}
              required
            />
          </div>
          <div class="form-row">
            <label class="form-label" for="f-transport">Transport</label>
            <select id="f-transport" class="form-select" bind:value={formTransport}>
              <option value="stdio">stdio</option>
              <option value="http">http</option>
              <option value="websocket">websocket</option>
            </select>
          </div>

          {#if formTransport === "stdio"}
            <div class="form-row">
              <label class="form-label" for="f-exec">Executable path</label>
              <input
                id="f-exec"
                class="form-input"
                type="text"
                placeholder="/usr/local/bin/my-agent"
                bind:value={formExecutablePath}
              />
            </div>
            <div class="form-row">
              <label class="form-label" for="f-spawn">Spawn command</label>
              <input
                id="f-spawn"
                class="form-input"
                type="text"
                placeholder="my-agent --serve"
                bind:value={formSpawnCommand}
              />
            </div>
          {:else}
            <div class="form-row">
              <label class="form-label" for="f-remote-url">Remote URL</label>
              <input
                id="f-remote-url"
                class="form-input"
                type="text"
                placeholder="http://localhost:8080"
                bind:value={formRemoteUrl}
              />
            </div>
          {/if}

          <!-- Env vars -->
          <div class="env-section">
            <div class="env-header">
              <span class="form-label">Environment variables</span>
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
                  aria-label="Remove"
                >
                  <Trash2 size={11} />
                </button>
              </div>
            {/each}
          </div>

          <div class="form-footer">
            <button type="button" class="btn-ghost" onclick={closeDialog}>Cancel</button>
            <button type="submit" class="btn-primary" disabled={!formName.trim() || !formAgentId.trim()}>
              {dialogMode === "edit" ? "Save changes" : "Add agent"}
            </button>
          </div>
        </form>
      {:else if addTab === "discover"}
        <div class="tab-content">
          {#if discoverLoading}
            <div class="tab-loading">
              <RefreshCw size={14} />
              <span>Scanning PATH...</span>
            </div>
          {:else if discoverError}
            <div class="tab-error">{discoverError}</div>
            <button class="btn-ghost btn-sm" onclick={runDiscover}>Retry</button>
          {:else if discoveredAgents.length === 0}
            <div class="tab-empty">No agents found on PATH.</div>
            <button class="btn-ghost btn-sm" onclick={runDiscover}>Scan again</button>
          {:else}
            {#each discoveredAgents as d}
              <div class="discovered-row">
                <div class="discovered-info">
                  <span class="discovered-name">{d.name}</span>
                  <span class="discovered-path">{d.executable_path}</span>
                  <span class="discovered-id">{d.agent_id}</span>
                </div>
                <button class="btn-primary btn-sm" onclick={() => addDiscoveredAgent(d)}>
                  Add
                </button>
              </div>
            {/each}
          {/if}
        </div>
      {:else if addTab === "registry"}
        <div class="tab-content">
          {#if registryLoading}
            <div class="tab-loading">
              <RefreshCw size={14} />
              <span>Fetching registry...</span>
            </div>
          {:else if registryError}
            <div class="tab-error">{registryError}</div>
            <button class="btn-ghost btn-sm" onclick={runFetchRegistry}>Retry</button>
          {:else if registryAgents.length === 0}
            <div class="tab-empty">Registry is empty or unavailable.</div>
          {:else}
            {#each registryAgents as r}
              <div class="registry-row">
                <div class="registry-info">
                  <div class="registry-name-line">
                    <span class="registry-name">{r.name}</span>
                    <span class="badge">{r.version}</span>
                  </div>
                  <span class="registry-desc">{r.description}</span>
                  <div class="registry-meta-line">
                    {#if r.repository}
                      <a class="registry-link" href={r.repository} target="_blank" rel="noopener noreferrer">
                        Repository
                      </a>
                    {/if}
                    {#if r.authors && r.authors.length > 0}
                      <span class="registry-authors">by {r.authors.join(", ")}</span>
                    {/if}
                  </div>
                </div>
                <button class="btn-primary btn-sm" onclick={() => addRegistryAgent(r)}>
                  Use
                </button>
              </div>
            {/each}
          {/if}
        </div>
      {/if}

      {#if dialogMode === "add"}
        <div class="acp-guide-section">
          <button type="button" class="guide-toggle" onclick={() => showGuide = !showGuide}>
            <span>ACP agent setup and installation guide</span>
            {#if showGuide}
              <ChevronUp size={14} strokeWidth={1.5} />
            {:else}
              <ChevronDown size={14} strokeWidth={1.5} />
            {/if}
          </button>

          {#if showGuide}
            <div class="guide-content">
              <p class="guide-intro">
                ACP (Agent Client Protocol) lets Runyard talk directly to local coding agents over stdio. Install the agent's own CLI, then use the spawn command shown below.
              </p>

              <div class="guide-list">
                <div class="guide-item">
                  <span class="agent-title">OpenCode</span>
                  <p class="agent-desc">Install the OpenCode CLI globally, then start it in ACP server mode with the <code>acp</code> subcommand:</p>
                  <pre class="code-block">npm install -g opencode-ai</pre>
                  <p class="agent-tip">Spawn command: <code>opencode acp</code></p>
                </div>

                <div class="guide-item">
                  <span class="agent-title">Claude Code</span>
                  <p class="agent-desc">Claude Code's own CLI has no ACP mode. Install the separate ACP adapter instead:</p>
                  <pre class="code-block">npm install -g @agentclientprotocol/claude-agent-acp</pre>
                  <p class="agent-tip">Spawn command: <code>claude-agent-acp</code> or <code>npx -y @agentclientprotocol/claude-agent-acp@latest</code></p>
                </div>

                <div class="guide-item">
                  <span class="agent-title">Codex CLI</span>
                  <p class="agent-desc">Codex CLI's own CLI has no ACP mode. Install the separate ACP adapter instead:</p>
                  <pre class="code-block">npm install -g @agentclientprotocol/codex-acp</pre>
                  <p class="agent-tip">Spawn command: <code>codex-acp</code> or <code>npx -y @agentclientprotocol/codex-acp@latest</code></p>
                </div>

                <div class="guide-item">
                  <span class="agent-title">Goose</span>
                  <p class="agent-desc">Install Block's Goose agent via your package manager or installer:</p>
                  <pre class="code-block">brew install goose</pre>
                  <p class="agent-tip">Spawn command: <code>goose session --acp</code></p>
                </div>

                <div class="guide-item">
                  <span class="agent-title">Gemini CLI</span>
                  <p class="agent-desc">Install the Gemini CLI tool globally, then start it in ACP mode with the <code>--acp</code> flag:</p>
                  <pre class="code-block">npm install -g @google/gemini-cli</pre>
                  <p class="agent-tip">Spawn command: <code>gemini --acp</code></p>
                </div>
              </div>

              <div class="guide-note">
                Runyard resolves the spawn command's executable through your system PATH on any platform. If an agent isn't found, use the "Discover" tab to scan PATH automatically, or provide an absolute path in "Executable path" above.
              </div>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}

<style>
  /* Root */
  .agent-manager-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    font-size: 12px;
    color: var(--text);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 0;
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
  }

  /* Agent list */
  .agent-list {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 32px 16px;
    color: var(--text-tertiary);
    font-size: 12px;
    text-align: center;
  }

  .empty-state p {
    margin: 0;
  }

  /* Agent rows */
  .agent-row-wrapper {
    border-bottom: 1px solid var(--border-secondary);
  }

  .agent-row {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px 10px;
  }

  .agent-row:hover {
    background: var(--bg-secondary);
  }

  .agent-main {
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

  .agent-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .agent-name-line {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .agent-name {
    font-weight: 500;
    color: var(--text);
    white-space: nowrap;
  }

  .agent-id-text {
    font-size: 11px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .capability-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .connection-meta {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 11px;
    color: var(--text-secondary);
  }

  .session-count {
    font-size: 11px;
  }

  .text-link {
    background: none;
    border: none;
    color: var(--accent);
    cursor: pointer;
    font-size: 11px;
    font-family: inherit;
    padding: 0;
  }

  .text-link:hover {
    text-decoration: underline;
  }

  .agent-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
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

  /* Log viewer */
  .log-viewer {
    border-top: 1px solid var(--border-secondary);
    background: var(--bg-secondary);
  }

  .log-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 8px;
    border-bottom: 1px solid var(--border-secondary);
  }

  .log-filters {
    display: flex;
    gap: 2px;
  }

  .filter-btn {
    background: none;
    border: none;
    border-radius: 2px;
    color: var(--text-secondary);
    font-size: 11px;
    font-family: inherit;
    padding: 2px 6px;
    cursor: pointer;
  }

  .filter-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text);
  }

  .filter-btn.filter-active {
    background: var(--bg-tertiary);
    color: var(--text);
    font-weight: 500;
  }

  .log-lines {
    max-height: 240px;
    overflow-y: auto;
    padding: 4px 0;
    font-family: "JetBrains Mono", ui-monospace, monospace;
    font-size: 11px;
  }

  .log-empty {
    display: block;
    padding: 8px 10px;
    color: var(--text-tertiary);
    font-style: italic;
  }

  .log-entry {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 1px 10px;
  }

  .log-entry:hover {
    background: var(--bg-tertiary);
  }

  .log-dir {
    flex-shrink: 0;
    font-weight: 600;
    width: 12px;
    text-align: center;
  }

  .log-line {
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Badges */
  .badge {
    background: var(--bg-tertiary);
    border-radius: 2px;
    font-size: 11px;
    font-weight: 500;
    padding: 2px 6px;
    color: var(--text-secondary);
    white-space: nowrap;
  }

  .transport-badge {
    color: var(--text-tertiary);
  }

  .cap-badge {
    font-size: 10px;
    padding: 1px 4px;
  }

  /* Buttons */
  .btn-primary {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: var(--accent);
    color: var(--bg);
    border: none;
    border-radius: 2px;
    padding: 4px 10px;
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

  .btn-secondary {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 4px 10px;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
    white-space: nowrap;
  }

  .btn-secondary:hover {
    background: var(--bg-tertiary);
    color: var(--text);
  }

  .btn-ghost {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 4px 10px;
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
    color: #fff;
    border: none;
    border-radius: 2px;
    padding: 4px 10px;
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
    border-radius: 2px;
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
    background: var(--bg-overlay, rgba(0, 0, 0, 0.5));
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal {
    background: var(--bg-elevated);
    border-radius: 6px;
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

  /* Dialog tabs */
  .dialog-tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    gap: 0;
  }

  .dialog-tab {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 12px;
    font-family: inherit;
    padding: 6px 14px;
    margin-bottom: -1px;
  }

  .dialog-tab:hover {
    color: var(--text);
  }

  .dialog-tab.tab-active {
    color: var(--text);
    border-bottom-color: var(--accent);
    font-weight: 500;
  }

  /* Forms */
  .agent-form {
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

  .form-input {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 2px;
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

  .form-select {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 2px;
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

  .form-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 4px;
    border-top: 1px solid var(--border);
  }

  /* Tab content */
  .tab-content {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 50vh;
    overflow-y: auto;
  }

  .tab-loading {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 12px;
    padding: 12px 0;
  }

  .tab-error {
    color: var(--text-error);
    font-size: 12px;
    padding: 8px 0;
  }

  .tab-empty {
    color: var(--text-tertiary);
    font-size: 12px;
    padding: 12px 0;
  }

  /* Discovered agents */
  .discovered-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border-secondary);
  }

  .discovered-info {
    display: flex;
    flex-direction: column;
    gap: 2px;
    flex: 1;
    min-width: 0;
  }

  .discovered-name {
    font-weight: 500;
    color: var(--text);
    font-size: 12px;
  }

  .discovered-path {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .discovered-id {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  /* Registry agents */
  .registry-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 10px 0;
    border-bottom: 1px solid var(--border-secondary);
  }

  .registry-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 0;
  }

  .registry-name-line {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .registry-name {
    font-weight: 500;
    color: var(--text);
    font-size: 12px;
  }

  .registry-desc {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .registry-meta-line {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .registry-link {
    font-size: 11px;
    color: var(--accent);
    text-decoration: none;
  }

  .registry-link:hover {
    text-decoration: underline;
  }

  .registry-authors {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  /* ACP Setup Guide */
  .acp-guide-section {
    margin-top: 16px;
    border-top: 1px solid var(--border-secondary);
    padding-top: 12px;
  }

  .guide-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px 12px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .guide-toggle:hover {
    background: var(--border-secondary);
    color: var(--text);
  }

  .guide-content {
    margin-top: 10px;
    padding: 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    max-height: 250px;
    overflow-y: auto;
  }

  .guide-intro {
    font-size: 11px;
    line-height: 1.4;
    color: var(--text-secondary);
    margin: 0;
  }

  .guide-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .guide-item {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .agent-title {
    font-weight: 600;
    font-size: 11px;
    color: var(--text);
  }

  .agent-desc {
    font-size: 11px;
    color: var(--text-secondary);
    margin: 0;
  }

  .code-block {
    background: #000;
    color: #a7f3d0;
    font-family: var(--font-mono, monospace);
    font-size: 11px;
    padding: 6px 10px;
    border-radius: 4px;
    margin: 4px 0;
    overflow-x: auto;
    border: 1px solid #10b98133;
  }

  .agent-tip {
    font-size: 10px;
    color: var(--text-tertiary);
    margin: 0;
  }

  .agent-tip code {
    font-family: var(--font-mono, monospace);
    color: var(--text-secondary);
    background: var(--border-secondary);
    padding: 1px 4px;
    border-radius: 3px;
  }

  .guide-note {
    font-size: 10px;
    color: var(--text-secondary);
    border-left: 2px solid var(--accent);
    padding-left: 8px;
    line-height: 1.4;
  }
</style>
