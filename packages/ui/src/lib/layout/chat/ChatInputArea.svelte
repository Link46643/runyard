<script lang="ts">
  import { ArrowUp, Square, Paperclip, X, ChevronDown, Loader } from "lucide-svelte";
  import { chatInputStore } from "../../stores/chatInputStore.svelte.js";
  import { chatStore } from "../../stores/chatStore.svelte.js";

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

  // @-mention chips: detect @word tokens in the text (stub - no autocomplete lookup)
  const mentionChips = $derived.by(() => {
    const matches = chatInputStore.text.match(/@\w+/g) ?? [];
    return [...new Set(matches)];
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

  function selectModel(option: string) {
    if (option === "custom...") {
      // Stub: show custom input - for now just open a prompt
      const custom = window.prompt("Enter model name:");
      if (custom && custom.trim()) {
        chatInputStore.model = custom.trim();
      }
    } else {
      chatInputStore.model = option;
    }
    modelDropdownOpen = false;
  }

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
          // Read images as base64
          const base64 = await readFileAsBase64(file);
          chatInputStore.addAttachment(file.name, base64, file.type);
        } else {
          // Read text files as plain text
          const text = await readFileAsText(file);
          chatInputStore.addAttachment(file.name, text, file.type || "text/plain");
        }
      } catch (err) {
        console.error("[ChatInputArea] Failed to read file", file.name, err);
      }
    }

    // Reset the input so the same file can be re-selected
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
        // Strip the data URL prefix, keep only base64 content
        const base64 = result.split(",")[1] ?? result;
        resolve(base64);
      };
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
  }

  function dismissModelDropdown(e: MouseEvent) {
    if (modelDropdownOpen) {
      modelDropdownOpen = false;
    }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div
  class="chat-input-area"
  bind:this={containerEl}
  onclick={(e) => {
    // Close model dropdown when clicking outside it
    const target = e.target as HTMLElement;
    if (!target.closest(".model-selector")) {
      modelDropdownOpen = false;
    }
  }}
>
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

  <!-- @-mention chips (stub rendering) -->
  {#if mentionChips.length > 0}
    <div class="mention-row">
      {#each mentionChips as chip (chip)}
        <span class="mention-chip">{chip}</span>
      {/each}
    </div>
  {/if}

  <!-- Main input row -->
  <div class="input-row">
    <textarea
      bind:this={textareaEl}
      bind:value={chatInputStore.text}
      oninput={autoResize}
      onkeydown={handleKeydown}
      placeholder={conversationId ? "Message... (Ctrl+Enter to send)" : "Select a conversation first"}
      disabled={!conversationId}
      rows={2}
      class="main-textarea"
    ></textarea>

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

    <!-- Prompt enhancer stub -->
    <!-- TODO 1.5.4: wire LLM-based prompt enhancer here -->

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
