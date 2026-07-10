<script lang="ts">
  import { X, Trash2, GitBranch } from "lucide-svelte";
  import { chatStore } from "../stores/chatStore.svelte.js";
  import ConversationList from "./chat/ConversationList.svelte";
  import ChatMessageList from "./chat/ChatMessageList.svelte";
  import ChatInput from "./chat/ChatInput.svelte";
  import { layoutEngine } from "./layoutStore.svelte.js";

  let { tab }: { tab?: { props: Record<string, unknown> } } = $props();

  async function newConversation() {
    await chatStore.createConversation("New conversation", "unassigned");
  }

  function openFileFromChat(path: string) {
    layoutEngine.openEditor(path, path.split("/").pop() ?? path);
  }

  const activeConversation = $derived(
    chatStore.conversations.find((c) => c.id === chatStore.activeConversationId)
  );

  function clearContext() {
    // Non-destructive: just clears the local view of messages for the active
    // conversation. Full "clear + preserve pinned context" semantics land
    // once context assembly (section 1.5) exists.
    if (chatStore.activeConversationId) {
      chatStore.loadMessages(chatStore.activeConversationId, 1, 0);
    }
  }
</script>

<div class="chat-panel">
  <ConversationList onNewConversation={newConversation} />

  <div class="chat-main">
    <div class="chat-header">
      <span class="conv-title">{activeConversation?.title ?? "AI CHAT"}</span>
      {#if activeConversation}
        <span class="model-badge">{activeConversation.model}</span>
      {/if}
      {#if chatStore.branches.length > 0}
        <span class="branch-indicator"><GitBranch size={12} strokeWidth={1.5} /> {chatStore.branches.length} branch{chatStore.branches.length === 1 ? "" : "es"}</span>
      {/if}
      <div class="spacer"></div>
      <button class="header-btn" onclick={clearContext} disabled={!activeConversation}>Clear</button>
    </div>

    {#if chatStore.conversationTabs.length > 0}
      <div class="conv-tabs">
        {#each chatStore.conversationTabs as convId (convId)}
          {@const conv = chatStore.conversations.find((c) => c.id === convId)}
          <button
            class="conv-tab"
            class:active={convId === chatStore.activeTabId}
            onclick={() => chatStore.selectConversation(convId)}
          >
            <span class="conv-tab-title">{conv?.title ?? "Conversation"}</span>
            <span
              class="conv-tab-close"
              role="button"
              tabindex="0"
              onclick={(e) => { e.stopPropagation(); chatStore.closeConversationTab(convId); }}
              onkeydown={(e) => { if (e.key === "Enter") { e.stopPropagation(); chatStore.closeConversationTab(convId); } }}
            >
              <X size={12} strokeWidth={1.5} />
            </span>
          </button>
        {/each}
      </div>
    {/if}

    {#if activeConversation}
      <ChatMessageList messages={chatStore.messages} onOpenFile={openFileFromChat} />
      <ChatInput />
    {:else}
      <div class="empty-panel">
        <p>No conversation open.</p>
        <button class="ghost-btn" onclick={newConversation}>New conversation</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .chat-panel {
    display: flex;
    width: 100%;
    height: 100%;
    background: var(--chat-bg);
  }
  .chat-main {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .chat-header {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    height: 36px;
    padding: 0 var(--space-4);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .conv-title {
    font-size: var(--text-base);
    font-weight: 600;
    color: var(--text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .model-badge {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    background: var(--bg-tertiary);
    padding: 1px 6px;
    border-radius: var(--radius-1);
    font-family: var(--font-mono);
  }
  .branch-indicator {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: var(--text-xs);
    color: var(--text-tertiary);
  }
  .spacer {
    flex: 1;
  }
  .header-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    border-radius: var(--radius-1);
    padding: 3px 10px;
    font-size: var(--text-xs);
    font-family: var(--font-sans);
    cursor: pointer;
  }
  .header-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .conv-tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
    flex-shrink: 0;
  }
  .conv-tab {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 8px 12px;
    background: var(--tab-bg);
    border: none;
    border-right: 1px solid var(--border);
    color: var(--tab-text);
    font-size: var(--text-base);
    cursor: pointer;
    max-width: 180px;
  }
  .conv-tab.active {
    background: var(--tab-active-bg);
    color: var(--tab-active-text);
    border-bottom: 2px solid var(--accent);
  }
  .conv-tab-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .conv-tab-close {
    display: flex;
    opacity: 0.6;
  }
  .conv-tab-close:hover {
    opacity: 1;
  }
  .empty-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-4);
    color: var(--text-tertiary);
  }
  .ghost-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--text);
    border-radius: var(--radius-1);
    padding: 6px 14px;
    font-family: var(--font-sans);
    font-size: var(--text-base);
    cursor: pointer;
  }
</style>
