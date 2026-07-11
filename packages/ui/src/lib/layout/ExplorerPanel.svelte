<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { FsEntry } from "@runyard/common";
  import TreeNode from "./TreeNode.svelte";

  import { webSocketClient } from "@runyard/common";

  let { workspacePath, onOpenFile } = $props<{ 
    workspacePath: string, 
    onOpenFile: (path: string, name: string) => void 
  }>();

  let rootNodes = $state<FsEntry[]>([]);
  let childComponents: ReturnType<typeof TreeNode>[] = [];

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
    EXPLORER
    {#if folderName}
      <span class="folder-name">{folderName}</span>
    {/if}
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
    font-size: 11px;
    font-weight: 700;
    color: var(--text-secondary);
    padding: 12px 16px;
    text-transform: uppercase;
    letter-spacing: 1px;
    flex-shrink: 0;
    user-select: none;
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
  .tree {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding-top: 4px;
  }
</style>
