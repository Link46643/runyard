<script lang="ts">
  import type { Tab } from "@runyard/common";
  import { layoutEngine } from "./layoutStore.svelte.js";
  import EditorPanel from "./EditorPanel.svelte";

  let { tab } = $props<{ tab: Tab }>();

  function handleDirtyChange(dirty: boolean) {
    layoutEngine.setTabDirty(tab.id, dirty);
  }
</script>

<div class="editor-tab">
  <EditorPanel 
    filePath={tab.props.filePath as string} 
    onDirtyChange={handleDirtyChange} 
  />
</div>

<style>
  .editor-tab { 
    width: 100%; 
    height: 100%; 
    overflow: hidden; 
  }
</style>
