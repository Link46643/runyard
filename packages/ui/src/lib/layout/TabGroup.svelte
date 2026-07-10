<script lang="ts">
  import type { LeafNode } from "@runyard/common";
  import TabBar from "./TabBar.svelte";
  import TabContent from "./TabContent.svelte";

  let { node } = $props<{ node: LeafNode }>();
</script>

<div class="tab-group">
  {#if node.tabs.length > 0}
    <TabBar tabs={node.tabs} activeTabId={node.activeTabId} leafId={node.id} />
    <div class="content-container">
      <!--
        Render ALL tabs and hide inactive ones via display:none.
        This keeps terminal PTY sessions and editor state alive across tab switches.
        CodeMirror's ResizeObserver will re-measure automatically when revealed.
      -->
      {#each node.tabs as tab (tab.id)}
        <div
          class="tab-slot"
          style:display={tab.id === node.activeTabId ? "flex" : "none"}
        >
          <TabContent {tab} />
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty-state">
      <p>No active tabs</p>
    </div>
  {/if}
</div>

<style>
  .tab-group {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    background: var(--bg, #000);
    color: var(--text, #e5e7eb);
    overflow: hidden;
  }

  .content-container {
    flex: 1;
    min-height: 0;
    position: relative;
  }

  .tab-slot {
    position: absolute;
    inset: 0;
    flex-direction: column;
    overflow: hidden;
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    color: var(--text-secondary, #6b7280);
    font-family: inherit;
    font-size: 13px;
  }
</style>
