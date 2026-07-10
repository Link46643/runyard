<script lang="ts">
  // MiniMap.svelte — lightweight visual document overview sidebar.
  // Renders up to MAX_LINES horizontal "line" bars at a 20% scale to give a
  // bird's-eye view of the document. Clicking scrolls the editor proportionally.
  // This is a pure CSS/SVG approximation — it is NOT a real CodeMirror extension.

  const MAX_LINES = 500;
  // Height of one minimap line row in pixels.
  const LINE_H = 2;
  // Gap between line rows in pixels.
  const LINE_GAP = 1;
  // Width of the content area inside the 72px column.
  const CONTENT_W = 60;

  let {
    content,
    scrollPercent,
    onScrollTo,
  }: {
    content: string;
    scrollPercent: number;
    onScrollTo: (percent: number) => void;
  } = $props();

  // Split into lines, capped at MAX_LINES.
  let lines = $derived(content.split("\n").slice(0, MAX_LINES));

  let totalHeight = $derived(lines.length * (LINE_H + LINE_GAP));

  // Height of the scroll-indicator overlay — represents ~20% of the document
  // (a rough approximation of the viewport).
  let indicatorHeight = $derived(Math.max(20, Math.round(totalHeight * 0.2)));

  // Top position of the scroll indicator, clamped so it never overflows.
  let indicatorTop = $derived(
    Math.min(
      Math.round(scrollPercent * totalHeight),
      totalHeight - indicatorHeight
    )
  );

  function handleClick(e: MouseEvent) {
    const el = e.currentTarget as HTMLElement;
    const rect = el.getBoundingClientRect();
    const y = e.clientY - rect.top;
    const pct = Math.max(0, Math.min(1, y / rect.height));
    onScrollTo(pct);
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="minimap"
  onclick={handleClick}
  role="scrollbar"
  aria-valuenow={Math.round(scrollPercent * 100)}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-label="Document minimap"
  tabindex="-1"
>
  <div class="minimap-lines" style="height: {totalHeight}px;">
    {#each lines as line, i}
      {@const hasContent = line.trim().length > 0}
      {@const indentDepth = line.length - line.trimStart().length}
      {@const contentLen = Math.min(line.trim().length, CONTENT_W)}
      {#if hasContent}
        <div
          class="minimap-line"
          style="
            top: {i * (LINE_H + LINE_GAP)}px;
            left: {Math.min(indentDepth, 20)}px;
            width: {contentLen}px;
          "
        ></div>
      {/if}
    {/each}

    <!-- Scroll position indicator overlay -->
    <div
      class="minimap-indicator"
      style="top: {indicatorTop}px; height: {indicatorHeight}px;"
    ></div>
  </div>
</div>

<style>
  .minimap {
    width: 72px;
    flex-shrink: 0;
    background: var(--bg-secondary);
    border-left: 1px solid var(--border);
    overflow: hidden;
    cursor: pointer;
    position: relative;
    /* Fill the editor height via flex parent */
    align-self: stretch;
  }

  .minimap-lines {
    position: relative;
    width: 100%;
  }

  .minimap-line {
    position: absolute;
    height: 2px;
    background: var(--text-tertiary, var(--text-secondary));
    opacity: 0.6;
    border-radius: 1px;
    pointer-events: none;
  }

  .minimap-indicator {
    position: absolute;
    left: 0;
    right: 0;
    background: var(--accent);
    opacity: 0.15;
    pointer-events: none;
  }
</style>
