<script lang="ts">
  import type { Tab } from "@runyard/common";
  import ExplorerPanel from "./ExplorerPanel.svelte";
  import { layoutEngine } from "./layoutStore.svelte.js";

  let { tab } = $props<{ tab: Tab }>();

  function handleOpenFile(path: string, name: string) {
    layoutEngine.openEditor(path, name);
  }
</script>

<div class="explorer-tab">
  <!-- Use a default workspacePath for now, e.g., the root of the project -->
  <ExplorerPanel 
    workspacePath={tab.props.workspacePath as string || "../../"} 
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
