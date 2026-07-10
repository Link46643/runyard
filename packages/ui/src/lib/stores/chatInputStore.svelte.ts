import { acpStore } from "./acpStore.svelte.js";
import { chatStore } from "./chatStore.svelte.js";

// ── Types ────────────────────────────────────────────────────────────────────

interface Attachment {
  name: string;
  content: string;
  type: string;
}

// ── Store class ──────────────────────────────────────────────────────────────

class ChatInputStore {
  // State
  text = $state("");
  attachments = $state<Attachment[]>([]);
  isStreaming = $state(false);
  activeConnectionId = $state<string | null>(null);
  activeSessionId = $state<string | null>(null);
  model = $state("claude-sonnet-4-5");

  // ── Private: ACP event listeners ──────────────────────────────────────────

  private _chunkHandler: ((e: Event) => void) | null = null;
  private _completedHandler: ((e: Event) => void) | null = null;
  private _errorHandler: ((e: Event) => void) | null = null;

  private _removeStreamListeners() {
    if (this._chunkHandler) {
      window.removeEventListener("acp:agent_message_chunk", this._chunkHandler);
      this._chunkHandler = null;
    }
    if (this._completedHandler) {
      window.removeEventListener("acp:prompt_completed", this._completedHandler);
      this._completedHandler = null;
    }
    if (this._errorHandler) {
      window.removeEventListener("acp:error", this._errorHandler);
      this._errorHandler = null;
    }
  }

  private _attachStreamListeners() {
    this._removeStreamListeners();

    this._chunkHandler = (_e: Event) => {
      // isStreaming stays true while chunks arrive; ChatPanel appends the text
      this.isStreaming = true;
    };

    this._completedHandler = (_e: Event) => {
      this.isStreaming = false;
      this._removeStreamListeners();
    };

    this._errorHandler = (_e: Event) => {
      this.isStreaming = false;
      this._removeStreamListeners();
    };

    window.addEventListener("acp:agent_message_chunk", this._chunkHandler);
    window.addEventListener("acp:prompt_completed", this._completedHandler);
    window.addEventListener("acp:error", this._errorHandler);
  }

  // ── Public API ─────────────────────────────────────────────────────────────

  async sendMessage(conversationId: string) {
    const trimmedText = this.text.trim();
    if (!trimmedText && this.attachments.length === 0) return;

    // Build full prompt text: prepend any attachments as XML file blocks
    let fullText = trimmedText;
    if (this.attachments.length > 0) {
      const attachmentBlock = this.attachments
        .map((a) => `<file name="${a.name}">\n${a.content}\n</file>`)
        .join("\n");
      fullText = attachmentBlock + (trimmedText ? "\n" + trimmedText : "");
    }

    // Check for active ACP agent
    const activeAgent = acpStore.agents.find(
      (a) => a.status === "connected" || a.status === "ready"
    );

    if (activeAgent) {
      // Find the connection id for this agent
      // connections is a Record<connectionId, status> - find a ready/connected entry
      const connectionEntry = Object.entries(acpStore.connections).find(
        ([, status]) => status === "ready" || status === "connected"
      );

      if (!connectionEntry) {
        // No connection established yet; fall through to stub path
        await this._insertUserMessageStub(conversationId, trimmedText);
        return;
      }

      const connectionId = connectionEntry[0];
      this.activeConnectionId = connectionId;

      // Create a session if we don't have one for this connection
      if (!this.activeSessionId) {
        try {
          const sessionId = await acpStore.newSession(connectionId, ".");
          this.activeSessionId = sessionId;
        } catch (e) {
          console.error("[ChatInputStore] Failed to create ACP session", e);
          await this._insertUserMessageStub(conversationId, trimmedText);
          return;
        }
      }

      // Insert the user message into chat store for display
      await this._insertUserMessageStub(conversationId, trimmedText);

      // Clear input state before sending
      this.text = "";
      this.attachments = [];

      // Attach streaming event listeners before sending prompt
      this._attachStreamListeners();
      this.isStreaming = true;

      try {
        await acpStore.sendPrompt(connectionId, this.activeSessionId!, fullText);
      } catch (e) {
        console.error("[ChatInputStore] Failed to send ACP prompt", e);
        this.isStreaming = false;
        this._removeStreamListeners();
      }
    } else {
      // No ACP agent connected: insert user message as a stub turn
      await this._insertUserMessageStub(conversationId, trimmedText);
      this.text = "";
      this.attachments = [];
      this.isStreaming = false;
    }
  }

  private async _insertUserMessageStub(conversationId: string, text: string) {
    // chatStore.sendMessage uses activeConversationId internally, so ensure it
    // matches the conversationId we're operating on.
    if (chatStore.activeConversationId !== conversationId) {
      await chatStore.selectConversation(conversationId);
    }
    await chatStore.sendMessage([{ type: "text", text }]);
  }

  cancel() {
    if (this.activeConnectionId && this.activeSessionId) {
      acpStore.cancel(this.activeConnectionId, this.activeSessionId).catch((e) => {
        console.error("[ChatInputStore] Failed to cancel ACP", e);
      });
    }
    this.isStreaming = false;
    this._removeStreamListeners();
  }

  addAttachment(name: string, content: string, type: string) {
    if (this.attachments.length >= 10) return;
    this.attachments = [...this.attachments, { name, content, type }];
  }

  removeAttachment(index: number) {
    this.attachments = this.attachments.filter((_, i) => i !== index);
  }

  clearAll() {
    this.text = "";
    this.attachments = [];
  }
}

export const chatInputStore = new ChatInputStore();
