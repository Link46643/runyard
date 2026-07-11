// ACP Agent Discovery & Integration types (engineering-todo-v2.md 1.6.1).
// Mirrors crates/runyard-core/src/acp_agent_db.rs's DbAcpAgent /
// DbAcpAgentEnvVar / AcpAgentExport structs and crates/runyard-acp's
// DiscoveredAgent / AcpEvent shapes field-for-field - snake_case throughout
// since these cross the Tauri IPC boundary as plain JSON, same convention as
// chat-types.ts.

export type AcpTransportKind = "stdio" | "http" | "websocket";

export type AcpAgentDiscoverySource = "builtin" | "path_scan" | "registry" | "manual";

/// Mirrors runyard_acp::ConnectionStatus, but also covers the extra values
/// AcpEvent::Error / a fresh unconnected row can produce - see
/// acp_agent_db.rs's `status` column, which stores these as plain strings.
export type AcpAgentStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "ready"
  | "processing"
  | "error";

export interface AcpAgentEnvVar {
  key: string;
  /** Empty string for secret entries that came from an import/export round trip - see AcpAgentExport. */
  value: string;
  is_secret: boolean;
}

export interface AcpAgentConfig {
  id: string;
  name: string;
  agent_id: string;
  executable_path: string | null;
  spawn_command: string | null;
  remote_url: string | null;
  transport: AcpTransportKind;
  env_vars: AcpAgentEnvVar[];
  capabilities: Record<string, unknown> | null;
  discovery_source: AcpAgentDiscoverySource;
  status: AcpAgentStatus;
  last_error: string | null;
  is_builtin: boolean;
  is_active: boolean;
  is_default: boolean;
  created_at: number;
  updated_at: number;
}

/** Portable subset used by acp_agent_export / acp_agent_import - never carries secret env var values. */
export interface AcpAgentExport {
  name: string;
  agent_id: string;
  executable_path: string | null;
  spawn_command: string | null;
  remote_url: string | null;
  transport: AcpTransportKind;
  env_vars: AcpAgentEnvVar[];
}

/** One agent found on the local machine via PATH scanning (runyard_acp::discovery). */
export interface DiscoveredAcpAgent {
  agent_id: string;
  name: string;
  executable_path: string;
  has_config_dir: boolean;
  /** Ready-to-use spawn command (path + any required ACP subcommand/args,
   * shell-quoted). Always use this for spawn_command, never
   * executable_path alone - some agents (e.g. OpenCode) require an
   * explicit subcommand to start their ACP server rather than their
   * default interactive mode. */
  recommended_spawn_command: string;
}

/** One entry from the live ACP Registry API (cdn.agentclientprotocol.com). */
export interface AcpRegistryAgent {
  id: string;
  name: string;
  version: string;
  description: string;
  repository?: string | null;
  authors?: string[];
  license?: string | null;
  icon?: string | null;
}

// ── Capabilities display (1.6.7) ────────────────────────────────────────────

/** One capability badge, derived client-side from an AcpAgentConfig's raw `capabilities` blob (the agent's InitializeResponse.agentCapabilities). */
export interface AcpAgentCapabilityBadge {
  key: string;
  label: string;
  supported: boolean;
  detail?: string;
}

// ── Live connection events (mirrors runyard_acp::AcpEvent 1:1) ─────────────

export type AcpConnectionStatusValue = "idle" | "initializing" | "ready" | "processing" | "error" | "disconnected";

export interface AcpPermissionOption {
  option_id: string;
  label: string;
}

export type AcpLogDirection = "stdin" | "stdout" | "stderr";

/**
 * Tagged union matching runyard_acp::events::AcpEvent's serde shape
 * (`#[serde(tag = "type", rename_all = "snake_case")]`) exactly - this is
 * what arrives on every Tauri `"acp:event"` listener callback.
 */
export type AcpClientEvent =
  | { type: "connected"; connection_id: string; agent_name: string | null; agent_capabilities: Record<string, unknown>; auth_methods: string[] }
  | { type: "disconnected"; connection_id: string; reason: string }
  | { type: "session_started"; connection_id: string; session_id: string }
  | { type: "session_closed"; connection_id: string; session_id: string }
  | { type: "agent_message_chunk"; connection_id: string; session_id: string; text: string }
  | { type: "user_message_chunk"; connection_id: string; session_id: string; text: string }
  | { type: "thought_chunk"; connection_id: string; session_id: string; text: string }
  | { type: "tool_call"; connection_id: string; session_id: string; tool_call_id: string; name: string; arguments: unknown }
  | { type: "tool_call_update"; connection_id: string; session_id: string; tool_call_id: string; status: string; content: unknown }
  | { type: "plan_update"; connection_id: string; session_id: string; plan: unknown }
  | { type: "available_commands_update"; connection_id: string; session_id: string; commands: unknown }
  | { type: "session_info_update"; connection_id: string; session_id: string; info: unknown }
  | {
      type: "permission_requested";
      connection_id: string;
      session_id: string;
      request_id: string;
      tool_name: string;
      arguments: unknown;
      options: AcpPermissionOption[];
    }
  | { type: "prompt_completed"; connection_id: string; session_id: string; stop_reason: string }
  | { type: "error"; connection_id: string; session_id: string | null; code: string; message: string; recoverable: boolean }
  | { type: "status_changed"; connection_id: string; status: AcpConnectionStatusValue }
  | { type: "log_line"; connection_id: string; direction: AcpLogDirection; line: string };
