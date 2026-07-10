// MCP server config types (engineering-todo-v2.md 1.8.1).
// Mirrors crates/runyard-core/src/mcp_server_db.rs exactly.

export type McpTransport = "stdio" | "http" | "websocket";

export interface McpEnvVar {
  key: string;
  value: string;
  is_secret: boolean;
}

export interface McpAuth {
  kind: "none" | "bearer" | "api_key";
  token?: string | null;
}

export interface McpServerConfig {
  id: string;
  name: string;
  transport: McpTransport;
  command: string | null;
  url: string | null;
  env_vars: McpEnvVar[];
  auth: McpAuth | null;
  is_global: boolean;
  is_active: boolean;
  project_id: string | null;
  created_at: number;
  updated_at: number;
}
