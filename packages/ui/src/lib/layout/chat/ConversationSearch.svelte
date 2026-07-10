<script lang="ts">
  import { invoke as tauriInvoke } from "@tauri-apps/api/core";
  import { webSocketClient } from "@runyard/common";
  import type { Message } from "@runyard/common";
  import { Search, X } from "lucide-svelte";
  import { chatStore } from "../../stores/chatStore.svelte.js";

  // ── Types ──────────────────────────────────────────────────────────────────

  interface SearchResult {
    id: string;
    conversation_id: string;
    snippet: string;
    role: string;
  }

  // ── Invoke helper (mirrors chatStore pattern) ──────────────────────────────

  async function invoke<T>(cmd: string, args?: any): Promise<T> {
    if (webSocketClient.status === "connected") {
      return webSocketClient.invoke<T>(cmd, args);
    } else {
      return tauriInvoke<T>(cmd, args);
    }
  }

  // ── State ──────────────────────────────────────────────────────────────────

  let searchQuery = $state("");
  let results = $state<Message[]>([]);
  let isSearching = $state(false);
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let inputEl: HTMLInputElement | undefined = $state();

  // ── Effects ────────────────────────────────────────────────────────────────

  $effect(() => {
    const query = searchQuery;

    if (debounceTimer !== null) {
      clearTimeout(debounceTimer);
    }

    if (!query.trim()) {
      results = [];
      isSearching = false;
      return;
    }

    isSearching = true;
    debounceTimer = setTimeout(async () => {
      try {
        // chat_search uses FTS5 under the hood; single-word param needs no camelCase change
        const list = await invoke<Message[]>("chat_search", { query });
        results = list;
      } catch (e) {
        console.error("[ConversationSearch] search failed", e);
        results = [];
      } finally {
        isSearching = false;
      }
    }, 300);
  });

  // ── Functions ──────────────────────────────────────────────────────────────

  async function selectResult(msg: Message) {
    await chatStore.selectConversation(msg.conversation_id);
    await chatStore.openConversationInTab(msg.conversation_id);
    clear();
  }

  function clear() {
    searchQuery = "";
    results = [];
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      clear();
    }
  }

  function getSnippet(msg: Message): string {
    for (const block of msg.content) {
      if ("text" in block && block.text) {
        const text = (block as any).text as string;
        return text.length > 120 ? text.slice(0, 120) + "..." : text;
      }
    }
    return "(no text)";
  }

  function getConversationTitle(conversationId: string): string {
    return (
      chatStore.conversations.find((c) => c.id === conversationId)?.title ??
      conversationId
    );
  }
</script>

<div class="conv-search">
  <div class="search-input-row">
    <Search size={13} strokeWidth={1.5} class="search-icon" />
    <input
      bind:this={inputEl}
      bind:value={searchQuery}
      onkeydown={handleKeydown}
      class="search-input"
      placeholder="Search conversations..."
      autocomplete="off"
      spellcheck="false"
    />
    {#if searchQuery}
      <button class="clear-btn" onclick={clear} aria-label="Clear search">
        <X size={12} strokeWidth={1.5} />
      </button>
    {/if}
  </div>

  {#if searchQuery.trim()}
    <div class="search-results">
      {#if isSearching}
        <div class="search-status">Searching...</div>
      {:else if results.length === 0}
        <div class="search-status">No results</div>
      {:else}
        {#each results as msg (msg.id)}
          <button
            class="search-result-item"
            onclick={() => selectResult(msg)}
          >
            <span class="result-conv-title">{getConversationTitle(msg.conversation_id)}</span>
            <span class="result-snippet">{getSnippet(msg)}</span>
            <span class="result-role">{msg.role}</span>
          </button>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .conv-search {
    position: relative;
    width: 100%;
  }

  .search-input-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 0 8px;
  }

  .search-input-row:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-color: var(--border-active);
  }

  :global(.search-icon) {
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .search-input {
    flex: 1;
    background: none;
    border: none;
    outline: none;
    color: var(--text);
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    padding: 6px 0;
  }

  .search-input::placeholder {
    color: var(--text-tertiary);
  }

  .clear-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-tertiary);
    display: flex;
    align-items: center;
    padding: 0;
    flex-shrink: 0;
  }

  .clear-btn:hover {
    color: var(--text);
  }

  .search-results {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    right: 0;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 2px;
    box-shadow: var(--shadow-1);
    max-height: 320px;
    overflow-y: auto;
    z-index: 300;
  }

  .search-status {
    padding: 10px 12px;
    font-size: var(--text-sm);
    color: var(--text-tertiary);
    text-align: center;
  }

  .search-result-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border);
    padding: 8px 12px;
    cursor: pointer;
    font-family: var(--font-sans);
  }

  .search-result-item:last-child {
    border-bottom: none;
  }

  .search-result-item:hover {
    background: var(--bg-tertiary);
  }

  .result-conv-title {
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .result-snippet {
    font-size: var(--text-sm);
    color: var(--text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .result-role {
    font-size: 11px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
  }
</style>
