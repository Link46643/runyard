// Notes and TODOs types (engineering-todo-v2.md 1.11.1-1.11.2).

export interface Note {
  id: string;
  workspace_path: string;
  content: string;
  created_at: number;
  updated_at: number;
}

export interface Todo {
  id: string;
  workspace_path: string;
  text: string;
  is_done: boolean;
  sort_order: number;
  created_at: number;
  updated_at: number;
}
