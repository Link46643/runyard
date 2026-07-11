<script lang="ts">
  import { X, GitBranch, ChevronDown, Cpu, Menu, Plus } from "lucide-svelte";
  import { chatStore } from "../stores/chatStore.svelte.js";
  import ConversationList from "./chat/ConversationList.svelte";
  import ChatMessageList from "./chat/ChatMessageList.svelte";
  import ChatInput from "./chat/ChatInput.svelte";
  import ChatInputArea from "./chat/ChatInputArea.svelte";
  import { layoutEngine } from "./layoutStore.svelte.js";
  import { acpStore } from "../stores/acpStore.svelte.js";
  import { chatInputStore } from "../stores/chatInputStore.svelte.js";
  import { invoke } from "@tauri-apps/api/core";
  import { onDestroy } from "svelte";

  let { tab }: { tab?: { props: Record<string, unknown> } } = $props();

  let editingModel = $state(false);
  let modelDraft = $state("");
  let branchMenuOpen = $state(false);
  let scrollToMessageId = $state<string | null>(null);
  let agentMenuOpen = $state(false);

  // Task 3: Mobile sidebar toggle
  let sidebarOpen = $state(false);

  // Retry last message: triggered by ErrorBlock's "chat:retry-last-message" event
  function handleRetryLastMessage() {
    const messages = chatStore.messages;
    const lastUserMsg = [...messages].reverse().find((m) => m.role === "user");
    if (!lastUserMsg) return;
    const lastUserText = lastUserMsg.content
      .map((b: any) => ("text" in b ? b.text : "code" in b ? b.code : ""))
      .join(" ")
      .trim();
    chatInputStore.text = lastUserText;
    chatInputStore.sendMessage(activeConversation?.id ?? "");
  }

  $effect(() => {
    window.addEventListener("chat:retry-last-message", handleRetryLastMessage);
    return () => {
      window.removeEventListener("chat:retry-last-message", handleRetryLastMessage);
    };
  });

  // Task 1: Branch new-branch inline input
  let creatingBranch = $state(false);
  let newBranchName = $state("");

  async function confirmNewBranch() {
    const name = newBranchName.trim();
    if (!name || !activeConversation) return;
    const lastMsgId = chatStore.messages[chatStore.messages.length - 1]?.id ?? "";
    await invoke("chat_branch_create", {
      conversationId: activeConversation.id,
      name,
      messageId: lastMsgId,
    });
    // Reload branches so the newly created one appears
    await chatStore.loadBranches(activeConversation.id);
    newBranchName = "";
    creatingBranch = false;
  }

  function cancelNewBranch() {
    newBranchName = "";
    creatingBranch = false;
  }

  function relativeTime(ts: number): string {
    const diffMs = Date.now() - ts;
    const diffSec = Math.floor(diffMs / 1000);
    if (diffSec < 60) return `${diffSec}s ago`;
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return `${diffHr}h ago`;
    return `${Math.floor(diffHr / 24)}d ago`;
  }

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

  const availableModels = $derived.by(() => {
    if (!activeAgent) return ["claude-3-5-sonnet-latest", "gemini-2.5-pro", "gpt-4o", "deepseek-reasoner"];
    
    const agentId = activeAgent.agent_id.toLowerCase();
    if (agentId.includes("claude")) {
      return [
        "claude-3-5-sonnet-latest",
        "claude-3-5-haiku-latest",
        "claude-3-opus-latest"
      ];
    } else if (agentId.includes("gemini")) {
      return [
        "gemini-2.5-flash",
        "gemini-2.5-pro",
        "gemini-1.5-pro"
      ];
    } else if (agentId.includes("opencode")) {
      return [
        "opencode-32b",
        "opencode-7b",
        "deepseek-coder-33b",
        "gpt-4o"
      ];
    } else if (agentId.includes("goose")) {
      return [
        "gpt-4o",
        "claude-3-5-sonnet-latest",
        "gemini-2.5-pro"
      ];
    } else if (agentId.includes("codex")) {
      return [
        "gpt-4o",
        "gpt-4-turbo",
        "gpt-3.5-turbo"
      ];
    }
    
    return [
      "claude-3-5-sonnet-latest",
      "gemini-2.5-pro",
      "gpt-4o",
      "deepseek-reasoner"
    ];
  });

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
      const modelVal = modelDraft.trim();
      await chatStore.updateConversation(activeConversation.id, { model: modelVal });
      
      if (chatInputStore.activeConnectionId && chatInputStore.activeSessionId) {
        try {
          await acpStore.setConfigOption(
            chatInputStore.activeConnectionId,
            chatInputStore.activeSessionId,
            "model",
            modelVal
          );
        } catch (e) {
          console.warn("[ChatPanel] Failed to set model option live on active session", e);
        }
      }
    }
    editingModel = false;
  }

  function jumpToBranch(messageId: string) {
    scrollToMessageId = messageId;
    branchMenuOpen = false;
  }
</script>

<div class="chat-panel">
  <!-- Mobile sidebar overlay backdrop -->
  {#if sidebarOpen}
    <div class="sidebar-backdrop" onclick={() => (sidebarOpen = false)}></div>
  {/if}

  <div class="sidebar-wrapper" class:sidebar-open={sidebarOpen}>
    <ConversationList onNewConversation={newConversation} />
  </div>

  <div class="chat-main">
    <div class="chat-header">
      <button class="hamburger-btn" onclick={() => (sidebarOpen = !sidebarOpen)} title="Toggle conversations">
        <Menu size={15} strokeWidth={1.5} />
      </button>

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
          <select
            class="model-select"
            bind:value={modelDraft}
            onchange={saveModel}
            onblur={saveModel}
          >
            {#each availableModels as modelOption}
              <option value={modelOption}>{modelOption}</option>
            {/each}
            {#if !availableModels.includes(modelDraft)}
              <option value={modelDraft}>{modelDraft}</option>
            {/if}
          </select>
        {:else}
          <button class="model-badge" onclick={startEditModel} title="Click to change model">{activeConversation.model}</button>
        {/if}
      {/if}

      {#if activeConversation}
        <div class="branch-menu-wrapper">
          <button class="branch-indicator" onclick={() => (branchMenuOpen = !branchMenuOpen)}>
            <GitBranch size={12} strokeWidth={1.5} />
            {chatStore.branches.length} branch{chatStore.branches.length === 1 ? "" : "es"}
            <ChevronDown size={11} strokeWidth={1.5} />
          </button>
          {#if branchMenuOpen}
            <div class="branch-menu branch-menu--rich">
              {#if chatStore.branches.length > 0}
                {#each chatStore.branches as branch (branch.id)}
                  <div class="branch-tree-item">
                    <span class="branch-tree-icon"><GitBranch size={11} strokeWidth={1.5} /></span>
                    <span class="branch-tree-name">{branch.name}</span>
                    <span class="branch-tree-time">{relativeTime(branch.created_at)}</span>
                    <button class="branch-jump-link" onclick={() => jumpToBranch(branch.message_id)}>Jump to</button>
                  </div>
                {/each}
                <div class="branch-menu-divider"></div>
              {/if}
              {#if creatingBranch}
                <div class="branch-new-row">
                  <input
                    class="branch-new-input"
                    placeholder="Branch name…"
                    bind:value={newBranchName}
                    onkeydown={(e) => {
                      if (e.key === "Enter") confirmNewBranch();
                      if (e.key === "Escape") cancelNewBranch();
                    }}
                    autofocus
                  />
                  <button class="branch-new-confirm" onclick={confirmNewBranch}>OK</button>
                  <button class="branch-new-cancel" onclick={cancelNewBranch}>✕</button>
                </div>
              {:else}
                <button class="branch-new-ghost" onclick={() => (creatingBranch = true)}>
                  <Plus size={11} strokeWidth={1.5} /> New branch
                </button>
              {/if}
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
  .model-select {
    font-size: var(--text-xs);
    font-family: var(--font-mono);
    background: var(--bg-tertiary);
    border: 1px solid var(--border-active);
    border-radius: var(--radius-1);
    padding: 1px 6px;
    color: var(--text);
    cursor: pointer;
    outline: none;
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

  /* ── Task 1: Rich branch tree dropdown ──────────────────────────────────── */
  .branch-menu--rich {
    max-height: 300px;
    overflow-y: auto;
    min-width: 240px;
  }

  .branch-tree-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    color: var(--text);
    font-size: var(--text-sm);
    border-radius: var(--radius-1);
  }

  .branch-tree-item:hover {
    background: var(--bg-tertiary);
  }

  .branch-tree-icon {
    color: var(--text-tertiary);
    display: flex;
    flex-shrink: 0;
  }

  .branch-tree-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .branch-tree-time {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    white-space: nowrap;
    flex-shrink: 0;
  }

  .branch-jump-link {
    background: none;
    border: none;
    color: var(--accent);
    font-size: var(--text-xs);
    cursor: pointer;
    padding: 0 2px;
    white-space: nowrap;
    flex-shrink: 0;
    font-family: var(--font-sans);
  }

  .branch-jump-link:hover {
    text-decoration: underline;
  }

  .branch-menu-divider {
    height: 1px;
    background: var(--border);
    margin: var(--space-1) 0;
  }

  .branch-new-ghost {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    background: none;
    border: none;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-family: var(--font-sans);
    padding: 6px 10px;
    cursor: pointer;
    border-radius: var(--radius-1);
  }

  .branch-new-ghost:hover {
    background: var(--bg-tertiary);
    color: var(--text);
  }

  .branch-new-row {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
  }

  .branch-new-input {
    flex: 1;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-active);
    border-radius: var(--radius-1);
    color: var(--text);
    font-size: var(--text-sm);
    padding: 3px 6px;
    font-family: var(--font-sans);
    outline: none;
  }

  .branch-new-confirm,
  .branch-new-cancel {
    background: none;
    border: none;
    cursor: pointer;
    font-size: var(--text-xs);
    padding: 2px 4px;
    border-radius: var(--radius-1);
    color: var(--text-secondary);
  }

  .branch-new-confirm:hover {
    color: var(--text);
    background: var(--bg-tertiary);
  }

  .branch-new-cancel:hover {
    color: var(--text);
    background: var(--bg-tertiary);
  }

  /* ── Task 3: Mobile layout ──────────────────────────────────────────────── */
  .sidebar-wrapper {
    display: contents;
  }

  .sidebar-backdrop {
    display: none;
  }

  .hamburger-btn {
    display: none;
    align-items: center;
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    padding: 2px 4px;
    border-radius: var(--radius-1);
    flex-shrink: 0;
  }

  .hamburger-btn:hover {
    background: var(--bg-tertiary);
    color: var(--text);
  }

  @media (max-width: 768px) {
    .hamburger-btn {
      display: flex;
    }

    .sidebar-wrapper {
      display: block;
      position: absolute;
      top: 0;
      left: 0;
      height: 100%;
      z-index: 50;
      transform: translateX(-100%);
      transition: transform 0.2s ease;
      border-right: 1px solid var(--border);
    }

    .sidebar-wrapper.sidebar-open {
      transform: translateX(0);
    }

    .sidebar-backdrop {
      display: block;
      position: absolute;
      inset: 0;
      z-index: 49;
      background: var(--bg);
      opacity: 0.5;
    }

    .chat-panel {
      position: relative;
      overflow: hidden;
    }

    :global(.chat-input-area) {
      position: sticky;
      bottom: 0;
    }
  }
</style>
