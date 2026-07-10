<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { commandRegistry } from "./commandRegistry.svelte.js";
  import { layoutEngine } from "./layoutStore.svelte.js";
  import { appStatus } from "./appStatusStore.svelte.js";

  let { open = $bindable(false) }: { open?: boolean } = $props();

  let query = $state("");
  let selectedIndex = $state(0);
  let inputEl: HTMLInputElement;

  /** Unified palette item — commands, tabs, and recent files all share this shape. */
  interface PaletteItem {
    id: string;
    title: string;
    subtitle?: string;
    category: string;
    shortcut?: string;
    handler: () => void;
  }

  /** Recursively collect all non-welcome tabs from the layout tree. */
  function collectTabs(node: any): { id: string; title: string; type: string }[] {
    if (node.type === "leaf") {
      return (node.tabs as any[]).filter((t) => t.type !== "welcome");
    }
    if (node.type === "split") {
      return (node.children as any[]).flatMap(collectTabs);
    }
    return [];
  }

  let tabResults = $derived.by((): PaletteItem[] =>
    collectTabs(layoutEngine.layout.root).map((tab) => ({
      id: `tab:${tab.id}`,
      title: tab.title,
      subtitle: tab.type === "editor" ? tab.id : tab.type,
      category: "Open Tabs",
      handler: () => layoutEngine.setActiveTab(tab.id),
    }))
  );

  let recentResults = $derived.by((): PaletteItem[] =>
    appStatus.recentFiles.map((path) => {
      const name = path.split("/").pop() ?? path;
      return {
        id: `recent:${path}`,
        title: name,
        subtitle: path,
        category: "Recent Files",
        handler: () => layoutEngine.openEditor(path, name),
      };
    })
  );

  // Fuzzy-search filtered results — commands + open tabs + recent files
  let results = $derived.by((): PaletteItem[] => {
    const q = query.trim().toLowerCase();
    const allItems: PaletteItem[] = [
      ...(commandRegistry.commands as PaletteItem[]),
      ...tabResults,
      ...recentResults,
    ];
    if (!q) return allItems.slice(0, 40);
    return allItems
      .filter((item) => {
        const haystack = `${item.title} ${item.category} ${item.subtitle ?? ""}`.toLowerCase();
        return fuzzyMatch(q, haystack);
      })
      .slice(0, 40);
  });

  $effect(() => {
    if (open) {
      selectedIndex = 0;
      query = "";
      setTimeout(() => inputEl?.focus(), 0);
    }
  });

  function fuzzyMatch(needle: string, haystack: string): boolean {
    let j = 0;
    for (let i = 0; i < haystack.length && j < needle.length; i++) {
      if (haystack[i] === needle[j]) j++;
    }
    return j === needle.length;
  }

  function close() {
    open = false;
    query = "";
  }

  function execute(cmd: PaletteItem) {
    close();
    setTimeout(() => cmd.handler(), 0);
  }

  function handleKey(e: KeyboardEvent) {
    switch (e.key) {
      case "Escape":
        e.preventDefault();
        close();
        break;
      case "ArrowDown":
        e.preventDefault();
        selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
        scrollSelectedIntoView();
        break;
      case "ArrowUp":
        e.preventDefault();
        selectedIndex = Math.max(selectedIndex - 1, 0);
        scrollSelectedIntoView();
        break;
      case "Enter":
        e.preventDefault();
        if (results[selectedIndex]) {
          execute(results[selectedIndex]);
        }
        break;
    }
  }

  function scrollSelectedIntoView() {
    requestAnimationFrame(() => {
      const el = document.querySelector(".palette-item.selected");
      el?.scrollIntoView({ block: "nearest" });
    });
  }

  // Group results by category
  type Group = { category: string; items: PaletteItem[] };
  let grouped = $derived.by((): Group[] => {
    const map = new Map<string, PaletteItem[]>();
    for (const cmd of results) {
      const list = map.get(cmd.category) ?? [];
      list.push(cmd);
      map.set(cmd.category, list);
    }
    const groups: Group[] = [];
    for (const [category, items] of map.entries()) {
      groups.push({ category, items });
    }
    return groups;
  });

  // Flat list for keyboard navigation (mirrors `results`)
  let flatIndex = 0;
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="palette-backdrop" onclick={close}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="palette-container" onclick={(e) => e.stopPropagation()}>
      <div class="palette-search">
        <svg class="search-icon" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8" /><path d="m21 21-4.35-4.35" />
        </svg>
        <input
          bind:this={inputEl}
          bind:value={query}
          onkeydown={handleKey}
          oninput={() => (selectedIndex = 0)}
          class="palette-input"
          placeholder="Search commands, files, panels..."
          autocomplete="off"
          spellcheck="false"
        />
        {#if query}
          <button class="clear-btn" onclick={() => (query = "")}>×</button>
        {:else}
          <kbd class="esc-hint">Esc</kbd>
        {/if}
      </div>

      <div class="palette-results">
        {#if results.length === 0}
          <div class="no-results">No results for "{query}"</div>
        {:else}
          {flatIndex = 0}
          {#each grouped as group}
            <div class="result-group">
              <div class="group-label">{group.category}</div>
              {#each group.items as cmd}
                {@const idx = flatIndex++}
                <!-- svelte-ignore a11y_click_events_have_key_events -->
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div
                  class="palette-item"
                  class:selected={selectedIndex === idx}
                  onmouseenter={() => (selectedIndex = idx)}
                  onclick={() => execute(cmd)}
                >
                  <div class="item-content">
                    <span class="item-title">{cmd.title}</span>
                    {#if cmd.subtitle}
                      <span class="item-subtitle">{cmd.subtitle}</span>
                    {/if}
                  </div>
                  {#if cmd.shortcut}
                    <kbd class="item-shortcut">{cmd.shortcut}</kbd>
                  {/if}
                </div>
              {/each}
            </div>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .palette-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 10vh;
    backdrop-filter: blur(2px);
    animation: fade-in 0.1s ease;
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .palette-container {
    width: 560px;
    max-width: calc(100vw - 32px);
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    box-shadow:
      0 4px 6px -1px rgba(0, 0, 0, 0.4),
      0 20px 60px -8px rgba(0, 0, 0, 0.6);
    overflow: hidden;
    animation: slide-down 0.12s ease;
    max-height: 60vh;
    display: flex;
    flex-direction: column;
  }

  @keyframes slide-down {
    from {
      opacity: 0;
      transform: translateY(-8px) scale(0.98);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .palette-search {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .search-icon {
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .palette-input {
    flex: 1;
    background: none;
    border: none;
    color: var(--text);
    font-size: 15px;
    font-family: "Google Sans Flex Variable", system-ui, sans-serif;
    outline: none;
    min-width: 0;
  }

  .palette-input::placeholder {
    color: var(--text-secondary);
  }

  .clear-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: 18px;
    cursor: pointer;
    line-height: 1;
    padding: 0 2px;
    flex-shrink: 0;
  }

  .clear-btn:hover {
    color: var(--text);
  }

  .esc-hint {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 11px;
    padding: 2px 6px;
    color: var(--text-secondary);
    font-family: "JetBrains Mono", monospace;
    flex-shrink: 0;
  }

  .palette-results {
    overflow-y: auto;
    padding: 6px 0 8px;
    flex: 1;
    min-height: 0;
  }

  .no-results {
    padding: 24px 16px;
    text-align: center;
    color: var(--text-secondary);
    font-size: 13px;
  }

  .result-group {
    margin-bottom: 4px;
  }

  .group-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-secondary);
    padding: 8px 16px 3px;
    font-weight: 600;
  }

  .palette-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 7px 16px;
    cursor: pointer;
    gap: 12px;
    border-radius: 4px;
    margin: 0 6px;
  }

  .palette-item.selected {
    background: rgba(59, 130, 246, 0.12);
  }

  .palette-item:hover {
    background: rgba(255, 255, 255, 0.05);
  }

  .palette-item.selected:hover {
    background: rgba(59, 130, 246, 0.15);
  }

  .item-content {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }

  .item-title {
    font-size: 13px;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .palette-item.selected .item-title {
    color: var(--accent);
  }

  .item-subtitle {
    font-size: 11px;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-top: 1px;
  }

  .item-shortcut {
    font-size: 11px;
    font-family: "JetBrains Mono", monospace;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 6px;
    color: var(--text-secondary);
    white-space: nowrap;
    flex-shrink: 0;
  }
</style>
