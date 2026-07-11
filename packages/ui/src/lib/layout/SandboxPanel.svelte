<script lang="ts">
  import { invoke as tauriInvoke } from "@tauri-apps/api/core";
  import { webSocketClient } from "@runyard/common";
  import { onMount, onDestroy } from "svelte";
  import type { AcpAgentConfig } from "@runyard/common";

  async function invoke<T>(cmd: string, args?: any): Promise<T> {
    if (webSocketClient.status === "connected") {
      return webSocketClient.invoke<T>(cmd, args);
    } else {
      return tauriInvoke<T>(cmd, args);
    }
  }

  interface SandboxConfig {
    agentId: string;
    allowedRoots: string;
    maxFileBytes: number;
    toolTimeoutSecs: number;
    allowNetwork: boolean;
  }

  interface AuditLogEntry {
    id: string;
    agentId: string;
    connectionId: string | null;
    sessionId: string | null;
    tool: string;
    argsJson: string;
    outcome: string;
    deniedReason: string | null;
    createdAt: number;
  }

  // Agent list
  let agents = $state<AcpAgentConfig[]>([]);
  let selectedAgentId = $state<string>("");

  // Config form state
  let configAllowedRoots = $state("");
  let configMaxFileMb = $state(10);
  let configTimeoutMins = $state(5);
  let configAllowNetwork = $state(false);
  let configLoading = $state(false);
  let configError = $state<string | null>(null);
  let configSaved = $state(false);

  // Audit log state
  let auditLog = $state<AuditLogEntry[]>([]);
  let auditLogVisible = $state<AuditLogEntry[]>([]);
  let auditError = $state<string | null>(null);
  let auditCleared = $state(false);

  async function loadAgents() {
    try {
      const list = await invoke<AcpAgentConfig[]>("acp_agent_list");
      agents = list;
      if (!selectedAgentId && list.length > 0) {
        selectedAgentId = (list[0] as any).id ?? "";
        await loadConfig();
      }
    } catch (e) {
      console.error("[SandboxPanel] Failed to load agents", e);
    }
  }

  async function loadConfig() {
    if (!selectedAgentId) return;
    configLoading = true;
    configError = null;
    try {
      const cfg = await invoke<SandboxConfig>("sandbox_get_config", { agentId: selectedAgentId });
      configAllowedRoots = cfg.allowedRoots ?? "";
      configMaxFileMb = Math.round((cfg.maxFileBytes ?? 10485760) / (1024 * 1024));
      configTimeoutMins = Math.round((cfg.toolTimeoutSecs ?? 300) / 60);
      configAllowNetwork = cfg.allowNetwork ?? false;
    } catch (e) {
      // Backend may not have an entry yet — use defaults silently
      configAllowedRoots = "";
      configMaxFileMb = 10;
      configTimeoutMins = 5;
      configAllowNetwork = false;
    } finally {
      configLoading = false;
    }
  }

  async function saveConfig() {
    if (!selectedAgentId) return;
    configError = null;
    configSaved = false;
    try {
      await invoke("sandbox_set_config", {
        agentId: selectedAgentId,
        allowedRoots: configAllowedRoots,
        maxFileBytes: configMaxFileMb * 1024 * 1024,
        toolTimeoutSecs: configTimeoutMins * 60,
        allowNetwork: configAllowNetwork,
      });
      configSaved = true;
      setTimeout(() => { configSaved = false; }, 2000);
    } catch (e) {
      configError = String(e);
    }
  }

  async function loadAuditLog() {
    auditError = null;
    try {
      const entries = await invoke<AuditLogEntry[]>("sandbox_get_audit_log", { limit: 100 });
      auditLog = entries;
      auditLogVisible = [...entries];
      auditCleared = false;
    } catch (e) {
      auditError = String(e);
    }
  }

  function clearAuditDisplay() {
    auditLogVisible = [];
    auditCleared = true;
  }

  function outcomeColor(outcome: string): string {
    if (outcome === "allowed") return "var(--text-success)";
    if (outcome === "denied") return "var(--text-error)";
    return "var(--text-warning)";
  }

  function formatTs(createdAt: number): string {
    return new Date(createdAt).toLocaleTimeString();
  }

  function truncateArgs(argsJson: string, maxLen = 60): string {
    if (argsJson.length <= maxLen) return argsJson;
    return argsJson.slice(0, maxLen) + "…";
  }

  let auditRefreshInterval: ReturnType<typeof setInterval>;

  onMount(async () => {
    await loadAgents();
    await loadAuditLog();
    auditRefreshInterval = setInterval(loadAuditLog, 10000);
  });

  onDestroy(() => {
    clearInterval(auditRefreshInterval);
  });
</script>

<div class="sandbox-panel">
  <!-- Header -->
  <div class="panel-header">
    <span class="panel-title">SANDBOX</span>
  </div>

  <div class="sandbox-body">
    <!-- Section 1: Per-agent config -->
    <section class="config-section">
      <div class="section-header">
        <span class="section-title">Per-agent config</span>
      </div>

      <div class="form-row">
        <label class="form-label" for="sandbox-agent-select">Agent</label>
        <select
          id="sandbox-agent-select"
          class="form-select"
          bind:value={selectedAgentId}
          onchange={loadConfig}
        >
          {#each agents as agent}
            <option value={(agent as any).id ?? ""}>{(agent as any).name ?? (agent as any).id ?? "Unknown"}</option>
          {/each}
          {#if agents.length === 0}
            <option value="" disabled>No agents configured</option>
          {/if}
        </select>
      </div>

      {#if configLoading}
        <div class="form-hint">Loading config…</div>
      {:else}
        <div class="form-row">
          <label class="form-label" for="sandbox-roots">Allowed roots</label>
          <textarea
            id="sandbox-roots"
            class="form-textarea"
            placeholder="One path per line"
            rows={4}
            bind:value={configAllowedRoots}
          ></textarea>
          <span class="form-hint">One absolute path per line</span>
        </div>

        <div class="form-row">
          <label class="form-label" for="sandbox-max-file">Max file size (MB)</label>
          <input
            id="sandbox-max-file"
            type="number"
            class="form-input"
            min={1}
            bind:value={configMaxFileMb}
          />
        </div>

        <div class="form-row">
          <label class="form-label" for="sandbox-timeout">Tool timeout (minutes)</label>
          <input
            id="sandbox-timeout"
            type="number"
            class="form-input"
            min={1}
            bind:value={configTimeoutMins}
          />
        </div>

        <div class="form-row form-row-check">
          <label class="form-label" for="sandbox-network">Allow network</label>
          <input
            id="sandbox-network"
            type="checkbox"
            class="form-checkbox"
            bind:checked={configAllowNetwork}
          />
        </div>

        {#if configError}
          <div class="form-error">{configError}</div>
        {/if}

        <div class="form-actions">
          <button class="save-btn" onclick={saveConfig}>Save</button>
          {#if configSaved}
            <span class="save-ok">Saved</span>
          {/if}
        </div>
      {/if}
    </section>

    <!-- Section 2: Audit log -->
    <section class="audit-section">
      <div class="section-header">
        <span class="section-title">Audit log</span>
        <button class="ghost-btn" onclick={clearAuditDisplay}>Clear display</button>
        <button class="ghost-btn" onclick={loadAuditLog}>Refresh</button>
      </div>

      {#if auditError}
        <div class="form-error">{auditError}</div>
      {/if}

      {#if auditCleared}
        <div class="audit-cleared">Display cleared. Entries are still stored.</div>
      {:else if auditLogVisible.length === 0}
        <div class="audit-empty">No audit log entries.</div>
      {:else}
        <div class="audit-table-wrapper">
          <table class="audit-table">
            <thead>
              <tr>
                <th>Time</th>
                <th>Agent</th>
                <th>Tool</th>
                <th>Args</th>
                <th>Outcome</th>
              </tr>
            </thead>
            <tbody>
              {#each auditLogVisible as entry (entry.id)}
                <tr>
                  <td class="mono">{formatTs(entry.createdAt)}</td>
                  <td class="mono">{entry.agentId}</td>
                  <td class="mono">{entry.tool}</td>
                  <td class="mono args-cell" title={entry.argsJson}>{truncateArgs(entry.argsJson)}</td>
                  <td>
                    <span class="outcome-badge" style="color: {outcomeColor(entry.outcome)};">
                      {entry.outcome}
                    </span>
                    {#if entry.deniedReason}
                      <span class="denied-reason" title={entry.deniedReason}>
                        — {entry.deniedReason.slice(0, 40)}{entry.deniedReason.length > 40 ? "…" : ""}
                      </span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .sandbox-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
  }

  /* Header */
  .panel-header {
    display: flex;
    align-items: center;
    gap: 10px;
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

  .sandbox-body {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  /* Sections */
  .config-section,
  .audit-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
    padding: 10px;
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }

  .section-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-secondary);
    flex: 1;
  }

  /* Form elements */
  .form-row {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .form-row-check {
    flex-direction: row;
    align-items: center;
    gap: 8px;
  }

  .form-label {
    font-size: 11px;
    color: var(--text-tertiary);
    font-weight: 600;
  }

  .form-select,
  .form-input {
    font-size: 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 4px 8px;
    color: var(--text);
    font-family: var(--font-sans);
    outline: none;
    width: 100%;
    box-sizing: border-box;
  }

  .form-select:focus,
  .form-input:focus {
    border-color: var(--border-active);
  }

  .form-textarea {
    font-size: 12px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 4px 8px;
    color: var(--text);
    font-family: var(--font-mono);
    outline: none;
    width: 100%;
    box-sizing: border-box;
    resize: vertical;
    min-height: 72px;
  }

  .form-textarea:focus {
    border-color: var(--border-active);
  }

  .form-checkbox {
    cursor: pointer;
    width: 14px;
    height: 14px;
  }

  .form-hint {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .form-error {
    font-size: 11px;
    color: var(--text-error);
  }

  .form-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
  }

  .save-btn {
    font-size: 11px;
    padding: 3px 12px;
    border-radius: 2px;
    border: 1px solid var(--border-active);
    background: var(--accent);
    color: var(--bg);
    cursor: pointer;
    font-family: var(--font-sans);
    font-weight: 500;
  }

  .save-btn:hover {
    opacity: 0.85;
  }

  .save-ok {
    font-size: 11px;
    color: var(--text-success);
  }

  .ghost-btn {
    font-size: 11px;
    color: var(--text-secondary);
    background: none;
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 2px 8px;
    cursor: pointer;
    white-space: nowrap;
    font-family: var(--font-sans);
  }

  .ghost-btn:hover {
    color: var(--text);
    border-color: var(--border-active);
    background: var(--bg-elevated);
  }

  /* Audit table */
  .audit-table-wrapper {
    overflow-x: auto;
    overflow-y: auto;
    max-height: 360px;
    border: 1px solid var(--border);
    border-radius: var(--radius-0);
  }

  .audit-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 11px;
  }

  .audit-table thead th {
    position: sticky;
    top: 0;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    padding: 4px 8px;
    text-align: left;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.04em;
    color: var(--text-tertiary);
    white-space: nowrap;
    text-transform: uppercase;
  }

  .audit-table tbody tr:nth-child(even) {
    background: var(--bg-secondary);
  }

  .audit-table tbody tr:hover {
    background: var(--bg-elevated);
  }

  .audit-table td {
    padding: 3px 8px;
    vertical-align: top;
    border-bottom: 1px solid var(--border);
    color: var(--text-secondary);
  }

  .mono {
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .args-cell {
    max-width: 220px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    cursor: help;
  }

  .outcome-badge {
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
  }

  .denied-reason {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .audit-empty,
  .audit-cleared {
    font-size: 12px;
    color: var(--text-tertiary);
    padding: 12px 0;
    text-align: center;
  }
</style>
