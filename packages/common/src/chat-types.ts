export interface Conversation {
  id: string;
  title: string;
  workspace_path: string;
  model: string;
  provider: string;
  system_prompt?: string;
  context_budget: number;
  created_at: number;
  updated_at: number;
  message_count: number;
  total_tokens_used: number;
  total_cost: number;
}

export type MessageRole = "user" | "assistant" | "system";

export type ContentBlockType =
  | "text"
  | "code"
  | "diff"
  | "tool_call"
  | "tool_result"
  | "thinking"
  | "permission_request"
  | "file_ref"
  | "plan"
  | "context_summary"
  | "error";

export interface BaseContentBlock {
  type: ContentBlockType;
}

export interface TextBlock extends BaseContentBlock {
  type: "text";
  text: string;
}

export interface CodeBlock extends BaseContentBlock {
  type: "code";
  language: string;
  code: string;
  filename?: string;
}

export interface DiffBlock extends BaseContentBlock {
  type: "diff";
  filepath: string;
  diff: string;
}

export interface ToolCallBlock extends BaseContentBlock {
  type: "tool_call";
  tool_id: string;
  name: string;
  arguments: Record<string, any>;
}

export interface ToolResultBlock extends BaseContentBlock {
  type: "tool_result";
  tool_id: string;
  output: string;
  is_error: boolean;
}

export interface ThinkingBlock extends BaseContentBlock {
  type: "thinking";
  thought: string;
  token_count?: number;
}

export interface PermissionBlock extends BaseContentBlock {
  type: "permission_request";
  tool_id: string;
  action: string;
  approved: boolean | null;
}

export interface FileRefBlock extends BaseContentBlock {
  type: "file_ref";
  filepath: string;
}

export interface PlanStep {
  id: string;
  description: string;
  status: "pending" | "running" | "completed" | "failed";
}

export interface PlanBlock extends BaseContentBlock {
  type: "plan";
  steps: PlanStep[];
}

export interface ContextSummaryBlock extends BaseContentBlock {
  type: "context_summary";
  summary: string;
  original_count: number;
}

export interface ErrorBlock extends BaseContentBlock {
  type: "error";
  code: string;
  message: string;
}

export type ContentBlock =
  | TextBlock
  | CodeBlock
  | DiffBlock
  | ToolCallBlock
  | ToolResultBlock
  | ThinkingBlock
  | PermissionBlock
  | FileRefBlock
  | PlanBlock
  | ContextSummaryBlock
  | ErrorBlock;

export interface Message {
  id: string;
  conversation_id: string;
  parent_id: string | null;
  role: MessageRole;
  content: ContentBlock[];
  created_at: number;
}

export interface Branch {
  id: string;
  conversation_id: string;
  name: string;
  message_id: string;
  created_at: number;
}

export interface PinnedContext {
  id: string;
  conversation_id: string;
  file_path: string;
  created_at: number;
}
