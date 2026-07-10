<script lang="ts">
  import { Search, Plus, Trash2 } from "lucide-svelte";
  import { chatStore } from "../../stores/chatStore.svelte.js";
  import Modal from "../../Modal.svelte";

  let { onNewConversation }: { onNewConversation?: () => void } = $props();

  let renamingId = $state<string | null>(null);
  let renameValue = $state("");
  let deleteTargetId = $state<string | null>(null);
  let showDeleteModal = $state(false);

  function startRename(id: string, currentTitle: string) {
    renamingId = id;
    renameValue = currentTitle;
  }

  async function commitRename() {
    if (renamingId) {
      await chatStore.renameConversation(renamingId, renameValue.trim() || "Untitled");
    }
    renamingId = null;
  }

  function requestDelete(id: string) {
    deleteTargetId = id;
    showDeleteModal = true;
  }

  async function confirmDelete() {
    if (deleteTargetId) {
      await chatStore.deleteConversation(deleteTargetId);
    }
    showDeleteModal = false;
    deleteTargetId = null;
  }

  const deleteTarget = $derived(chatStore.conversations.find((c) => c.id === deleteTargetId));
</script>

<div class="conversation-list">
  <div class="list-header">
    <span class="header-label">Conversations</span>
    <button class="icon-btn" onclick={() => onNewConversation?.()} title="New conversation">
      <Plus size={14} strokeWidth={1.5} />
    </button>
  </div>
  <div class="search-row">
    <Search size={12} strokeWidth={1.5} />
    <input
      type="text"
      placeholder="Search conversations"
      bind:value={chatStore.searchQuery}
    />
  </div>
  <select class="sort-select" bind:value={chatStore.sortBy}>
    <option value="recent">Recent</option>
    <option value="name">Name</option>
    <option value="tokens">Tokens</option>
  </select>

  <div class="list-body">
    {#each chatStore.filteredConversations as conv (conv.id)}
      <div class="conv-item" class:active={conv.id === chatStore.activeConversationId}>
        {#if renamingId === conv.id}
          <input
            class="rename-input"
            bind:value={renameValue}
            onblur={commitRename}
            onkeydown={(e) => e.key === "Enter" && commitRename()}
          />
        {:else}
          <button
            class="conv-title"
            ondblclick={() => startRename(conv.id, conv.title)}
            onclick={() => chatStore.openConversationInTab(conv.id)}
          >
            {conv.title}
          </button>
        {/if}
        <span class="conv-meta">{conv.message_count} msgs</span>
        <button class="icon-btn delete-btn" onclick={() => requestDelete(conv.id)} title="Delete">
          <Trash2 size={12} strokeWidth={1.5} />
        </button>
      </div>
    {/each}
    {#if chatStore.filteredConversations.length === 0}
      <div class="empty">No conversations</div>
    {/if}
  </div>
</div>

<Modal
  bind:show={showDeleteModal}
  title="Delete conversation"
  message={deleteTarget ? `This will delete "${deleteTarget.title}" and its ${deleteTarget.message_count} messages permanently.` : ""}
  confirmLabel="Delete"
  cancelLabel="Cancel"
  onConfirm={confirmDelete}
  onCancel={() => (showDeleteModal = false)}
/>

<style>
  .conversation-list {
    width: 220px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    background: var(--sidebar-bg);
    border-right: 1px solid var(--border);
    height: 100%;
  }
  .list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-3) var(--space-4);
  }
  .header-label {
    font-size: var(--text-xs);
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }
  .icon-btn {
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    display: flex;
    align-items: center;
    padding: 2px;
  }
  .icon-btn:hover {
    color: var(--text);
  }
  .search-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0 var(--space-3) var(--space-2) var(--space-3);
    padding: 4px 8px;
    background: var(--bg-tertiary);
    border-radius: var(--radius-1);
    color: var(--text-tertiary);
  }
  .search-row input {
    background: none;
    border: none;
    color: var(--text);
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    outline: none;
    flex: 1;
  }
  .sort-select {
    margin: 0 var(--space-3) var(--space-3) var(--space-3);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-secondary);
    font-size: var(--text-xs);
    border-radius: var(--radius-1);
    padding: 3px 6px;
  }
  .list-body {
    flex: 1;
    overflow-y: auto;
  }
  .conv-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 6px var(--space-4);
    cursor: pointer;
  }
  .conv-item:hover {
    background: var(--bg-tertiary);
  }
  .conv-item.active {
    background: var(--bg-tertiary);
    border-left: 2px solid var(--accent);
  }
  .conv-title {
    flex: 1;
    background: none;
    border: none;
    text-align: left;
    color: var(--text);
    font-size: var(--text-base);
    font-family: var(--font-sans);
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 0;
  }
  .rename-input {
    flex: 1;
    background: var(--bg-elevated);
    border: 1px solid var(--border-active);
    color: var(--text);
    font-size: var(--text-base);
    padding: 2px 4px;
    border-radius: var(--radius-1);
  }
  .conv-meta {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
  }
  .delete-btn {
    opacity: 0;
  }
  .conv-item:hover .delete-btn {
    opacity: 1;
  }
  .empty {
    padding: var(--space-5);
    text-align: center;
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }
</style>
