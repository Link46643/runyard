<script lang="ts">
  import type { Tab } from "@runyard/common";
  import ExplorerPanel from "./ExplorerPanel.svelte";
  import { layoutEngine } from "./layoutStore.svelte.js";
  import { workspaceStore } from "../stores/workspaceStore.svelte.js";

  let { tab } = $props<{ tab: Tab }>();

  function handleOpenFile(path: string, name: string) {
    layoutEngine.openEditor(path, name);
  }
</script>

<div class="explorer-tab">
  <!-- Use workspaceStore.currentPath as the root; fall back to tab prop then "../../" -->
  <ExplorerPanel
    workspacePath={workspaceStore.currentPath || (tab.props.workspacePath as string) || "../../"}
    onOpenFile={handleOpenFile}
  />
</div>

<style>
  .explorer-tab { 
    width: 100%; 
    height: 100%; 
    overflow: hidden; 
  }
</style>
