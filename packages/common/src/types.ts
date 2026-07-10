// ─── Layout types ──────────────────────────────────────────────────────────────

export type WorkspaceId = string;

export type TabType =
  | "editor"
  | "explorer"
  | "terminal"
  | "chat"
  | "git"
  | "settings"
  | "agent-manager"
  | "orchestrator"
  | "welcome";

export interface Tab {
  id: string;
  type: TabType;
  title: string;
  icon?: string;
  dirty?: boolean;
  props: Record<string, unknown>;
}

export interface SplitNode {
  type: "split";
  id: string;
  direction: "horizontal" | "vertical";
  children: LayoutNode[];
  sizes: number[]; // Percentages (e.g., [50, 50])
}

export interface LeafNode {
  type: "leaf";
  id: string;
  tabs: Tab[];
  activeTabId: string | null;
}

export type LayoutNode = SplitNode | LeafNode;

export interface Layout {
  root: LayoutNode;
}

// ─── Filesystem types ──────────────────────────────────────────────────────────

export interface FsEntry {
  name: string;
  path: string;
  kind: "file" | "dir";
  size: number;
}

// ─── Terminal types ────────────────────────────────────────────────────────────

export interface TerminalSessionInfo {
  id: string;
  cwd: string;
  cols: number;
  rows: number;
}

// ─── Git types ─────────────────────────────────────────────────────────────────

export interface GitFileEntry {
  path: string;
  status: "modified" | "added" | "deleted" | "renamed" | "untracked";
}

export interface GitStatus {
  branch: string | null;
  changed: GitFileEntry[];
  staged: GitFileEntry[];
  untracked: GitFileEntry[];
  ahead: number;
  behind: number;
}

export interface GitCommit {
  hash: string;
  short_hash: string;
  message: string;
  author: string;
  timestamp: number;
}

export interface GitBranch {
  name: string;
  is_current: boolean;
  is_remote: boolean;
}

export interface GitWorktree {
  name: string;
  path: string;
  branch: string | null;
  is_main: boolean;
}

// ─── Settings types ────────────────────────────────────────────────────────────

export interface EditorSettings {
  font_size: number;
  tab_size: number;
  line_wrap: boolean;
  format_on_save: boolean;
  vim_mode: boolean;
}

export interface TerminalSettings {
  default_shell: string | null;
  font_size: number;
  scrollback_limit: number;
}

export interface AppearanceSettings {
  theme: "dark" | "light";
  font_family: string;
}

export interface LspLanguageConfig {
  enabled: boolean;
  path_override: string | null;
}

export interface LspSettings {
  typescript: LspLanguageConfig;
  python: LspLanguageConfig;
  rust: LspLanguageConfig;
  go: LspLanguageConfig;
}

export interface RunyardSettings {
  editor: EditorSettings;
  terminal: TerminalSettings;
  appearance: AppearanceSettings;
  lsp: LspSettings;
}

export const DEFAULT_SETTINGS: RunyardSettings = {
  editor: {
    font_size: 14,
    tab_size: 2,
    line_wrap: false,
    format_on_save: false,
    vim_mode: false,
  },
  terminal: {
    default_shell: null,
    font_size: 13,
    scrollback_limit: 5000,
  },
  appearance: {
    theme: "dark",
    font_family: "JetBrains Mono",
  },
  lsp: {
    typescript: { enabled: true, path_override: null },
    python: { enabled: true, path_override: null },
    rust: { enabled: true, path_override: null },
    go: { enabled: true, path_override: null },
  },
};

// ─── LSP types ─────────────────────────────────────────────────────────────────

export type LspStatusKind = "disconnected" | "starting" | "ready" | "error";

export interface LspServerStatus {
  language: string;
  status: LspStatusKind;
  error: string | null;
  executable: string | null;
}

// ─── Command palette types ─────────────────────────────────────────────────────

export interface Command {
  id: string;
  title: string;
  subtitle?: string;
  category: string;
  shortcut?: string;
  handler: () => void | Promise<void>;
}
