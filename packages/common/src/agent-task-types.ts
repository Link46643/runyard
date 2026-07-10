// Agent task tracking types (engineering-todo-v2.md 1.10.1).

export type AgentTaskStatus =
  | "queued"
  | "running"
  | "awaiting_hil"
  | "completed"
  | "failed"
  | "cancelled";

export interface AgentTask {
  id: string;
  agent_id: string;
  agent_name: string;
  connection_id: string | null;
  session_id: string | null;
  conversation_id: string | null;
  project: string | null;
  description: string;
  status: AgentTaskStatus;
  created_at: number;
  updated_at: number;
  completed_at: number | null;
  cost_usd: number;
  current_tool: string | null;
  error: string | null;
}

export interface AgentTaskStats {
  activeTasks: number;
  totalTasks: number;
  totalCostUsd: number;
  hilPending: number;
}
