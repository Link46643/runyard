// Workspace management types
export interface RecentWorkspace {
  path: string;
  name: string;
  last_opened_at: number;
}

// Sandbox types
export interface SandboxConfig {
  agentId: string;
  allowedRoots: string;
  maxFileBytes: number;
  toolTimeoutSecs: number;
  allowNetwork: boolean;
}

export interface AuditLogEntry {
  id: string;
  agentId: string;
  connectionId: string | null;
  sessionId: string | null;
  tool: string;
  argsJson: string;
  outcome: "allowed" | "denied" | "error";
  deniedReason: string | null;
  createdAt: number;
}
