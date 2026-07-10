<script lang="ts">
  import type { Message } from "@runyard/common";
  import { ArrowDown, ArrowUp } from "lucide-svelte";
  import ChatMessage from "./ChatMessage.svelte";
  import { tick } from "svelte";

  let {
    messages,
    onOpenFile,
    scrollToMessageId,
  }: { messages: Message[]; onOpenFile?: (path: string) => void; scrollToMessageId?: string | null } = $props();

  let container: HTMLDivElement;
  let isNearBottom = $state(true);
  let showJumpToTop = $state(false);
  let lastMessageCount = 0;
  let highlightedId = $state<string | null>(null);

  function handleScroll() {
    if (!container) return;
    const distanceFromBottom = container.scrollHeight - container.scrollTop - container.clientHeight;
    isNearBottom = distanceFromBottom < 120;
    showJumpToTop = container.scrollTop > 800;
  }

  function scrollToBottom(smooth = true) {
    if (!container) return;
    container.scrollTo({ top: container.scrollHeight, behavior: smooth ? "smooth" : "auto" });
  }

  function scrollToTop() {
    if (!container) return;
    container.scrollTo({ top: 0, behavior: "smooth" });
  }

  $effect(() => {
    const count = messages.length;
    if (count !== lastMessageCount) {
      lastMessageCount = count;
      if (isNearBottom) {
        tick().then(() => scrollToBottom(false));
      }
    }
  });

  $effect(() => {
    const targetId = scrollToMessageId;
    if (!targetId) return;
    tick().then(() => {
      const el = document.getElementById(`message-${targetId}`);
      if (el) {
        el.scrollIntoView({ behavior: "smooth", block: "center" });
        highlightedId = targetId;
        setTimeout(() => (highlightedId = null), 1500);
      }
    });
  });
</script>

<div class="message-list" bind:this={container} onscroll={handleScroll}>
  {#if messages.length === 0}
    <div class="empty-state">No messages yet. Send one to get started.</div>
  {/if}
  {#each messages as message (message.id)}
    <div class="message-wrapper" id={`message-${message.id}`} class:highlighted={highlightedId === message.id}>
      <ChatMessage {message} {onOpenFile} />
    </div>
  {/each}
</div>

{#if showJumpToTop}
  <button class="float-btn top" onclick={scrollToTop} title="Jump to top">
    <ArrowUp size={16} strokeWidth={1.5} />
  </button>
{/if}
{#if !isNearBottom && messages.length > 0}
  <button class="float-btn bottom" onclick={() => scrollToBottom(true)} title="Scroll to bottom">
    <ArrowDown size={16} strokeWidth={1.5} />
  </button>
{/if}

<style>
  .message-list {
    flex: 1;
    overflow-y: auto;
    position: relative;
    background: var(--chat-bg);
  }
  .message-wrapper {
    /* Lazy rendering: skip layout/paint work for messages far outside the
       viewport. Real performance benefit, no custom windowing math to get wrong. */
    content-visibility: auto;
    contain-intrinsic-size: auto 120px;
    transition: background-color 300ms ease;
  }
  .message-wrapper.highlighted {
    background-color: color-mix(in srgb, var(--accent) 12%, transparent);
  }
  .empty-state {
    padding: var(--space-8);
    text-align: center;
    color: var(--text-tertiary);
    font-size: var(--text-base);
  }
  .float-btn {
    position: absolute;
    right: var(--space-5);
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius-full);
    width: 32px;
    height: 32px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
    cursor: pointer;
    box-shadow: var(--shadow-1);
  }
  .float-btn:hover {
    color: var(--text);
    border-color: var(--border-secondary);
  }
  .float-btn.top {
    top: 56px;
  }
  .float-btn.bottom {
    bottom: 16px;
  }
</style>
