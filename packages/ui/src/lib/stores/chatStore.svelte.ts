import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { webSocketClient } from "@runyard/common";
import type { Conversation, Message, ContentBlock, PinnedContext, Branch } from "@runyard/common";

async function invoke<T>(cmd: string, args?: any): Promise<T> {
  if (webSocketClient.status === "connected") {
    return webSocketClient.invoke<T>(cmd, args);
  } else {
    return tauriInvoke<T>(cmd, args);
  }
}


class ChatStore {
  // Reactive states
  conversations = $state<Conversation[]>([]);
  activeConversationId = $state<string | null>(null);
  messages = $state<Message[]>([]);
  isStreaming = $state<boolean>(false);

  // Tabs management
  conversationTabs = $state<string[]>([]);
  activeTabId = $state<string | null>(null);

  // Sidebar search, filter and sorting options
  searchQuery = $state<string>("");
  workspaceFilter = $state<string | null>(null);
  sortBy = $state<"recent" | "name" | "tokens">("recent");

  // Branches & Pinned Context
  branches = $state<Branch[]>([]);
  pinnedContext = $state<PinnedContext[]>([]);

  // Derived filtered & sorted conversations list
  filteredConversations = $derived.by(() => {
    let list = [...this.conversations];

    // Filter by workspace path
    if (this.workspaceFilter) {
      list = list.filter((c) => c.workspace_path === this.workspaceFilter);
    }

    // Filter by search query (title match)
    if (this.searchQuery.trim()) {
      const q = this.searchQuery.toLowerCase();
      list = list.filter((c) => c.title.toLowerCase().includes(q));
    }

    // Sort based on option
    list.sort((a, b) => {
      if (this.sortBy === "name") {
        return a.title.localeCompare(b.title);
      } else if (this.sortBy === "tokens") {
        return b.total_tokens_used - a.total_tokens_used;
      } else {
        // default: "recent"
        return b.updated_at - a.updated_at;
      }
    });

    return list;
  });

  constructor() {
    this.init();
  }

  async init() {
    try {
      const list = await invoke<Conversation[]>("chat_conversation_list");
      this.conversations = list;
      if (list.length > 0) {
        // Open the first conversation in tabs by default
        this.openConversationInTab(list[0].id);
      }
    } catch (e) {
      console.error("[ChatStore] Failed to load conversation list", e);
    }
  }

  async selectConversation(id: string) {
    this.activeConversationId = id;
    this.activeTabId = id;
    if (!this.conversationTabs.includes(id)) {
      this.conversationTabs = [...this.conversationTabs, id];
    }
    
    // Load conversation data in parallel
    await Promise.all([
      this.loadMessages(id),
      this.loadPinnedContext(id),
      this.loadBranches(id)
    ]);
  }

  async loadMessages(conversationId: string, page?: number, limit?: number) {
    try {
      const list = await invoke<Message[]>("chat_messages_load", {
        conversation_id: conversationId,
        page,
        limit
      });
      this.messages = list;
    } catch (e) {
      console.error("[ChatStore] Failed to load messages", e);
    }
  }

  async createConversation(
    title: string, 
    model: string, 
    workspacePath = "", 
    provider = "", 
    systemPrompt?: string, 
    contextBudget = 0
  ): Promise<string> {
    try {
      const conv = await invoke<Conversation>("chat_conversation_create", {
        title,
        model,
        workspace_path: workspacePath,
        provider,
        system_prompt: systemPrompt,
        context_budget: contextBudget
      });
      this.conversations = [conv, ...this.conversations];
      this.openConversationInTab(conv.id);
      return conv.id;
    } catch (e) {
      console.error("[ChatStore] Failed to create conversation", e);
      throw e;
    }
  }

  async updateConversation(id: string, updates: Partial<Omit<Conversation, "id" | "created_at" | "updated_at">>) {
    try {
      const updated = await invoke<Conversation>("chat_conversation_update", {
        id,
        title: updates.title,
        model: updates.model,
        provider: updates.provider,
        system_prompt: updates.system_prompt,
        context_budget: updates.context_budget,
        workspace_path: updates.workspace_path
      });

      this.conversations = this.conversations.map((c) => (c.id === id ? updated : c));
    } catch (e) {
      console.error("[ChatStore] Failed to update conversation", e);
    }
  }

  async renameConversation(id: string, newTitle: string) {
    await this.updateConversation(id, { title: newTitle });
  }

  async moveConversation(id: string, newWorkspacePath: string) {
    await this.updateConversation(id, { workspace_path: newWorkspacePath });
  }

  async deleteConversation(id: string) {
    try {
      await invoke("chat_conversation_delete", { id });
      this.conversations = this.conversations.filter((c) => c.id !== id);
      this.closeConversationTab(id);
    } catch (e) {
      console.error("[ChatStore] Failed to delete conversation", e);
    }
  }

  // ── Tab Management ──────────────────────────────────────────────────────────

  openConversationInTab(id: string) {
    if (!this.conversationTabs.includes(id)) {
      this.conversationTabs = [...this.conversationTabs, id];
    }
    this.selectConversation(id);
  }

  async closeConversationTab(id: string) {
    this.conversationTabs = this.conversationTabs.filter((t) => t !== id);
    if (this.activeTabId === id) {
      if (this.conversationTabs.length > 0) {
        const nextId = this.conversationTabs[this.conversationTabs.length - 1];
        await this.selectConversation(nextId);
      } else {
        this.activeConversationId = null;
        this.activeTabId = null;
        this.messages = [];
        this.pinnedContext = [];
        this.branches = [];
      }
    }
  }

  // ── Message Operations ──────────────────────────────────────────────────────

  async sendMessage(content: ContentBlock[]) {
    if (!this.activeConversationId) return;
    const parentId = this.messages.length > 0 ? this.messages[this.messages.length - 1].id : null;

    try {
      const msg = await invoke<Message>("chat_message_insert", {
        conversation_id: this.activeConversationId,
        parent_id: parentId,
        role: "user",
        content,
      });
      this.messages = [...this.messages, msg];
      
      // Update local conversation object's message_count and updated_at
      this.conversations = this.conversations.map((c) => {
        if (c.id === this.activeConversationId) {
          return {
            ...c,
            message_count: c.message_count + 1,
            updated_at: Date.now()
          };
        }
        return c;
      });
    } catch (e) {
      console.error("[ChatStore] Failed to send message", e);
    }
  }

  async updateMessage(id: string, content: ContentBlock[]) {
    try {
      const updated = await invoke<Message>("chat_message_update", {
        id,
        content
      });
      this.messages = this.messages.map((m) => (m.id === id ? updated : m));
    } catch (e) {
      console.error("[ChatStore] Failed to update message", e);
    }
  }

  async setMessagePinned(id: string, isPinned: boolean) {
    try {
      const updated = await invoke<Message>("chat_message_set_pinned", {
        id,
        is_pinned: isPinned
      });
      this.messages = this.messages.map((m) => (m.id === id ? updated : m));
    } catch (e) {
      console.error("[ChatStore] Failed to set message pinned state", e);
    }
  }

  async searchMessages(query: string): Promise<Message[]> {
    try {
      return await invoke<Message[]>("chat_search", { query });
    } catch (e) {
      console.error("[ChatStore] Failed to search messages", e);
      return [];
    }
  }

  // ── Pinned Context Operations ───────────────────────────────────────────────

  async loadPinnedContext(conversationId: string) {
    try {
      const list = await invoke<PinnedContext[]>("chat_pinned_context_load", {
        conversation_id: conversationId
      });
      this.pinnedContext = list;
    } catch (e) {
      console.error("[ChatStore] Failed to load pinned context", e);
    }
  }

  async pinFile(filePath: string) {
    if (!this.activeConversationId) return;
    try {
      const pc = await invoke<PinnedContext>("chat_pinned_context_save", {
        conversation_id: this.activeConversationId,
        file_path: filePath
      });
      this.pinnedContext = [...this.pinnedContext, pc];
    } catch (e) {
      console.error("[ChatStore] Failed to pin file", e);
    }
  }

  async unpinFile(id: string) {
    try {
      await invoke("chat_pinned_context_delete", { id });
      this.pinnedContext = this.pinnedContext.filter((pc) => pc.id !== id);
    } catch (e) {
      console.error("[ChatStore] Failed to unpin file", e);
    }
  }

  // ── Branch Operations ───────────────────────────────────────────────────────

  async loadBranches(conversationId: string) {
    try {
      const list = await invoke<Branch[]>("chat_branch_list", {
        conversation_id: conversationId
      });
      this.branches = list;
    } catch (e) {
      console.error("[ChatStore] Failed to load branches", e);
    }
  }

  async createBranch(name: string, messageId: string) {
    if (!this.activeConversationId) return;
    try {
      const b = await invoke<Branch>("chat_branch_create", {
        conversation_id: this.activeConversationId,
        name,
        message_id: messageId
      });
      this.branches = [...this.branches, b];
    } catch (e) {
      console.error("[ChatStore] Failed to create branch", e);
    }
  }

  async deleteBranch(id: string) {
    try {
      await invoke("chat_branch_delete", { id });
      this.branches = this.branches.filter((b) => b.id !== id);
    } catch (e) {
      console.error("[ChatStore] Failed to delete branch", e);
    }
  }
}

export const chatStore = new ChatStore();
