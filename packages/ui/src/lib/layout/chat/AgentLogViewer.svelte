<script lang="ts">
  // 1. Imports
  import type { AcpLogDirection } from "@runyard/common";

  // 2. Types
  interface LogEntry {
    direction: AcpLogDirection;
    line: string;
    ts: number;
  }

  type FilterMode = "all" | AcpLogDirection;

  // 3. Props
  let {
    connectionId,
    logs,
    onClear,
  }: {
    connectionId: string;
    logs: LogEntry[];
    onClear: () => void;
  } = $props();

  // 4. State
  let activeFilters = $state<Set<AcpLogDirection>>(new Set(["stdout", "stderr"]));
  let containerEl = $state<HTMLDivElement | null>(null);

  // 5. Derived
  const filteredLogs = $derived(
    logs.filter((entry) => activeFilters.has(entry.direction))
  );

  // 6. Effects
  $effect(() => {
    // Track logs.length to re-run when new logs arrive
    const _len = logs.length;
    if (!containerEl) return;
    const el = containerEl;
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    if (distanceFromBottom <= 48) {
      el.scrollTop = el.scrollHeight;
    }
  });

  // 7. Functions
  function isFilterActive(dir: AcpLogDirection): boolean {
    return activeFilters.has(dir);
  }

  function isAllActive(): boolean {
    return activeFilters.has("stdin") && activeFilters.has("stdout") && activeFilters.has("stderr");
  }

  function toggleAll() {
    if (isAllActive()) {
      // Reset to default: stdout + stderr
      activeFilters = new Set(["stdout", "stderr"]);
    } else {
      activeFilters = new Set(["stdin", "stdout", "stderr"]);
    }
  }

  function toggleFilter(dir: AcpLogDirection) {
    const next = new Set(activeFilters);
    if (next.has(dir)) {
      next.delete(dir);
    } else {
      next.add(dir);
    }
    activeFilters = next;
  }

  function directionGlyph(dir: AcpLogDirection): string {
    if (dir === "stdin") return "→";
    if (dir === "stdout") return "←";
    return "!";
  }

  function directionClass(dir: AcpLogDirection): string {
    if (dir === "stdin") return "dir-stdin";
    if (dir === "stdout") return "dir-stdout";
    return "dir-stderr";
  }
</script>

<div class="agent-log-viewer">
  <div class="log-header">
    <span class="log-title">AGENT LOGS</span>
    <div class="filter-group">
      <button
        class="filter-btn"
        class:active={isAllActive()}
        onclick={toggleAll}
      >All</button>
      <button
        class="filter-btn"
        class:active={isFilterActive("stdin")}
        onclick={() => toggleFilter("stdin")}
      >stdin</button>
      <button
        class="filter-btn"
        class:active={isFilterActive("stdout")}
        onclick={() => toggleFilter("stdout")}
      >stdout</button>
      <button
        class="filter-btn"
        class:active={isFilterActive("stderr")}
        onclick={() => toggleFilter("stderr")}
      >stderr</button>
    </div>
    <button class="clear-btn" onclick={onClear}>Clear</button>
  </div>

  <div class="log-body" bind:this={containerEl}>
    {#if filteredLogs.length === 0}
      <div class="log-empty">No logs yet.</div>
    {:else}
      {#each filteredLogs as entry, i (entry.ts + entry.line + i)}
        <div class="log-row" class:even={i % 2 === 1}>
          <span class="log-dir {directionClass(entry.direction)}" aria-label={entry.direction}>
            {directionGlyph(entry.direction)}
          </span>
          <span class="log-line" title={entry.line}>{entry.line}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .agent-log-viewer {
    border-top: 1px solid var(--border);
    background: var(--bg);
    border-radius: 0;
  }

  .log-header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 4px 8px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .log-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    color: var(--text-secondary);
    text-transform: uppercase;
    flex-shrink: 0;
  }

  .filter-group {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: var(--space-2);
  }

  .filter-btn {
    font-size: 10px;
    font-family: var(--font-mono);
    padding: 1px 6px;
    background: none;
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--text-tertiary);
    cursor: pointer;
    line-height: 1.6;
  }

  .filter-btn:hover {
    color: var(--text-secondary);
    border-color: var(--border-active);
  }

  .filter-btn.active {
    color: var(--text);
    border-color: var(--border-active);
    background: var(--bg-tertiary);
  }

  .clear-btn {
    margin-left: auto;
    font-size: 11px;
    font-family: var(--font-sans);
    padding: 1px 8px;
    background: none;
    border: 1px solid var(--border);
    border-radius: 3px;
    color: var(--text-secondary);
    cursor: pointer;
    line-height: 1.6;
  }

  .clear-btn:hover {
    color: var(--text);
    border-color: var(--border-active);
  }

  .log-body {
    max-height: 240px;
    overflow-y: auto;
  }

  .log-empty {
    font-size: 11px;
    color: var(--text-tertiary);
    text-align: center;
    padding: 20px 0;
  }

  .log-row {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 1px 8px;
    background: transparent;
  }

  .log-row.even {
    background: color-mix(in srgb, var(--bg-tertiary) 30%, transparent);
  }

  .log-dir {
    font-size: 11px;
    font-family: var(--font-mono);
    width: 1.2em;
    flex-shrink: 0;
    text-align: center;
    user-select: none;
  }

  .dir-stdin {
    color: var(--accent);
  }

  .dir-stdout {
    color: var(--text-secondary);
  }

  .dir-stderr {
    color: var(--text-warning);
  }

  .log-line {
    font-size: 11px;
    font-family: var(--font-mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
    flex: 1;
    min-width: 0;
  }
</style>
