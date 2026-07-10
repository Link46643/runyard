import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
class LspStore {
    statuses = $state({});
    messageHandlers = [];
    unlistenFn = null;
    async init() {
        // Listen for LSP messages from Rust
        this.unlistenFn = await listen("lsp:message", (event) => {
            const { language, message } = event.payload;
            this.messageHandlers.forEach((h) => h(language, message));
        });
    }
    destroy() {
        this.unlistenFn?.();
    }
    onMessage(handler) {
        this.messageHandlers.push(handler);
        return () => {
            this.messageHandlers = this.messageHandlers.filter((h) => h !== handler);
        };
    }
    async start(language, workspacePath, pathOverride) {
        try {
            const status = await invoke("lsp_start", {
                language,
                workspacePath,
                pathOverride: pathOverride ?? null,
            });
            this.statuses = { ...this.statuses, [language]: status };
            return status;
        }
        catch (e) {
            const err = {
                language,
                status: "error",
                error: String(e),
                executable: null,
            };
            this.statuses = { ...this.statuses, [language]: err };
            return err;
        }
    }
    async stop(language) {
        try {
            await invoke("lsp_stop", { language });
        }
        catch (_) { }
        const { [language]: _, ...rest } = this.statuses;
        this.statuses = rest;
    }
    async send(language, message) {
        const msg = typeof message === "string" ? message : JSON.stringify(message);
        try {
            await invoke("lsp_send", { language, message: msg });
        }
        catch (e) {
            console.warn(`[LSP] Failed to send message to ${language}:`, e);
        }
    }
    getStatus(language) {
        return this.statuses[language]?.status ?? "disconnected";
    }
    getActiveLangs() {
        return Object.keys(this.statuses).filter((l) => this.statuses[l].status === "ready" ||
            this.statuses[l].status === "starting");
    }
}
export const lspStore = new LspStore();
