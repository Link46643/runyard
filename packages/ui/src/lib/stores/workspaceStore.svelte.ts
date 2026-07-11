import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { webSocketClient } from "@runyard/common";

async function invoke<T>(cmd: string, args?: any): Promise<T> {
  if (webSocketClient.status === "connected") {
    return webSocketClient.invoke<T>(cmd, args);
  } else {
    return tauriInvoke<T>(cmd, args);
  }
}

export interface RecentWorkspace {
  path: string;
  name: string;
  last_opened_at: number;
}

class WorkspaceStore {
  currentPath = $state("../../");
  currentName = $derived(
    (() => {
      const p = this.currentPath.replace(/\/$/, "");
      return p.split("/").filter(Boolean).pop() ?? p;
    })()
  );
  recentWorkspaces = $state<RecentWorkspace[]>([]);
  isLoading = $state(false);

  constructor() {
    this.loadRecent();
  }

  async open(path: string) {
    this.currentPath = path;
    try {
      await invoke("workspace_open", { path });
    } catch (e) {
      console.error("[WorkspaceStore] Failed to record workspace open", e);
    }
    await this.loadRecent();
  }

  async openViaDialog() {
    try {
      // Dynamic import to avoid issues in non-Tauri/web environments
      const { open } = await import("@tauri-apps/plugin-dialog");
      const result = await open({ directory: true, multiple: false });
      if (result && typeof result === "string") {
        await this.open(result);
      }
    } catch (e) {
      console.error("[WorkspaceStore] Failed to open folder dialog", e);
    }
  }

  async loadRecent() {
    try {
      const list = await invoke<RecentWorkspace[]>("workspace_list_recent");
      this.recentWorkspaces = list;
    } catch (e) {
      console.error("[WorkspaceStore] Failed to load recent workspaces", e);
    }
  }

  async removeRecent(path: string) {
    try {
      await invoke("workspace_remove_recent", { path });
    } catch (e) {
      console.error("[WorkspaceStore] Failed to remove recent workspace", e);
    }
    await this.loadRecent();
  }

  resolve(relative: string): string {
    return this.currentPath.endsWith("/")
      ? this.currentPath + relative
      : this.currentPath + "/" + relative;
  }
}

export const workspaceStore = new WorkspaceStore();
