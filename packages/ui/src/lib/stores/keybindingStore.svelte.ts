// Keybinding system (engineering-todo-v2.md 1.13.1).
// Stores per-command key bindings in localStorage, provides platform-aware
// defaults (⌘ on Mac, Ctrl elsewhere), and supports import from VS Code's
// keybindings.json format.

const STORAGE_KEY = "runyard:keybindings";

// VS Code command → Runyard command ID mapping for import.
const VS_CODE_MAP: Record<string, string> = {
  "workbench.action.showCommands":         "commands.open",
  "workbench.action.files.openFolder":     "workspace.open",
  "workbench.action.quickOpen":            "file.quickOpen",
  "workbench.action.terminal.new":         "terminal.new",
  "git.openRepository":                   "git.open",
  "workbench.action.splitEditor":          "view.splitHorizontal",
  "workbench.action.openSettings":         "settings.open",
  "workbench.action.findInFiles":          "commands.search",
  "workbench.action.newWindow":            "workspace.open",
  "workbench.action.closeActiveEditor":    "tab.close",
  "workbench.view.explorer":              "explorer.focus",
  "workbench.view.scm":                   "git.open",
};

// Platform detection — must run in the browser (never SSR).
function isMac(): boolean {
  try { return navigator.platform.toLowerCase().includes("mac"); } catch { return false; }
}

// Platform-aware default key bindings.
function platformDefault(cmdId: string): string | undefined {
  const mac = isMac();
  const mod = mac ? "Cmd" : "Ctrl";
  const defaults: Record<string, string> = {
    "commands.open":        `${mod}+Shift+P`,
    "file.quickOpen":       `${mod}+P`,
    "terminal.new":         `${mod}+\``,
    "settings.open":        `${mod}+,`,
    "git.open":             "",
    "chat.open":            "",
    "agent.manager":        "",
    "workspace.open":       "",
    "workspace.switcher":   `${mod}+Shift+W`,
    "view.splitHorizontal": `${mod}+\\`,
    "view.toggleTheme":     "",
  };
  return defaults[cmdId];
}

class KeybindingStore {
  bindings = $state<Record<string, string>>({});

  constructor() {
    this._load();
  }

  private _load() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) this.bindings = JSON.parse(raw);
    } catch { /* ignore */ }
  }

  private _save() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(this.bindings));
    } catch { /* ignore */ }
  }

  /** Get the binding for a command (custom override or platform default). */
  getBinding(cmdId: string): string | undefined {
    return this.bindings[cmdId] ?? platformDefault(cmdId);
  }

  /** Set a custom binding for a command. */
  set(cmdId: string, binding: string) {
    this.bindings = { ...this.bindings, [cmdId]: binding };
    this._save();
  }

  /** Remove a custom binding, falling back to platform default. */
  reset(cmdId: string) {
    const { [cmdId]: _, ...rest } = this.bindings;
    this.bindings = rest;
    this._save();
  }

  /** Format a binding string for display with platform symbols. */
  format(binding: string): string {
    if (!binding) return "";
    if (isMac()) {
      return binding
        .replace(/Cmd\+?/g, "⌘")
        .replace(/Ctrl\+?/g, "⌃")
        .replace(/Alt\+?/g, "⌥")
        .replace(/Shift\+?/g, "⇧");
    }
    return binding;
  }

  /**
   * Import keybindings from VS Code's keybindings.json format.
   * Parses [{key, command, when?}] and maps known commands to Runyard IDs.
   */
  importFromVsCode(json: string): { imported: number; skipped: number } {
    let imported = 0;
    let skipped = 0;
    try {
      const entries = JSON.parse(json) as Array<{ key: string; command: string }>;
      for (const entry of entries) {
        const runyardId = VS_CODE_MAP[entry.command];
        if (runyardId && entry.key) {
          // Normalize VS Code key format: "ctrl+shift+p" → "Ctrl+Shift+P"
          const normalized = entry.key
            .split("+")
            .map((k) => k.charAt(0).toUpperCase() + k.slice(1).toLowerCase())
            .join("+")
            .replace("Meta", "Cmd");
          this.set(runyardId, normalized);
          imported++;
        } else {
          skipped++;
        }
      }
    } catch {
      throw new Error("Invalid VS Code keybindings.json format");
    }
    return { imported, skipped };
  }

  /** All command IDs with either a custom or default binding. */
  get allBindings(): Array<{ cmdId: string; binding: string; isCustom: boolean }> {
    const customIds = Object.keys(this.bindings);
    const defaultIds = Object.keys({
      "commands.open": "", "file.quickOpen": "", "terminal.new": "",
      "settings.open": "", "git.open": "", "chat.open": "",
      "agent.manager": "", "workspace.open": "", "workspace.switcher": "",
      "view.splitHorizontal": "",
    });
    const allIds = [...new Set([...customIds, ...defaultIds])];
    return allIds.map((id) => ({
      cmdId: id,
      binding: this.getBinding(id) ?? "",
      isCustom: id in this.bindings,
    })).filter((b) => b.binding);
  }
}

export const keybindingStore = new KeybindingStore();
