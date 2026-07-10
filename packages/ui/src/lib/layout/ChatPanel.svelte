<script lang="ts">
  import { X, GitBranch, ChevronDown, Cpu } from "lucide-svelte";
  import { chatStore } from "../stores/chatStore.svelte.js";
  import ConversationList from "./chat/ConversationList.svelte";
  import ChatMessageList from "./chat/ChatMessageList.svelte";
  import ChatInput from "./chat/ChatInput.svelte";
  import ChatInputArea from "./chat/ChatInputArea.svelte";
  import { layoutEngine } from "./layoutStore.svelte.js";
  import { acpStore } from "../stores/acpStore.svelte.js";
  import { chatInputStore } from "../stores/chatInputStore.svelte.js";

  let { tab }: { tab?: { props: Record<string, unknown> } } = $props();

  let editingModel = $state(false);
  let modelDraft = $state("");
  let branchMenuOpen = $state(false);
  let scrollToMessageId = $state<string | null>(null);
  let agentMenuOpen = $state(false);

  // ── Streaming: bridge ACP chunk events into chatStore messages ──────────────
  // When chatInputStore.isStreaming becomes true, we push an empty assistant
  // message into the store so incoming chunks have somewhere to land.
  // When it becomes false, we stop listening.

  $effect(() => {
    const streaming = chatInputStore.isStreaming;

    if (!streaming) return;

    // Push an empty assistant message as a placeholder for the incoming stream
    const convId = chatStore.activeConversationId;
    if (convId) {
      const parentId =
        chatStore.messages.length > 0
          ? chatStore.messages[chatStore.messages.length - 1].id
          : null;

      // Insert a streaming placeholder message locally (no DB round-trip yet).
      // We synthesise a temporary Message object and append it to the list.
      const tempMsg = {
        id: `streaming-${Date.now()}`,
        conversation_id: convId,
        parent_id: parentId,
        role: "assistant" as const,
        content: [{ type: "text" as const, text: "" }],
        created_at: Date.now(),
        is_pinned: false,
      };
      chatStore.messages = [...chatStore.messages, tempMsg];
    }

    function onChunk(e: Event) {
      const detail = (e as CustomEvent).detail as { text?: string } | undefined;
      const chunk = detail?.text ?? "";
      if (!chunk) return;

      // Append chunk text to the last assistant message's first TextBlock
      const msgs = chatStore.messages;
      if (msgs.length === 0) return;
      const last = msgs[msgs.length - 1];
      if (last.role !== "assistant") return;

      const firstBlock = last.content[0];
      if (!firstBlock || firstBlock.type !== "text") return;

      // Produce a new messages array with the updated text block
      const updatedBlock = { ...firstBlock, text: firstBlock.text + chunk };
      const updatedMsg = { ...last, content: [updatedBlock, ...last.content.slice(1)] };
      chatStore.messages = [...msgs.slice(0, -1), updatedMsg];
    }

    function onCompleted(_e: Event) {
      // Streaming done — chatInputStore will set isStreaming = false via its own
      // listener; nothing extra to do here.
    }

    window.addEventListener("acp:agent_message_chunk", onChunk);
    window.addEventListener("acp:prompt_completed", onCompleted);

    return () => {
      window.removeEventListener("acp:agent_message_chunk", onChunk);
      window.removeEventListener("acp:prompt_completed", onCompleted);
    };
  });

  async function newConversation() {
    await chatStore.createConversation("New conversation", "unassigned");
  }

  function openFileFromChat(path: string) {
    layoutEngine.openEditor(path, path.split("/").pop() ?? path);
  }

  const activeConversation = $derived(
    chatStore.conversations.find((c) => c.id === chatStore.activeConversationId)
  );

  const activeAgent = $derived(
    acpStore.agents.find(
      (a) => a.status === "connected" || a.status === "ready" || a.status === "processing"
    )
  );
  const hasActiveAgent = $derived(activeAgent !== null && activeAgent !== undefined);

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
      <div class="agent-switcher-wrapper">
        {#if hasActiveAgent}
          <div
            class="agent-status-dot"
            class:dot-ready={activeAgent?.status === "connected" || activeAgent?.status === "ready"}
            class:dot-processing={activeAgent?.status === "processing"}
            class:dot-error={activeAgent?.status === "error"}
            class:dot-muted={activeAgent?.status === "connecting"}
          ></div>
        {/if}
        <button
          class="agent-switcher-btn"
          onclick={() => (agentMenuOpen = !agentMenuOpen)}
          title={hasActiveAgent ? `Agent: ${activeAgent?.name}` : "No agent connected"}
        >
          <Cpu size={14} strokeWidth={1.5} />
          <span class="agent-switcher-name">{hasActiveAgent ? activeAgent?.name : "No agent"}</span>
          <ChevronDown size={11} strokeWidth={1.5} />
        </button>
        {#if agentMenuOpen}
          <div class="agent-menu">
            {#if hasActiveAgent}
              {#each acpStore.agents.filter(a => a.status === "connected" || a.status === "ready" || a.status === "processing") as agent (agent.id)}
                <div class="agent-menu-item agent-menu-item--display">
                  <span class="agent-menu-dot dot-ready"></span>
                  <span>{agent.name}</span>
                </div>
              {/each}
              <div class="agent-menu-divider"></div>
            {:else}
              <div class="agent-menu-empty">No agent connected.</div>
            {/if}
            <button
              class="agent-menu-manage"
              onclick={() => {
                agentMenuOpen = false;
                (layoutEngine as any).openAgentManager?.();
              }}
            >Manage agents →</button>
          </div>
        {/if}
      </div>

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
      <ChatInputArea conversationId={activeConversation?.id ?? ""} />
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

  /* Agent switcher */
  .agent-switcher-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .agent-switcher-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 4px;
    border-radius: var(--radius-1);
    color: var(--text-secondary);
  }

  .agent-switcher-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text);
  }

  .agent-switcher-name {
    font-size: 12px;
    color: var(--text-secondary);
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .agent-menu {
    position: absolute;
    top: calc(100% + 4px);
    left: 0;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: var(--shadow-1);
    min-width: 180px;
    max-width: 240px;
    z-index: 100;
    padding: var(--space-1);
  }

  .agent-menu-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    font-size: var(--text-sm);
    color: var(--text);
    border-radius: var(--radius-1);
    cursor: default;
    user-select: none;
  }

  .agent-menu-empty {
    padding: 6px 10px;
    font-size: var(--text-sm);
    color: var(--text-tertiary);
  }

  .agent-menu-divider {
    height: 1px;
    background: var(--border);
    margin: var(--space-1) 0;
  }

  .agent-menu-manage {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 6px 10px;
    font-size: var(--text-sm);
    color: var(--text-secondary);
    cursor: pointer;
    border-radius: var(--radius-1);
    font-family: var(--font-sans);
  }

  .agent-menu-manage:hover {
    background: var(--bg-tertiary);
    color: var(--text);
  }

  /* Status dots - 6x6 static circles */
  .agent-status-dot {
    width: 6px;
    height: 6px;
    border-radius: 9999px;
    flex-shrink: 0;
  }

  .agent-menu-dot {
    width: 6px;
    height: 6px;
    border-radius: 9999px;
    flex-shrink: 0;
  }

  .dot-ready {
    background: var(--text-success);
  }

  .dot-processing {
    background: var(--text-warning);
  }

  .dot-error {
    background: var(--text-error);
  }

  .dot-muted {
    background: var(--text-tertiary);
  }
</style>
