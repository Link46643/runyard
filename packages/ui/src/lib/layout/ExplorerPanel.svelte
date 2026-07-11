<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { FsEntry } from "@runyard/common";
  import TreeNode from "./TreeNode.svelte";
  import { FilePlus, FolderPlus } from "lucide-svelte";
  import { explorerStore } from "./explorerStore.svelte.js";

  import { webSocketClient } from "@runyard/common";

  let { workspacePath, onOpenFile } = $props<{ 
    workspacePath: string, 
    onOpenFile: (path: string, name: string) => void 
  }>();

  let rootNodes = $state<FsEntry[]>([]);
  let childComponents: ReturnType<typeof TreeNode>[] = [];

  async function createNewFile() {
    const filename = prompt("Enter new file name:");
    if (!filename || !filename.trim()) return;

    let targetDir = workspacePath;
    if (explorerStore.selectedPath) {
      if (explorerStore.selectedKind === "dir") {
        targetDir = explorerStore.selectedPath;
      } else {
        const path = explorerStore.selectedPath.replace(/\\/g, "/");
        targetDir = path.substring(0, path.lastIndexOf("/")) || workspacePath;
      }
    }

    const separator = targetDir.includes("/") ? "/" : "\\";
    const newFilePath = targetDir.endsWith(separator) 
      ? targetDir + filename.trim() 
      : targetDir + separator + filename.trim();

    try {
      if (webSocketClient.status === "connected") {
        await webSocketClient.invoke("fs_write", { path: newFilePath, contents: "" });
      } else {
        await invoke("fs_write", { path: newFilePath, contents: "" });
      }
      
      if (explorerStore.selectedPath && explorerStore.selectedKind === "dir") {
        explorerStore.toggle(targetDir, true);
      }

      await loadRoot();
      onOpenFile(newFilePath, filename.trim());
    } catch (e) {
      alert("Failed to create file: " + e);
    }
  }

  async function createNewFolder() {
    const foldername = prompt("Enter new folder name:");
    if (!foldername || !foldername.trim()) return;

    let targetDir = workspacePath;
    if (explorerStore.selectedPath) {
      if (explorerStore.selectedKind === "dir") {
        targetDir = explorerStore.selectedPath;
      } else {
        const path = explorerStore.selectedPath.replace(/\\/g, "/");
        targetDir = path.substring(0, path.lastIndexOf("/")) || workspacePath;
      }
    }

    const separator = targetDir.includes("/") ? "/" : "\\";
    const newFolderPath = targetDir.endsWith(separator) 
      ? targetDir + foldername.trim() 
      : targetDir + separator + foldername.trim();

    try {
      if (webSocketClient.status === "connected") {
        await webSocketClient.invoke("fs_create_dir", { path: newFolderPath });
      } else {
        await invoke("fs_create_dir", { path: newFolderPath });
      }

      if (explorerStore.selectedPath && explorerStore.selectedKind === "dir") {
        explorerStore.toggle(targetDir, true);
      }

      await loadRoot();
    } catch (e) {
      alert("Failed to create folder: " + e);
    }
  }

  let folderName = $derived(
    (() => {
      const p = workspacePath.replace(/[/\\]$/, "");
      return p.split(/[/\\]/).filter(Boolean).pop() || p || "";
    })()
  );

  async function loadRoot() {
    try {
      let res: FsEntry[];
      if (webSocketClient.status === "connected") {
        res = await webSocketClient.invoke<FsEntry[]>("fs_list", { path: workspacePath });
      } else {
        res = await invoke<FsEntry[]>("fs_list", { path: workspacePath });
      }
      res.sort((a, b) => {
        if (a.kind === b.kind) return a.name.localeCompare(b.name);
        return a.kind === "dir" ? -1 : 1;
      });
      rootNodes = res;
    } catch(e) {
      console.error("Failed to load root", e);
    }
  }

  // Reactively reload files when the workspace path changes
  $effect(() => {
    loadRoot();

    // Start watching the new workspace root
    if (webSocketClient.status === "connected") {
      webSocketClient.invoke("fs_watch", { path: workspacePath }).catch(console.error);
    } else {
      invoke("fs_watch", { path: workspacePath }).catch(console.error);
    }
  });

  onMount(() => {
    // Setup fs:changed listener
    const unlisten = listen<string>("fs:changed", (e) => {
      loadRoot();
    });

    return () => {
      unlisten.then(f => f());
    };
  });
</script>

<div class="explorer-panel">
  <div class="header">
    <span class="title">
      EXPLORER
      {#if folderName}
        <span class="folder-name">{folderName}</span>
      {/if}
    </span>
    <div class="action-buttons">
      <button class="action-btn" onclick={createNewFile} title="New File...">
        <FilePlus size={14} strokeWidth={1.5} />
      </button>
      <button class="action-btn" onclick={createNewFolder} title="New Folder...">
        <FolderPlus size={14} strokeWidth={1.5} />
      </button>
    </div>
  </div>
  <div class="tree">
    {#each rootNodes as node (node.path)}
      <TreeNode {node} {onOpenFile} />
    {/each}
  </div>
</div>

<style>
  .explorer-panel {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background-color: var(--sidebar-bg);
    overflow: hidden;
    border-right: 1px solid var(--border);
  }
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 11px;
    font-weight: 700;
    color: var(--text-secondary);
    padding: 10px 16px;
    text-transform: uppercase;
    letter-spacing: 1px;
    flex-shrink: 0;
    user-select: none;
    border-bottom: 1px solid var(--border);
  }
  .title {
    display: flex;
    align-items: center;
  }
  .folder-name {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-left: 6px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    padding: 1px 6px;
    border-radius: 4px;
    text-transform: none;
    letter-spacing: 0;
  }
  .action-buttons {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    padding: 4px;
    border-radius: 4px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: background-color 0.2s, color 0.2s;
  }
  .action-btn:hover {
    background-color: var(--bg-secondary);
    color: var(--text);
  }
  .tree {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding-top: 4px;
  }
</style>
