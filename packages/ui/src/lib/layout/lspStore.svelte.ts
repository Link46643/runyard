import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { LspServerStatus, LspStatusKind } from "@runyard/common";

type LspMessageHandler = (language: string, message: unknown) => void;

class LspStore {
  statuses = $state<Record<string, LspServerStatus>>({});
  private messageHandlers: LspMessageHandler[] = [];
  private unlistenFn: (() => void) | null = null;

  async init() {
    // Listen for LSP messages from Rust
    this.unlistenFn = await listen<{ language: string; message: unknown }>(
      "lsp:message",
      (event) => {
        const { language, message } = event.payload;
        this.messageHandlers.forEach((h) => h(language, message));
      }
    );
  }

  destroy() {
    this.unlistenFn?.();
  }

  onMessage(handler: LspMessageHandler) {
    this.messageHandlers.push(handler);
    return () => {
      this.messageHandlers = this.messageHandlers.filter((h) => h !== handler);
    };
  }

  async start(language: string, workspacePath: string, pathOverride?: string) {
    try {
      const status = await invoke<LspServerStatus>("lsp_start", {
        language,
        workspacePath,
        pathOverride: pathOverride ?? null,
      });
      this.statuses = { ...this.statuses, [language]: status };
      return status;
    } catch (e) {
      const err: LspServerStatus = {
        language,
        status: "error" as LspStatusKind,
        error: String(e),
        executable: null,
      };
      this.statuses = { ...this.statuses, [language]: err };
      return err;
    }
  }

  async stop(language: string) {
    try {
      await invoke("lsp_stop", { language });
    } catch (_) {}
    const { [language]: _, ...rest } = this.statuses;
    this.statuses = rest;
  }

  async send(language: string, message: unknown) {
    const msg =
      typeof message === "string" ? message : JSON.stringify(message);
    try {
      await invoke("lsp_send", { language, message: msg });
    } catch (e) {
      console.warn(`[LSP] Failed to send message to ${language}:`, e);
    }
  }

  getStatus(language: string): LspStatusKind {
    return this.statuses[language]?.status ?? "disconnected";
  }

  getActiveLangs(): string[] {
    return Object.keys(this.statuses).filter(
      (l) =>
        this.statuses[l].status === "ready" ||
        this.statuses[l].status === "starting"
    );
  }
}

export const lspStore = new LspStore();
