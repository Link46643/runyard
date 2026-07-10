<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { FsEntry } from "@runyard/common";
  import TreeNode from "./TreeNode.svelte";

  let { workspacePath, onOpenFile } = $props<{ 
    workspacePath: string, 
    onOpenFile: (path: string, name: string) => void 
  }>();

  let rootNodes = $state<FsEntry[]>([]);
  let childComponents: ReturnType<typeof TreeNode>[] = [];

  async function loadRoot() {
    try {
      let res = await invoke<FsEntry[]>("fs_list", { path: workspacePath });
      res.sort((a, b) => {
        if (a.kind === b.kind) return a.name.localeCompare(b.name);
        return a.kind === "dir" ? -1 : 1;
      });
      rootNodes = res;
    } catch(e) {
      console.error("Failed to load root", e);
    }
  }

  onMount(() => {
    loadRoot();
    
    // Setup fs:changed listener
    const unlisten = listen<string>("fs:changed", (e) => {
      // In a robust implementation, we would recursively find the exact node.
      // For Milestone 1, we re-fetch the root.
      // Or we can just call loadRoot() and let components handle updates if needed.
      loadRoot();
    });

    // Start watching the workspace root
    invoke("fs_watch", { path: workspacePath }).catch(console.error);

    return () => {
      unlisten.then(f => f());
    };
  });
</script>

<div class="explorer-panel">
  <div class="header">EXPLORER</div>
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
    font-size: 11px;
    font-weight: 700;
    color: var(--text-secondary);
    padding: 12px 16px;
    text-transform: uppercase;
    letter-spacing: 1px;
    flex-shrink: 0;
    user-select: none;
  }
  .tree {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding-top: 4px;
  }
</style>
