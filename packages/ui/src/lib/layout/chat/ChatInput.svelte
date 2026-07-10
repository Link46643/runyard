<script lang="ts">
  import { chatStore } from "../../stores/chatStore.svelte.js";
  import { Square, RotateCcw } from "lucide-svelte";

  let draft = $state("");
  let textareaEl: HTMLTextAreaElement;

  function autoResize() {
    if (!textareaEl) return;
    textareaEl.style.height = "auto";
    const maxHeight = 20 * 20; // ~20 lines at ~20px line-height
    textareaEl.style.height = Math.min(textareaEl.scrollHeight, maxHeight) + "px";
  }

  async function send() {
    const text = draft.trim();
    if (!text || !chatStore.activeConversationId) return;
    draft = "";
    if (textareaEl) textareaEl.style.height = "auto";
    await chatStore.sendMessage([{ type: "text", text }]);
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      send();
    }
  }

  const lastMessage = $derived(chatStore.messages[chatStore.messages.length - 1]);
  const canRegenerate = $derived(lastMessage?.role === "assistant");

  async function regenerate() {
    if (!lastMessage) return;
    // Removes the last assistant message so a fresh one can be produced.
    // There is no agent wired up yet (section 1.7) to actually produce the
    // replacement, so this is a real, working half of the feature - the
    // "replace" side is honestly incomplete until an agent exists.
    await chatStore.deleteMessage(lastMessage.id);
  }
</script>

<div class="chat-input-bar">
  {#if canRegenerate}
    <div class="regenerate-row">
      <button class="regen-btn" onclick={regenerate}>
        <RotateCcw size={12} strokeWidth={1.5} /> Regenerate
      </button>
    </div>
  {/if}
  <div class="input-row">
    <textarea
      bind:this={textareaEl}
      bind:value={draft}
      oninput={autoResize}
      onkeydown={handleKeydown}
      placeholder={chatStore.activeConversationId ? "Message..." : "Select or create a conversation first"}
      disabled={!chatStore.activeConversationId}
      rows="2"
    ></textarea>
  </div>
  <div class="input-toolbar">
    <span class="char-count">{draft.length}</span>
    <div class="spacer"></div>
    {#if chatStore.isStreaming}
      <button class="stop-btn" onclick={() => (chatStore.isStreaming = false)}>
        <Square size={12} strokeWidth={1.5} /> Stop generation
      </button>
    {:else}
      <button class="send-btn" onclick={send} disabled={!draft.trim() || !chatStore.activeConversationId}>
        Send
        <span class="shortcut">Ctrl+Enter</span>
      </button>
    {/if}
  </div>
</div>

<style>
  .chat-input-bar {
    border-top: 1px solid var(--border);
    background: var(--bg);
    padding: var(--space-3) var(--space-4);
  }
  .regenerate-row {
    display: flex;
    justify-content: center;
    margin-bottom: var(--space-2);
  }
  .regen-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    border-radius: var(--radius-1);
    padding: 4px 10px;
    font-family: var(--font-sans);
    font-size: var(--text-xs);
    cursor: pointer;
  }
  .regen-btn:hover {
    color: var(--text);
  }
  .input-row textarea {
    width: 100%;
    min-height: 48px;
    max-height: 400px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--radius-1);
    color: var(--text);
    font-family: var(--font-sans);
    font-size: var(--text-md);
    padding: var(--space-3);
    resize: none;
    outline: none;
  }
  .input-row textarea:focus {
    border-color: var(--border-active);
  }
  .input-row textarea:disabled {
    opacity: 0.5;
  }
  .input-toolbar {
    display: flex;
    align-items: center;
    margin-top: var(--space-2);
  }
  .char-count {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
    font-family: var(--font-mono);
  }
  .spacer {
    flex: 1;
  }
  .send-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    background: var(--accent);
    color: var(--text-inverse);
    border: none;
    border-radius: var(--radius-1);
    padding: 6px 12px;
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .send-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .send-btn .shortcut {
    font-size: var(--text-xs);
    opacity: 0.8;
  }
  .stop-btn {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    border-radius: var(--radius-1);
    padding: 6px 12px;
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    cursor: pointer;
  }
</style>
