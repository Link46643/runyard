<script lang="ts">
  import { X, GitBranch, ChevronDown } from "lucide-svelte";
  import { chatStore } from "../stores/chatStore.svelte.js";
  import ConversationList from "./chat/ConversationList.svelte";
  import ChatMessageList from "./chat/ChatMessageList.svelte";
  import ChatInput from "./chat/ChatInput.svelte";
  import { layoutEngine } from "./layoutStore.svelte.js";

  let { tab }: { tab?: { props: Record<string, unknown> } } = $props();

  let editingModel = $state(false);
  let modelDraft = $state("");
  let branchMenuOpen = $state(false);
  let scrollToMessageId = $state<string | null>(null);

  async function newConversation() {
    await chatStore.createConversation("New conversation", "unassigned");
  }

  function openFileFromChat(path: string) {
    layoutEngine.openEditor(path, path.split("/").pop() ?? path);
  }

  const activeConversation = $derived(
    chatStore.conversations.find((c) => c.id === chatStore.activeConversationId)
  );

  // Rough, clearly-labeled estimate (~4 chars/token) - there is no real
  // tokenizer wired up yet (that lands with context assembly in section 1.5),
  // so this is deliberately presented as an approximation, not an exact count.
  const estimatedTokens = $derived.by(() => {
    const chars = chatStore.messages.reduce((sum, m) => {
      const text = m.content
        .map((b) => ("text" in b ? b.text : "code" in b ? b.code : ""))
        .join(" ");
      return sum + text.length;
    }, 0);
    return Math.round(chars / 4);
  });
  const budget = $derived(activeConversation?.context_budget ?? 0);
  const usagePct = $derived(budget > 0 ? Math.min(100, (estimatedTokens / budget) * 100) : null);

  function clearContext() {
    // Non-destructive: just clears the local view of messages for the active
    // conversation. Full "clear + preserve pinned context" semantics land
    // once context assembly (section 1.5) exists.
    if (chatStore.activeConversationId) {
      chatStore.loadMessages(chatStore.activeConversationId, 1, 0);
    }
  }

  function startEditModel() {
    modelDraft = activeConversation?.model ?? "";
    editingModel = true;
  }

  async function saveModel() {
    if (activeConversation && modelDraft.trim()) {
      await chatStore.updateConversation(activeConversation.id, { model: modelDraft.trim() });
    }
    editingModel = false;
  }

  function jumpToBranch(messageId: string) {
    scrollToMessageId = messageId;
    branchMenuOpen = false;
  }
</script>

<div class="chat-panel">
  <ConversationList onNewConversation={newConversation} />

  <div class="chat-main">
    <div class="chat-header">
      <span class="conv-title">{activeConversation?.title ?? "AI CHAT"}</span>

      {#if activeConversation}
        {#if editingModel}
          <input
            class="model-input"
            bind:value={modelDraft}
            onblur={saveModel}
            onkeydown={(e) => e.key === "Enter" && saveModel()}
          />
        {:else}
          <button class="model-badge" onclick={startEditModel} title="Click to change model">{activeConversation.model}</button>
        {/if}
      {/if}

      {#if chatStore.branches.length > 0}
        <div class="branch-menu-wrapper">
          <button class="branch-indicator" onclick={() => (branchMenuOpen = !branchMenuOpen)}>
            <GitBranch size={12} strokeWidth={1.5} />
            {chatStore.branches.length} branch{chatStore.branches.length === 1 ? "" : "es"}
            <ChevronDown size={11} strokeWidth={1.5} />
          </button>
          {#if branchMenuOpen}
            <div class="branch-menu">
              {#each chatStore.branches as branch (branch.id)}
                <button class="branch-menu-item" onclick={() => jumpToBranch(branch.message_id)}>
                  {branch.name}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      {#if activeConversation}
        <div class="context-bar" title={`~${estimatedTokens.toLocaleString()} tokens (estimated)`}>
          {#if usagePct !== null}
            <div class="context-bar-track">
              <div class="context-bar-fill" style:width={`${usagePct}%`} class:warn={usagePct > 80}></div>
            </div>
            <span class="context-bar-label">~{estimatedTokens.toLocaleString()} / {budget.toLocaleString()}</span>
          {:else}
            <span class="context-bar-label">~{estimatedTokens.toLocaleString()} tokens (estimated)</span>
          {/if}
        </div>
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
      <ChatMessageList messages={chatStore.messages} onOpenFile={openFileFromChat} {scrollToMessageId} />
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
    border: none;
    cursor: pointer;
  }
  .model-badge:hover {
    color: var(--text);
  }
  .model-input {
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    background: var(--bg-tertiary);
    border: 1px solid var(--border-active);
    border-radius: var(--radius-1);
    padding: 1px 6px;
    color: var(--text);
    width: 120px;
  }
  .branch-menu-wrapper {
    position: relative;
  }
  .branch-indicator {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    background: none;
    border: none;
    cursor: pointer;
  }
  .branch-indicator:hover {
    color: var(--text);
  }
  .branch-menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-2);
    box-shadow: var(--shadow-1);
    min-width: 180px;
    z-index: 100;
    padding: var(--space-1);
  }
  .branch-menu-item {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 6px 10px;
    font-size: var(--text-sm);
    color: var(--text);
    cursor: pointer;
    border-radius: var(--radius-1);
  }
  .branch-menu-item:hover {
    background: var(--bg-tertiary);
  }
  .context-bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .context-bar-track {
    width: 60px;
    height: 3px;
    background: var(--border);
    border-radius: var(--radius-full);
    overflow: hidden;
  }
  .context-bar-fill {
    height: 100%;
    background: var(--accent-success);
  }
  .context-bar-fill.warn {
    background: var(--accent-warning);
  }
  .context-bar-label {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    white-space: nowrap;
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
