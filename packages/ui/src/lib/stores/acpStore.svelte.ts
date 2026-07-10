import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { webSocketClient } from "@runyard/common";
import type {
  AcpAgentConfig,
  AcpConnectionStatusValue,
  AcpLogDirection,
  AcpClientEvent,
  DiscoveredAcpAgent,
  AcpRegistryAgent,
} from "@runyard/common";

async function invoke<T>(cmd: string, args?: any): Promise<T> {
  if (webSocketClient.status === "connected") {
    return webSocketClient.invoke<T>(cmd, args);
  } else {
    return tauriInvoke<T>(cmd, args);
  }
}

interface LogEntry {
  direction: AcpLogDirection;
  line: string;
  ts: number;
}

const LOG_CAP = 2000;

class AcpStore {
  // ── Reactive state ──────────────────────────────────────────────────────────
  agents = $state<AcpAgentConfig[]>([]);
  connections = $state<Record<string, AcpConnectionStatusValue>>({});
  sessions = $state<Record<string, string[]>>({});
  logs = $state<Record<string, LogEntry[]>>({});
  isLoading = $state(false);
  error = $state<string | null>(null);
  selectedAgentId = $state<string | null>(null);
  expandedConnectionId = $state<string | null>(null);

  constructor() {
    // $effect.root creates an independent reactive root that is NOT tied to a
    // component lifecycle - correct for stores instantiated at module scope.
    $effect.root(() => {
      this.setupEventListener();
    });
    this.loadAgents();
  }

  // ── Private: event listener ─────────────────────────────────────────────────
  private setupEventListener(): () => void {
    let unlistenFn: (() => void) | null = null;

    listen<AcpClientEvent>("acp:event", (event) => {
      const payload = event.payload;
      const connId = payload.connection_id;

      switch (payload.type) {
        case "connected": {
          this.connections = { ...this.connections, [connId]: "ready" };
          break;
        }
        case "disconnected": {
          const { [connId]: _conn, ...restConns } = this.connections;
          const { [connId]: _sess, ...restSess } = this.sessions;
          const { [connId]: _logs, ...restLogs } = this.logs;
          this.connections = restConns;
          this.sessions = restSess;
          this.logs = restLogs;
          break;
        }
        case "status_changed": {
          this.connections = { ...this.connections, [connId]: payload.status };
          break;
        }
        case "session_started": {
          const existing = this.sessions[connId] ?? [];
          this.sessions = {
            ...this.sessions,
            [connId]: [...existing, payload.session_id],
          };
          break;
        }
        case "session_closed": {
          const filtered = (this.sessions[connId] ?? []).filter(
            (sid) => sid !== payload.session_id
          );
          this.sessions = { ...this.sessions, [connId]: filtered };
          break;
        }
        case "log_line": {
          const existingLogs = this.logs[connId] ?? [];
          const entry: LogEntry = {
            direction: payload.direction,
            line: payload.line,
            ts: Date.now(),
          };
          const updated =
            existingLogs.length >= LOG_CAP
              ? [...existingLogs.slice(existingLogs.length - (LOG_CAP - 1)), entry]
              : [...existingLogs, entry];
          this.logs = { ...this.logs, [connId]: updated };
          break;
        }
        default: {
          window.dispatchEvent(
            new CustomEvent(`acp:${payload.type}`, { detail: payload })
          );
          break;
        }
      }
    }).then((fn) => {
      unlistenFn = fn;
    });

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }

  // ── Agent CRUD ──────────────────────────────────────────────────────────────
  async loadAgents() {
    this.isLoading = true;
    this.error = null;
    try {
      const list = await invoke<AcpAgentConfig[]>("acp_agent_list");
      this.agents = list;
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_agent_list failed", e);
    } finally {
      this.isLoading = false;
    }
  }

  async createAgent(params: {
    name: string;
    agentId: string;
    transport: string;
    executablePath?: string | null;
    spawnCommand?: string | null;
    remoteUrl?: string | null;
    envVars?: Array<{ key: string; value: string; isSecret: boolean }>;
  }) {
    this.error = null;
    try {
      await invoke("acp_agent_create", params);
      await this.loadAgents();
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_agent_create failed", e);
      throw e;
    }
  }

  async updateAgent(id: string, params: Partial<{
    name: string;
    agentId: string;
    transport: string;
    executablePath: string | null;
    spawnCommand: string | null;
    remoteUrl: string | null;
    envVars: Array<{ key: string; value: string; isSecret: boolean }>;
    isActive: boolean;
  }>) {
    this.error = null;
    try {
      await invoke("acp_agent_update", { id, ...params });
      await this.loadAgents();
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_agent_update failed", e);
      throw e;
    }
  }

  async deleteAgent(id: string) {
    this.error = null;
    try {
      await invoke("acp_agent_delete", { id });
      await this.loadAgents();
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_agent_delete failed", e);
      throw e;
    }
  }

  async setAgentActive(id: string, isActive: boolean) {
    this.error = null;
    try {
      await invoke("acp_agent_set_active", { id, isActive });
      await this.loadAgents();
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_agent_set_active failed", e);
      throw e;
    }
  }

  async exportAgent(id: string): Promise<string> {
    try {
      return await invoke<string>("acp_agent_export", { id });
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_agent_export failed", e);
      throw e;
    }
  }

  async importAgent(json: string) {
    this.error = null;
    try {
      await invoke("acp_agent_import", { json });
      await this.loadAgents();
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_agent_import failed", e);
      throw e;
    }
  }

  async discoverAgents(): Promise<DiscoveredAcpAgent[]> {
    try {
      return await invoke<DiscoveredAcpAgent[]>("acp_agent_discover");
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_agent_discover failed", e);
      throw e;
    }
  }

  async fetchRegistry(): Promise<AcpRegistryAgent[]> {
    try {
      return await invoke<AcpRegistryAgent[]>("acp_agent_fetch_registry");
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_agent_fetch_registry failed", e);
      throw e;
    }
  }

  // ── Connection management ───────────────────────────────────────────────────
  async connect(agentRowId: string): Promise<string> {
    this.error = null;
    try {
      const connectionId = await invoke<string>("acp_connect", { agentRowId });
      this.connections = { ...this.connections, [connectionId]: "initializing" };
      return connectionId;
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_connect failed", e);
      throw e;
    }
  }

  async disconnect(connectionId: string) {
    this.error = null;
    try {
      await invoke("acp_disconnect", { connectionId });
      const { [connectionId]: _conn, ...rest } = this.connections;
      this.connections = rest;
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_disconnect failed", e);
      throw e;
    }
  }

  // ── Session management ──────────────────────────────────────────────────────
  async newSession(
    connectionId: string,
    cwd: string,
    mcpServers?: unknown
  ): Promise<string> {
    try {
      return await invoke<string>("acp_new_session", {
        connectionId,
        cwd,
        mcpServers,
      });
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_new_session failed", e);
      throw e;
    }
  }

  async sendPrompt(connectionId: string, sessionId: string, text: string) {
    try {
      await invoke("acp_send_prompt", { connectionId, sessionId, text });
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_send_prompt failed", e);
      throw e;
    }
  }

  async cancel(connectionId: string, sessionId: string) {
    try {
      await invoke("acp_cancel", { connectionId, sessionId });
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_cancel failed", e);
      throw e;
    }
  }

  async closeSession(connectionId: string, sessionId: string) {
    try {
      await invoke("acp_close_session", { connectionId, sessionId });
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_close_session failed", e);
      throw e;
    }
  }

  async respondPermission(
    connectionId: string,
    requestId: string,
    optionId?: string
  ) {
    try {
      await invoke("acp_respond_permission", { connectionId, requestId, optionId });
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_respond_permission failed", e);
      throw e;
    }
  }

  async setMode(connectionId: string, sessionId: string, mode: string) {
    try {
      await invoke("acp_set_mode", { connectionId, sessionId, mode });
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_set_mode failed", e);
      throw e;
    }
  }

  async logout(connectionId: string) {
    try {
      await invoke("acp_logout", { connectionId });
    } catch (e) {
      this.error = String(e);
      console.error("[AcpStore] acp_logout failed", e);
      throw e;
    }
  }

  // ── Log utilities ───────────────────────────────────────────────────────────
  clearLogs(connectionId: string) {
    this.logs = { ...this.logs, [connectionId]: [] };
  }
}

export const acpStore = new AcpStore();
