<script lang="ts">
  import type { Message } from "@runyard/common";
  import { Copy, Check, Pencil, GitBranch, Pin, PinOff } from "lucide-svelte";
  import ContentBlockRenderer from "./ContentBlockRenderer.svelte";
  import { chatStore } from "../../stores/chatStore.svelte.js";

  let { message, onOpenFile }: { message: Message; onOpenFile?: (path: string) => void } = $props();

  let editing = $state(false);
  let editText = $state("");
  let copied = $state(false);

  const roleLabel = $derived(
    message.role === "user" ? "You" : message.role === "system" ? "System" : "Assistant"
  );

  function relativeTime(ts: number): string {
    const diffMs = Date.now() - ts;
    const diffSec = Math.floor(diffMs / 1000);
    if (diffSec < 5) return "now";
    if (diffSec < 60) return `${diffSec}s ago`;
    const diffMin = Math.floor(diffSec / 60);
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffHr = Math.floor(diffMin / 60);
    if (diffHr < 24) return `${diffHr}h ago`;
    const diffDay = Math.floor(diffHr / 24);
    return `${diffDay}d ago`;
  }

  function plainTextOf(message: Message): string {
    return message.content
      .map((b) => {
        if (b.type === "text") return b.text;
        if (b.type === "code") return b.code;
        return "";
      })
      .filter(Boolean)
      .join("\n\n");
  }

  async function copyMessage() {
    try {
      await navigator.clipboard.writeText(plainTextOf(message));
      copied = true;
      setTimeout(() => (copied = false), 800);
    } catch (e) {
      console.error("[ChatMessage] Clipboard write failed", e);
    }
  }

  function startEdit() {
    editText = plainTextOf(message);
    editing = true;
  }

  async function saveEdit() {
    await chatStore.updateMessage(message.id, [{ type: "text", text: editText }]);
    editing = false;
  }

  async function branchFromHere() {
    const name = `Branch from ${new Date(message.created_at).toLocaleTimeString()}`;
    await chatStore.createBranch(name, message.id);
  }

  async function togglePin() {
    await chatStore.setMessagePinned(message.id, !message.is_pinned);
  }
</script>

<div class="message" class:is-user={message.role === "user"} class:is-pinned={message.is_pinned}>
  <div class="message-meta">
    <span class="role-label">{roleLabel}</span>
    <span class="timestamp" title={new Date(message.created_at).toLocaleString()}>{relativeTime(message.created_at)}</span>
    <div class="spacer"></div>
    <div class="message-actions">
      <button class="action-btn" onclick={copyMessage} title="Copy">
        {#if copied}<Check size={14} strokeWidth={1.5} />{:else}<Copy size={14} strokeWidth={1.5} />{/if}
      </button>
      {#if message.role === "user"}
        <button class="action-btn" onclick={startEdit} title="Edit"><Pencil size={14} strokeWidth={1.5} /></button>
      {/if}
      <button class="action-btn" onclick={branchFromHere} title="Branch from here"><GitBranch size={14} strokeWidth={1.5} /></button>
      <button class="action-btn" onclick={togglePin} title={message.is_pinned ? "Unpin" : "Pin"}>
        {#if message.is_pinned}<PinOff size={14} strokeWidth={1.5} />{:else}<Pin size={14} strokeWidth={1.5} />{/if}
      </button>
    </div>
  </div>

  {#if editing}
    <div class="edit-area">
      <textarea bind:value={editText} rows="4"></textarea>
      <div class="edit-actions">
        <button class="ghost-btn primary" onclick={saveEdit}>Save</button>
        <button class="ghost-btn" onclick={() => (editing = false)}>Cancel</button>
      </div>
    </div>
  {:else}
    <div class="message-content">
      <ContentBlockRenderer content={message.content} {onOpenFile} />
    </div>
  {/if}
</div>

<style>
  .message {
    padding: var(--space-4) var(--space-5);
    border-bottom: 1px solid var(--border);
    border-left: 2px solid transparent;
  }
  .message.is-user {
    border-left-color: var(--accent);
  }
  .message.is-pinned {
    background: color-mix(in srgb, var(--accent) 4%, transparent);
  }
  .message-meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }
  .role-label {
    font-size: var(--text-xs);
    font-weight: 600;
    color: var(--text-secondary);
  }
  .timestamp {
    font-size: var(--text-xs);
    color: var(--text-tertiary);
  }
  .spacer {
    flex: 1;
  }
  .message-actions {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 100ms ease;
  }
  .message:hover .message-actions {
    opacity: 1;
  }
  .action-btn {
    background: none;
    border: none;
    color: var(--text-tertiary);
    cursor: pointer;
    padding: 3px;
    border-radius: var(--radius-1);
    display: flex;
    align-items: center;
  }
  .action-btn:hover {
    color: var(--text);
    background: var(--bg-tertiary);
  }
  .message-content {
    font-size: var(--text-md);
  }
  .edit-area textarea {
    width: 100%;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-active);
    border-radius: var(--radius-1);
    color: var(--text);
    font-family: var(--font-sans);
    font-size: var(--text-md);
    padding: var(--space-3);
    resize: vertical;
  }
  .edit-actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .ghost-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--text-secondary);
    border-radius: var(--radius-1);
    padding: 4px 10px;
    font-family: var(--font-sans);
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .ghost-btn.primary {
    background: var(--accent);
    color: var(--text-inverse);
    border-color: var(--accent);
  }
</style>
