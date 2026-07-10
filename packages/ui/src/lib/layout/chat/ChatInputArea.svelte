<script lang="ts">
  import { invoke as tauriInvoke } from "@tauri-apps/api/core";
  import { webSocketClient } from "@runyard/common";
  import type { FsEntry, SkillCatalogEntry } from "@runyard/common";
  import {
    ArrowUp,
    Square,
    Paperclip,
    X,
    ChevronDown,
    Loader,
    Eye,
    Edit2,
    Zap,
    ChevronRight,
  } from "lucide-svelte";
  import { chatInputStore } from "../../stores/chatInputStore.svelte.js";
  import { chatStore } from "../../stores/chatStore.svelte.js";
  import { acpStore } from "../../stores/acpStore.svelte.js";

  // ── Local invoke helper (mirrors the pattern used across the codebase) ─────
  async function invoke<T>(cmd: string, args?: any): Promise<T> {
    if (webSocketClient.status === "connected") {
      return webSocketClient.invoke<T>(cmd, args);
    } else {
      return tauriInvoke<T>(cmd, args);
    }
  }

  // ── Types ──────────────────────────────────────────────────────────────────

  interface Props {
    conversationId: string;
    onHeightChange?: (height: number) => void;
  }

  // ── Props ──────────────────────────────────────────────────────────────────

  const { conversationId, onHeightChange }: Props = $props();

  // ── State ──────────────────────────────────────────────────────────────────

  let textareaEl: HTMLTextAreaElement | undefined = $state();
  let containerEl: HTMLDivElement | undefined = $state();
  let fileInputEl: HTMLInputElement | undefined = $state();
  let modelDropdownOpen = $state(false);

  // 1.5.11 – Markdown preview
  let showPreview = $state(false);

  // 1.5.14 – Context dropdown
  let contextDropdownOpen = $state(false);

  // 1.5.4 / 1.5.19 – Prompt enhancer
  let showEnhancer = $state(false);
  let enhancedText = $state("");

  // 1.5.2 – @-mention autocomplete
  let mentionOpen = $state(false);
  let mentionQuery = $state("");
  let mentionTab = $state<"files" | "skills" | "agents" | "conversations">("files");
  let mentionHighlight = $state(0);
  let mentionFiles = $state<FsEntry[]>([]);
  let mentionSkills = $state<SkillCatalogEntry[]>([]);
  let mentionDropdownEl: HTMLDivElement | undefined = $state();

  // ── Derived ────────────────────────────────────────────────────────────────

  const modelOptions = [
    "claude-sonnet-4-5",
    "claude-opus-4",
    "gpt-4o",
    "gemini-2.5-pro",
    "custom...",
  ] as const;

  // Rough token estimate: total chars in all messages / 4 (no real tokenizer)
  const estimatedTokens = $derived.by(() => {
    const chars = chatStore.messages.reduce((sum, m) => {
      const text = m.content
        .map((b) => ("text" in b ? (b as any).text : "code" in b ? (b as any).code : ""))
        .join(" ");
      return sum + text.length;
    }, 0);
    // Also count the current draft
    const draftChars = chatInputStore.text.length;
    return Math.round((chars + draftChars) / 4);
  });

  const canSend = $derived(
    (chatInputStore.text.trim().length > 0 || chatInputStore.attachments.length > 0) &&
      !!conversationId &&
      !chatInputStore.isStreaming
  );

  // @-mention chips: detect @{type}:{name} tokens in the text
  const mentionChips = $derived.by(() => {
    const matches = chatInputStore.text.match(/@[\w:]+/g) ?? [];
    return [...new Set(matches)];
  });

  // 1.5.11 – Markdown preview derived HTML
  function simpleMarkdown(raw: string): string {
    let html = raw
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/^### (.+)$/gm, "<h3>$1</h3>")
      .replace(/^## (.+)$/gm, "<h2>$1</h2>")
      .replace(/^# (.+)$/gm, "<h1>$1</h1>")
      .replace(/\*\*\*(.+?)\*\*\*/g, "<strong><em>$1</em></strong>")
      .replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>")
      .replace(/\*(.+?)\*/g, "<em>$1</em>")
      .replace(/`([^`]+)`/g, "<code>$1</code>")
      .replace(/^- (.+)$/gm, "<li>$1</li>")
      .replace(/(<li>.*<\/li>\n?)+/g, (match) => `<ul>${match}</ul>`)
      .replace(/(?<!>)\n(?!<)/g, "<br>");
    return html;
  }

  const previewHtml = $derived(simpleMarkdown(chatInputStore.text));

  // 1.5.2 – filtered mention items per tab
  const filteredMentionFiles = $derived.by(() => {
    if (!mentionQuery) return mentionFiles.slice(0, 20);
    const q = mentionQuery.toLowerCase();
    return mentionFiles.filter((f) => f.name.toLowerCase().includes(q)).slice(0, 20);
  });

  const filteredMentionSkills = $derived.by(() => {
    if (!mentionQuery) return mentionSkills.slice(0, 20);
    const q = mentionQuery.toLowerCase();
    return mentionSkills
      .filter(
        (s) =>
          s.name.toLowerCase().includes(q) ||
          s.description.toLowerCase().includes(q)
      )
      .slice(0, 20);
  });

  const filteredMentionAgents = $derived.by(() => {
    if (!mentionQuery) return acpStore.agents.slice(0, 20);
    const q = mentionQuery.toLowerCase();
    return acpStore.agents.filter((a) => a.name.toLowerCase().includes(q)).slice(0, 20);
  });

  const filteredMentionConversations = $derived.by(() => {
    if (!mentionQuery) return chatStore.conversations.slice(0, 20);
    const q = mentionQuery.toLowerCase();
    return chatStore.conversations
      .filter((c) => c.title.toLowerCase().includes(q))
      .slice(0, 20);
  });

  const currentTabItems = $derived.by((): Array<{ label: string; sub?: string; insertValue: string }> => {
    switch (mentionTab) {
      case "files":
        return filteredMentionFiles.map((f) => ({
          label: f.name,
          sub: f.kind === "dir" ? "directory" : "file",
          insertValue: `@file:${f.name}`,
        }));
      case "skills":
        return filteredMentionSkills.map((s) => ({
          label: s.name,
          sub: s.description,
          insertValue: `@skill:${s.name}`,
        }));
      case "agents":
        return filteredMentionAgents.map((a) => ({
          label: a.name,
          sub: a.agent_id,
          insertValue: `@agent:${a.name}`,
        }));
      case "conversations":
        return filteredMentionConversations.map((c) => ({
          label: c.title,
          insertValue: `@conversation:${c.title}`,
        }));
      default:
        return [];
    }
  });

  // ── Effects ────────────────────────────────────────────────────────────────

  $effect(() => {
    // Notify parent when container height changes
    if (!containerEl || !onHeightChange) return;
    const ro = new ResizeObserver(() => {
      onHeightChange(containerEl!.offsetHeight);
    });
    ro.observe(containerEl);
    return () => ro.disconnect();
  });

  // Reset highlight when tab or items change
  $effect(() => {
    // Access reactive deps
    mentionTab;
    mentionQuery;
    mentionHighlight = 0;
  });

  // ── Functions ──────────────────────────────────────────────────────────────

  function autoResize() {
    if (!textareaEl) return;
    textareaEl.style.height = "auto";
    const lineHeight = 20;
    const minHeight = 2 * lineHeight; // 2 lines
    const maxHeight = 20 * lineHeight; // 20 lines
    const newHeight = Math.min(Math.max(textareaEl.scrollHeight, minHeight), maxHeight);
    textareaEl.style.height = newHeight + "px";
  }

  async function handleSend() {
    if (!canSend) return;
    await chatInputStore.sendMessage(conversationId);
    if (textareaEl) textareaEl.style.height = "auto";
  }

  function handleStop() {
    chatInputStore.cancel();
  }

  function handleKeydown(e: KeyboardEvent) {
    // Handle mention dropdown navigation first
    if (mentionOpen) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        mentionHighlight = Math.min(mentionHighlight + 1, currentTabItems.length - 1);
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        mentionHighlight = Math.max(mentionHighlight - 1, 0);
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        const item = currentTabItems[mentionHighlight];
        if (item) insertMention(item.insertValue);
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        closeMentionDropdown();
        return;
      }
      if (e.key === "Tab") {
        e.preventDefault();
        cycleMentionTab();
        return;
      }
    }

    if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      handleSend();
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      chatInputStore.clearAll();
      if (textareaEl) textareaEl.style.height = "auto";
    }
  }

  function handleTextareaInput() {
    autoResize();
    detectMentionTrigger();
  }

  // ── 1.5.2 @-mention helpers ─────────────────────────────────────────────────

  function detectMentionTrigger() {
    if (!textareaEl) return;
    const text = chatInputStore.text;
    const cursor = textareaEl.selectionStart ?? text.length;
    // Find the last @ before the cursor that is followed only by word chars (no spaces)
    const beforeCursor = text.slice(0, cursor);
    const match = beforeCursor.match(/@(\w*)$/);
    if (match) {
      mentionQuery = match[1];
      if (!mentionOpen) {
        openMentionDropdown();
      }
    } else {
      if (mentionOpen) closeMentionDropdown();
    }
  }

  async function openMentionDropdown() {
    mentionOpen = true;
    mentionHighlight = 0;
    // Load files and skills lazily when first opened
    if (mentionFiles.length === 0) {
      try {
        const entries = await invoke<FsEntry[]>("fs_list", { path: "../../" });
        mentionFiles = entries;
      } catch (e) {
        console.error("[ChatInputArea] fs_list failed", e);
      }
    }
    if (mentionSkills.length === 0) {
      try {
        const catalog = await invoke<SkillCatalogEntry[]>("skill_catalog");
        mentionSkills = catalog;
      } catch (e) {
        console.error("[ChatInputArea] skill_catalog failed", e);
      }
    }
  }

  function closeMentionDropdown() {
    mentionOpen = false;
    mentionQuery = "";
  }

  function cycleMentionTab() {
    const tabs: Array<"files" | "skills" | "agents" | "conversations"> = [
      "files",
      "skills",
      "agents",
      "conversations",
    ];
    const idx = tabs.indexOf(mentionTab);
    mentionTab = tabs[(idx + 1) % tabs.length];
  }

  function insertMention(value: string) {
    if (!textareaEl) return;
    const text = chatInputStore.text;
    const cursor = textareaEl.selectionStart ?? text.length;
    const beforeCursor = text.slice(0, cursor);
    // Replace the trailing @word with the insertion value
    const replaced = beforeCursor.replace(/@(\w*)$/, value + " ");
    const afterCursor = text.slice(cursor);
    chatInputStore.text = replaced + afterCursor;
    closeMentionDropdown();
    // Restore focus + move cursor after inserted text
    requestAnimationFrame(() => {
      if (textareaEl) {
        textareaEl.focus();
        const newPos = replaced.length;
        textareaEl.setSelectionRange(newPos, newPos);
      }
    });
  }

  function handleTextareaClick() {
    detectMentionTrigger();
  }

  function handleTextareaBlur() {
    // Small delay so click on dropdown isn't swallowed
    setTimeout(() => {
      if (mentionOpen) closeMentionDropdown();
    }, 150);
  }

  // ── Model selector ─────────────────────────────────────────────────────────

  function selectModel(option: string) {
    if (option === "custom...") {
      const custom = window.prompt("Enter model name:");
      if (custom && custom.trim()) {
        chatInputStore.model = custom.trim();
      }
    } else {
      chatInputStore.model = option;
    }
    modelDropdownOpen = false;
  }

  // ── File attachment ────────────────────────────────────────────────────────

  function triggerFileInput() {
    fileInputEl?.click();
  }

  async function handleFileSelect(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const files = input.files;
    if (!files) return;

    for (const file of Array.from(files)) {
      if (chatInputStore.attachments.length >= 10) break;

      try {
        if (file.type.startsWith("image/")) {
          const base64 = await readFileAsBase64(file);
          chatInputStore.addAttachment(file.name, base64, file.type);
        } else {
          const text = await readFileAsText(file);
          chatInputStore.addAttachment(file.name, text, file.type || "text/plain");
        }
      } catch (err) {
        console.error("[ChatInputArea] Failed to read file", file.name, err);
      }
    }

    input.value = "";
  }

  function readFileAsText(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = () => reject(reader.error);
      reader.readAsText(file);
    });
  }

  function readFileAsBase64(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => {
        const result = reader.result as string;
        const base64 = result.split(",")[1] ?? result;
        resolve(base64);
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
  }

  // ── 1.5.4 / 1.5.19 Prompt enhancer ────────────────────────────────────────

  function openEnhancer() {
    if (!chatInputStore.text.trim()) return;
    enhancedText = chatInputStore.text;
    showEnhancer = true;
  }

  function useEnhanced() {
    chatInputStore.text = enhancedText;
    showEnhancer = false;
    enhancedText = "";
  }

  function sendOriginal() {
    showEnhancer = false;
    enhancedText = "";
  }

  // ── 1.5.14 Context dropdown – clear messages ───────────────────────────────

  function clearContext() {
    chatInputStore.clearMessages();
    contextDropdownOpen = false;
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div
  class="chat-input-area"
  bind:this={containerEl}
  onclick={(e) => {
    const target = e.target as HTMLElement;
    if (!target.closest(".model-selector")) modelDropdownOpen = false;
    if (!target.closest(".context-selector")) contextDropdownOpen = false;
  }}
>
  <!-- 1.5.19 – Enhance preview card (shown above input when enhancer is open) -->
  {#if showEnhancer}
    <div class="enhancer-card">
      <div class="enhancer-header">
        <span class="enhancer-title">Prompt Enhancer</span>
        <button class="enhancer-close" onclick={() => { showEnhancer = false; enhancedText = ""; }}>
          <X size={12} strokeWidth={2} />
        </button>
      </div>
      <div class="enhancer-body">
        <div class="enhancer-section">
          <span class="enhancer-label">Original</span>
          <div class="enhancer-original">{chatInputStore.text}</div>
        </div>
        <div class="enhancer-section">
          <span class="enhancer-label">Enhanced</span>
          <textarea
            class="enhancer-textarea"
            bind:value={enhancedText}
            placeholder="Enhanced version will appear here — edit as needed"
            rows={4}
          ></textarea>
        </div>
      </div>
      <div class="enhancer-actions">
        <button class="enhancer-btn-primary" onclick={useEnhanced}>Use enhanced</button>
        <button class="enhancer-btn-secondary" onclick={sendOriginal}>Send original</button>
      </div>
    </div>
  {/if}

  <!-- Attachment chips row -->
  {#if chatInputStore.attachments.length > 0}
    <div class="attachment-row">
      {#each chatInputStore.attachments as attachment, i (i)}
        <div class="attachment-chip">
          <span class="attachment-chip-name">{attachment.name}</span>
          <button
            class="attachment-chip-remove"
            onclick={() => chatInputStore.removeAttachment(i)}
            aria-label="Remove {attachment.name}"
          >
            <X size={10} strokeWidth={2} />
          </button>
        </div>
      {/each}
    </div>
  {/if}

  <!-- @-mention chips (rendered from text) -->
  {#if mentionChips.length > 0}
    <div class="mention-row">
      {#each mentionChips as chip (chip)}
        <span class="mention-chip">{chip}</span>
      {/each}
    </div>
  {/if}

  <!-- Main input row (with relative positioning for mention dropdown) -->
  <div class="input-wrapper">
    <!-- 1.5.2 – @-mention autocomplete dropdown (positioned above textarea) -->
    {#if mentionOpen}
      <div class="mention-dropdown" bind:this={mentionDropdownEl}>
        <!-- Tab bar -->
        <div class="mention-tabs">
          {#each (["files", "skills", "agents", "conversations"] as const) as tab (tab)}
            <button
              class="mention-tab"
              class:mention-tab--active={mentionTab === tab}
              onclick={() => { mentionTab = tab; mentionHighlight = 0; }}
            >
              {tab.charAt(0).toUpperCase() + tab.slice(1)}
            </button>
          {/each}
        </div>

        <!-- Items list -->
        <div class="mention-list">
          {#if currentTabItems.length === 0}
            <div class="mention-empty">No {mentionTab} found{mentionQuery ? ` for "${mentionQuery}"` : ""}</div>
          {:else}
            {#each currentTabItems as item, idx (item.insertValue)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <div
                class="mention-item"
                class:mention-item--active={mentionHighlight === idx}
                onclick={() => insertMention(item.insertValue)}
                onmouseenter={() => { mentionHighlight = idx; }}
                role="option"
                aria-selected={mentionHighlight === idx}
              >
                <span class="mention-item-label">{item.label}</span>
                {#if item.sub}
                  <span class="mention-item-sub">{item.sub}</span>
                {/if}
                <ChevronRight size={10} class="mention-item-arrow" />
              </div>
            {/each}
          {/if}
        </div>

        <div class="mention-footer">
          <span>Tab to switch section</span>
          <span>Enter to select</span>
          <span>Esc to close</span>
        </div>
      </div>
    {/if}

    <div class="input-row">
      <!-- 1.5.11 – show preview OR textarea -->
      {#if showPreview}
        <div class="markdown-preview">
          {#if chatInputStore.text.trim()}
            {@html previewHtml}
          {:else}
            <span class="preview-placeholder">Nothing to preview yet — type something first</span>
          {/if}
        </div>
      {:else}
        <textarea
          bind:this={textareaEl}
          bind:value={chatInputStore.text}
          oninput={handleTextareaInput}
          onkeydown={handleKeydown}
          onclick={handleTextareaClick}
          onblur={handleTextareaBlur}
          placeholder={conversationId ? "Message… (Ctrl+Enter to send, @ to mention)" : "Select a conversation first"}
          disabled={!conversationId}
          rows={2}
          class="main-textarea"
        ></textarea>
      {/if}

      <!-- Send / Stop button -->
      <div class="send-col">
        {#if chatInputStore.isStreaming}
          <button class="stop-btn" onclick={handleStop} title="Stop generation">
            <Square size={14} strokeWidth={1.5} />
            <span>Stop</span>
          </button>
        {:else}
          <button
            class="send-btn"
            onclick={handleSend}
            disabled={!canSend}
            title="Send (Ctrl+Enter)"
          >
            <ArrowUp size={14} strokeWidth={2} />
            <span>Send</span>
          </button>
        {/if}
      </div>
    </div>
  </div>

  <!-- Bottom toolbar -->
  <div class="input-toolbar">
    <!-- Model selector -->
    <div class="model-selector">
      <button
        class="model-badge"
        onclick={() => (modelDropdownOpen = !modelDropdownOpen)}
        title="Select model"
      >
        <span class="model-badge-text">{chatInputStore.model}</span>
        <ChevronDown size={10} strokeWidth={1.5} />
      </button>
      {#if modelDropdownOpen}
        <div class="model-dropdown">
          {#each modelOptions as opt (opt)}
            <button
              class="model-option"
              class:model-option--active={chatInputStore.model === opt}
              onclick={() => selectModel(opt)}
            >
              {opt}
            </button>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Attach file button -->
    <button class="toolbar-icon-btn" onclick={triggerFileInput} title="Attach files (max 10)">
      <Paperclip size={13} strokeWidth={1.5} />
    </button>
    <input
      bind:this={fileInputEl}
      type="file"
      multiple
      class="file-input-hidden"
      onchange={handleFileSelect}
    />

    <!-- 1.5.11 – Markdown preview toggle -->
    <button
      class="toolbar-ghost-btn"
      class:toolbar-ghost-btn--active={showPreview}
      onclick={() => (showPreview = !showPreview)}
      title={showPreview ? "Switch back to editor" : "Preview markdown"}
    >
      {#if showPreview}
        <Edit2 size={11} strokeWidth={1.5} />
        <span>Edit</span>
      {:else}
        <Eye size={11} strokeWidth={1.5} />
        <span>Preview</span>
      {/if}
    </button>

    <!-- 1.5.4 – Prompt enhancer button -->
    <button
      class="toolbar-ghost-btn"
      onclick={openEnhancer}
      disabled={!chatInputStore.text.trim()}
      title="Enhance prompt"
    >
      <Zap size={11} strokeWidth={1.5} />
      <span>Enhance</span>
    </button>

    <!-- 1.5.14 – Context dropdown -->
    <div class="context-selector">
      <button
        class="toolbar-ghost-btn"
        onclick={() => (contextDropdownOpen = !contextDropdownOpen)}
        title="Context settings"
      >
        <span>Context</span>
        <ChevronDown size={10} strokeWidth={1.5} />
      </button>
      {#if contextDropdownOpen}
        <div class="context-dropdown">
          <label class="context-row context-row--toggle">
            <input
              type="checkbox"
              bind:checked={chatInputStore.autoCompress}
              class="context-checkbox"
            />
            <span class="context-row-label">Auto-compress context</span>
          </label>
          <div class="context-divider"></div>
          <button class="context-row context-row--btn" onclick={clearContext}>
            Clear context
          </button>
          <div class="context-divider"></div>
          <div class="context-row context-row--info">
            <span class="context-row-label">Estimated tokens</span>
            <span class="context-token-count">~{estimatedTokens.toLocaleString()}</span>
          </div>
        </div>
      {/if}
    </div>

    <div class="toolbar-spacer"></div>

    <!-- Streaming indicator -->
    {#if chatInputStore.isStreaming}
      <span class="streaming-indicator" title="Streaming">
        <Loader size={14} strokeWidth={1} class="spinner-icon" />
      </span>
    {/if}

    <!-- Token estimate -->
    <span class="token-estimate" title="Estimated context size (rough, ~4 chars/token)">
      ~{estimatedTokens.toLocaleString()} tokens (estimated)
    </span>
  </div>
</div>

<style>
  .chat-input-area {
    border-top: 1px solid var(--border);
    background: var(--bg);
    padding: var(--space-3) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  /* ── 1.5.19 Enhancer card ─────────────────────────────────────────────── */
  .enhancer-card {
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg-elevated);
    display: flex;
    flex-direction: column;
    gap: 0;
    overflow: hidden;
  }

  .enhancer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .enhancer-title {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--text-secondary);
  }

  .enhancer-close {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-tertiary);
    display: flex;
    align-items: center;
    padding: 2px;
    border-radius: 2px;
  }

  .enhancer-close:hover {
    color: var(--text);
    background: var(--bg-tertiary);
  }

  .enhancer-body {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    padding: var(--space-3);
  }

  .enhancer-section {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .enhancer-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-tertiary);
  }

  .enhancer-original {
    font-size: var(--text-sm);
    color: var(--text-secondary);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 6px 8px;
    white-space: pre-wrap;
    word-break: break-word;
    line-height: 1.5;
    max-height: 80px;
    overflow-y: auto;
  }

  .enhancer-textarea {
    font-size: var(--text-sm);
    color: var(--text);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 6px 8px;
    font-family: var(--font-sans);
    resize: vertical;
    line-height: 1.5;
    outline: none;
    min-height: 60px;
  }

  .enhancer-textarea:focus {
    border-color: var(--border-active);
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .enhancer-actions {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-top: 1px solid var(--border);
    background: var(--bg-secondary);
  }

  .enhancer-btn-primary {
    background: var(--accent);
    color: var(--text-inverse);
    border: none;
    border-radius: 2px;
    padding: 4px 12px;
    font-size: var(--text-sm);
    font-family: var(--font-sans);
    cursor: pointer;
  }

  .enhancer-btn-primary:hover {
    background: var(--accent-hover);
  }

  .enhancer-btn-secondary {
    background: none;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 4px 12px;
    font-size: var(--text-sm);
    font-family: var(--font-sans);
    cursor: pointer;
  }

  .enhancer-btn-secondary:hover {
    color: var(--text);
    border-color: var(--border-active);
    background: var(--bg-tertiary);
  }

  /* Attachment chips */
  .attachment-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .attachment-chip {
    display: flex;
    align-items: center;
    gap: 4px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 2px 6px;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-secondary);
    max-width: 160px;
  }

  .attachment-chip-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .attachment-chip-remove {
    background: none;
    border: none;
    cursor: pointer;
    padding: 0;
    display: flex;
    align-items: center;
    color: var(--text-tertiary);
    flex-shrink: 0;
  }

  .attachment-chip-remove:hover {
    color: var(--accent-danger);
  }

  /* @-mention chips */
  .mention-row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }

  .mention-chip {
    background: var(--bg-tertiary);
    border: 1px solid var(--border-active);
    border-radius: 2px;
    padding: 1px 6px;
    font-size: 11px;
    font-weight: 500;
    color: var(--accent);
  }

  /* Input wrapper (relative for dropdown positioning) */
  .input-wrapper {
    position: relative;
  }

  /* ── 1.5.2 @-mention dropdown ──────────────────────────────────────────── */
  .mention-dropdown {
    position: absolute;
    bottom: calc(100% + 4px);
    left: 0;
    right: 60px; /* leave room for send button */
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: var(--shadow-1);
    z-index: 300;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    max-height: 240px;
  }

  .mention-tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .mention-tab {
    flex: 1;
    background: none;
    border: none;
    border-right: 1px solid var(--border);
    padding: 5px 4px;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-tertiary);
    cursor: pointer;
    font-family: var(--font-sans);
  }

  .mention-tab:last-child {
    border-right: none;
  }

  .mention-tab:hover {
    color: var(--text);
    background: var(--bg-tertiary);
  }

  .mention-tab--active {
    color: var(--accent);
    background: var(--bg-tertiary);
  }

  .mention-list {
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }

  .mention-empty {
    padding: 12px;
    font-size: var(--text-sm);
    color: var(--text-tertiary);
    text-align: center;
  }

  .mention-item {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 6px 10px;
    cursor: pointer;
    border-bottom: 1px solid var(--border);
  }

  .mention-item:last-child {
    border-bottom: none;
  }

  .mention-item:hover,
  .mention-item--active {
    background: var(--bg-tertiary);
  }

  .mention-item-label {
    font-size: var(--text-sm);
    color: var(--text);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 0;
    max-width: 160px;
  }

  .mention-item-sub {
    font-size: 11px;
    color: var(--text-tertiary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    flex: 1;
  }

  :global(.mention-item-arrow) {
    color: var(--text-tertiary);
    flex-shrink: 0;
    margin-left: auto;
  }

  .mention-footer {
    display: flex;
    gap: var(--space-3);
    padding: 4px 8px;
    border-top: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }

  .mention-footer span {
    font-size: 10px;
    color: var(--text-tertiary);
  }

  /* Input row */
  .input-row {
    display: flex;
    align-items: flex-end;
    gap: var(--space-2);
  }

  .main-textarea {
    flex: 1;
    min-height: 40px;
    max-height: 400px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 2px;
    color: var(--text);
    font-family: var(--font-sans);
    font-size: var(--text-md);
    padding: 8px 12px;
    resize: none;
    outline: none;
    line-height: 20px;
  }

  .main-textarea:focus {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
    border-color: var(--border-active);
  }

  .main-textarea:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .main-textarea::placeholder {
    color: var(--text-tertiary);
  }

  /* 1.5.11 – Markdown preview area (replaces textarea) */
  .markdown-preview {
    flex: 1;
    min-height: 40px;
    max-height: 400px;
    overflow-y: auto;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-active);
    border-radius: 2px;
    padding: 8px 12px;
    font-size: var(--text-md);
    color: var(--text);
    line-height: 1.6;
  }

  .markdown-preview :global(h1) { font-size: 16px; font-weight: 700; margin: 0 0 6px; }
  .markdown-preview :global(h2) { font-size: 14px; font-weight: 700; margin: 8px 0 4px; }
  .markdown-preview :global(h3) { font-size: 13px; font-weight: 700; margin: 6px 0 3px; }
  .markdown-preview :global(strong) { font-weight: 700; }
  .markdown-preview :global(em) { font-style: italic; color: var(--text-secondary); }
  .markdown-preview :global(code) {
    font-family: var(--font-mono);
    font-size: 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 1px 4px;
    color: var(--accent);
  }
  .markdown-preview :global(ul) { margin: 4px 0; padding-left: 18px; }
  .markdown-preview :global(li) { margin: 2px 0; color: var(--text-secondary); }

  .preview-placeholder {
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }

  /* Send column */
  .send-col {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    flex-shrink: 0;
  }

  .send-btn {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    background: var(--accent);
    color: var(--text-inverse);
    border: none;
    border-radius: 2px;
    padding: 6px 12px;
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    cursor: pointer;
    white-space: nowrap;
  }

  .send-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .send-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .stop-btn {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    background: none;
    border: 1px solid var(--accent-danger);
    color: var(--accent-danger);
    border-radius: 2px;
    padding: 6px 12px;
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    cursor: pointer;
    white-space: nowrap;
  }

  .stop-btn:hover {
    background: var(--accent-danger);
    color: var(--text-inverse);
  }

  /* Bottom toolbar */
  .input-toolbar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .toolbar-spacer {
    flex: 1;
  }

  /* Model selector */
  .model-selector {
    position: relative;
  }

  .model-badge {
    display: flex;
    align-items: center;
    gap: 3px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 2px 6px;
    cursor: pointer;
    font-family: var(--font-mono);
    color: var(--text-tertiary);
  }

  .model-badge:hover {
    color: var(--text);
    border-color: var(--border-active);
  }

  .model-badge-text {
    font-size: 11px;
    font-weight: 500;
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .model-dropdown {
    position: absolute;
    bottom: calc(100% + 4px);
    left: 0;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 2px;
    box-shadow: var(--shadow-1);
    min-width: 180px;
    z-index: 200;
    padding: 2px 0;
  }

  .model-option {
    display: block;
    width: 100%;
    text-align: left;
    background: none;
    border: none;
    padding: 6px 12px;
    font-size: var(--text-sm);
    font-family: var(--font-mono);
    color: var(--text);
    cursor: pointer;
  }

  .model-option:hover {
    background: var(--bg-tertiary);
  }

  .model-option--active {
    color: var(--accent);
  }

  /* Toolbar icon button */
  .toolbar-icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--text-tertiary);
    padding: 2px;
    border-radius: 2px;
  }

  .toolbar-icon-btn:hover {
    color: var(--text);
    background: var(--bg-tertiary);
  }

  /* Ghost toolbar buttons (with text labels) */
  .toolbar-ghost-btn {
    display: flex;
    align-items: center;
    gap: 3px;
    background: none;
    border: 1px solid var(--border);
    border-radius: 2px;
    padding: 2px 6px;
    font-size: 11px;
    font-family: var(--font-sans);
    color: var(--text-tertiary);
    cursor: pointer;
    white-space: nowrap;
  }

  .toolbar-ghost-btn:hover:not(:disabled) {
    color: var(--text);
    border-color: var(--border-active);
    background: var(--bg-tertiary);
  }

  .toolbar-ghost-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .toolbar-ghost-btn--active {
    border-color: var(--accent);
    color: var(--accent);
  }

  /* ── 1.5.14 Context dropdown ──────────────────────────────────────────── */
  .context-selector {
    position: relative;
  }

  .context-dropdown {
    position: absolute;
    bottom: calc(100% + 4px);
    left: 0;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: var(--shadow-1);
    min-width: 200px;
    z-index: 200;
    padding: 4px 0;
  }

  .context-row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 6px 12px;
    width: 100%;
  }

  .context-row--toggle {
    cursor: pointer;
  }

  .context-row--btn {
    background: none;
    border: none;
    cursor: pointer;
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    color: var(--accent-danger);
    text-align: left;
  }

  .context-row--btn:hover {
    background: var(--bg-tertiary);
  }

  .context-row--info {
    justify-content: space-between;
  }

  .context-row-label {
    font-size: var(--text-sm);
    color: var(--text);
  }

  .context-checkbox {
    accent-color: var(--accent);
    width: 13px;
    height: 13px;
    cursor: pointer;
    flex-shrink: 0;
  }

  .context-token-count {
    font-size: var(--text-sm);
    color: var(--text-tertiary);
    font-family: var(--font-mono);
  }

  .context-divider {
    height: 1px;
    background: var(--border);
    margin: 2px 0;
  }

  /* Hidden file input */
  .file-input-hidden {
    display: none;
  }

  /* Streaming indicator */
  .streaming-indicator {
    display: flex;
    align-items: center;
    color: var(--text-tertiary);
  }

  /* CSS-only spinner: rotate the Loader icon via animation */
  :global(.spinner-icon) {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }

  /* Token estimate */
  .token-estimate {
    font-size: 11px;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    white-space: nowrap;
  }
</style>
