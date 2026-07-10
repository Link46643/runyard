<script lang="ts">
  import type { SplitNode } from "@runyard/common";
  import LayoutRenderer from "./Layout.svelte";
  import { platform } from "./platformStore.svelte.js";
  import { layoutEngine } from "./layoutStore.svelte.js";

  let { node } = $props<{ node: SplitNode }>();

  let container: HTMLDivElement;
  let isDragging = $state(false);
  let activeResizerIndex = $state(-1);

  function onPointerDown(e: PointerEvent, index: number) {
    if (platform.current !== "desktop") return;
    isDragging = true;
    activeResizerIndex = index;
    (e.target as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!isDragging || activeResizerIndex === -1 || !container) return;

    const rect = container.getBoundingClientRect();
    const totalSize = node.direction === "horizontal" ? rect.width : rect.height;
    const offset = node.direction === "horizontal" ? e.clientX - rect.left : e.clientY - rect.top;
    
    const percentage = (offset / totalSize) * 100;
    
    // Simple two-child resize logic for now
    // In a multi-child scenario, this would adjust adjacent sizes
    if (node.children.length === 2) {
        const newSizes = [percentage, 100 - percentage];
        // Constrain to 10% - 90%
        if (newSizes[0] >= 5 && newSizes[1] >= 5) {
            layoutEngine.resizeLeaf(node.id, newSizes);
        }
    }
  }

  function onPointerUp(e: PointerEvent) {
    isDragging = false;
    activeResizerIndex = -1;
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div 
    bind:this={container}
    class="split-pane {node.direction} {isDragging ? 'dragging' : ''}"
    onpointermove={onPointerMove}
    role="presentation"
>
  {#each node.children as child, i}
    <div class="pane" style="flex: {node.sizes[i] || (100 / node.children.length)}%;">
      <LayoutRenderer node={child} />
    </div>
    {#if i < node.children.length - 1 && platform.current === "desktop"}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div 
        class="resizer {node.direction}" 
        onpointerdown={(e) => onPointerDown(e, i)}
        onpointerup={onPointerUp}
      ></div>
    {/if}
  {/each}
</div>

<style>
  .split-pane { display: flex; width: 100%; height: 100%; overflow: hidden; position: relative; }
  .split-pane.horizontal { flex-direction: row; }
  .split-pane.vertical { flex-direction: column; }
  .pane { display: flex; flex-direction: column; overflow: hidden; position: relative; }
  .resizer { background: #333; z-index: 10; flex-shrink: 0; transition: background 0.2s; }
  .resizer.horizontal { width: 4px; cursor: col-resize; margin: 0 -2px; }
  .resizer.vertical { height: 4px; cursor: row-resize; margin: -2px 0; }
  .resizer:hover, .dragging .resizer { background: #007acc; }
  .dragging { cursor: inherit; }
  .dragging.horizontal { cursor: col-resize; }
  .dragging.vertical { cursor: row-resize; }
</style>
