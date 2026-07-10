# Runyard — Chat Panel Specification

Complete specification for the Runyard AI Chat Panel: conversation model, message type taxonomy, rendering and interaction patterns, context management, input system, provider orchestration, and data persistence.

## 1. Design Principles

The chat panel is not a sidebar widget. It is a first-class tab — equal to the editor, terminal, and explorer. These principles guide every decision:

- **Chat is a workspace tool, not a popup.** It owns the full pane, can be split, docked, detached, and pinned like any editor tab.
- **Every message is a living document.** Messages can be edited, branched, pinned, searched, and referenced. They are not ephemeral bubbles.
- **The agent is a collaborator, not a chatbot.** Agent output renders as rich structured content (diffs, tool traces, thinking blocks) — not plain text.
- **Context is visible and controllable.** The user always knows how much context they're using, what's in it, and can shape it.
- **Performance at 10k+ messages.** Virtual scrolling, lazy rendering, and efficient SQLite queries from message one.
- **Offline-first.** The chat works without network. Messages persist locally. AI degrades gracefully.

## 2. Conversation Model

## 2.1 Core Entities

### Conversation
A conversation is a named, branchable thread of messages scoped to a workspace.

```typescript
interface Conversation {
  id: string;                          // UUID
  title: string;                       // User-editable, auto-generated from first prompt
  workspacePath: string;               // Which project this belongs to
  model: string;                       // e.g. "claude-sonnet-4-6"
  provider: string;                    // e.g. "anthropic", "openai", "google", "local"
  systemPrompt?: string;               // Per-conversation system prompt override
  pinnedFiles: PinnedContext[];        // Files/symbols pinned to context
  contextBudget: number;               // Max tokens (from model capabilities)
  createdAt: number;                   // Unix timestamp
  updatedAt: number;
  messageCount: number;                // Denormalized for list views
  totalTokensUsed: number;             // Lifetime token counter
  totalCost: number;                   // Lifetime cost in USD
}
```

### Branch
A conversation can fork at any message. Branches are named and switchable.

```typescript
interface Branch {
  id: string;
  conversationId: string;
  name: string;                        // User-named or auto-generated
  forkPointMessageId: string;          // Which message this branched from
  createdAt: number;
  isMain: boolean;                     // The main conversation trunk
}
```

### Pinned Context
Files, symbols, or conversations pinned to the agent's context.

```typescript
interface PinnedContext {
  type: "file" | "symbol" | "conversation" | "skill" | "note";
  uri: string;                         // e.g. "file:///src/main.rs", "symbol://MyClass", "skill://pdf-processing"
  label: string;                       // Display name
  lineRange?: { start: number; end: number };  // For file pins
  tokenEstimate: number;               // Approximate token count
}
```

## 2.2 Persistence Layer

All conversations stored in SQLite (`~/.runyard/chat.db`). Schema:

```sql
CREATE TABLE conversations (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  workspace_path TEXT NOT NULL,
  model TEXT NOT NULL,
  provider TEXT NOT NULL,
  system_prompt TEXT,
  context_budget INTEGER NOT NULL,
  created_at INTEGER NOT NULL,
  updated_at INTEGER NOT NULL,
  message_count INTEGER DEFAULT 0,
  total_tokens_used INTEGER DEFAULT 0,
  total_cost REAL DEFAULT 0.0
);

CREATE TABLE messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id),
  branch_id TEXT NOT NULL,
  parent_id TEXT,                       -- NULL for root messages
  role TEXT NOT NULL,                   -- 'user' | 'assistant' | 'system' | 'tool'
  content_json TEXT NOT NULL,           -- Full message content as JSON (see §3)
  token_count INTEGER,
  cost REAL,
  model TEXT,                           -- Model used for this specific message
  created_at INTEGER NOT NULL,
  edited_at INTEGER,                    -- NULL if never edited
  is_pinned INTEGER DEFAULT 0
);

CREATE TABLE branches (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id),
  name TEXT NOT NULL,
  fork_point_message_id TEXT NOT NULL,
  is_main INTEGER DEFAULT 0,
  created_at INTEGER NOT NULL
);

CREATE TABLE pinned_context (
  id TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id),
  type TEXT NOT NULL,
  uri TEXT NOT NULL,
  label TEXT NOT NULL,
  line_start INTEGER,
  line_end INTEGER,
  token_estimate INTEGER
);

-- Full-text search index
CREATE VIRTUAL TABLE messages_fts USING fts5(
  content_text,                         -- Extracted plain text from content_json
  content='messages',
  content_rowid='rowid'
);
```

## 2.3 Multi-Conversation Management

- **Conversation tabs** in the chat panel — like editor tabs but for conversations. Each tab = one conversation.
- **Conversation list sidebar** (collapsible): search, filter by workspace, sort by recent/name/tokens.
- **Rename**: double-click title or right-click → Rename.
- **Delete**: right-click → Delete with confirmation dialog showing message count and token cost.
- **Export**: Export conversation as Markdown, JSON, or HTML. Includes all messages, tool traces, and diffs.
- **Move**: Reassign conversation to a different workspace.

## 2.4 Branching Model

- **Fork from any message**: Right-click → "Branch from here". Creates a new branch starting from that message as parent.
- **Branch tree visualizer**: Horizontal tree diagram showing branch lineage. Click any node to switch to that branch.
- **Branch indicator**: Current branch name shown in conversation header with dropdown to switch.
- **Merge branches** (Phase 2): Selectively merge messages from one branch into another.
- **Branch comparison**: Side-by-side diff of two branches.

## 3. Message Type Taxonomy

Every message has a `role` and a `content` array of typed blocks. This is the complete taxonomy:

## 3.1 Message Roles

| Role | Description | Examples |
|------|-------------|----------|
| `user` | Human message | Typed prompt, pasted code, file reference |
| `assistant` | AI response | Answer, code generation, explanation |
| `system` | System directive | Skill injection, context summary, model switch notice |
| `tool_call` | Agent invokes a tool | MCP tool call, filesystem operation, shell command |
| `tool_result` | Tool execution result | Command output, API response, file contents |
| `diff` | File modification | Agent edits a file, user accepts/rejects hunks |
| `thinking` | Agent reasoning trace | Extended thinking output, chain-of-thought |
| `error` | Error state | API error, tool failure, permission denied |

## 3.2 Content Block Types

Each message contains an ordered array of content blocks. A single message can combine multiple block types.

### text
Plain or Markdown text. The primary content block.

```typescript
interface TextBlock {
  type: "text";
  text: string;                        // Markdown content
}
```

### code
Syntax-highlighted code block with action buttons.

```typescript
interface CodeBlock {
  type: "code";
  code: string;
  language: string;                    // e.g. "rust", "typescript", "python"
  filename?: string;                   // If the code is from a file
  actions: CodeAction[];               // Available actions
}

type CodeAction = "copy" | "insert_at_cursor" | "apply_to_file" | "create_file" | "run_in_terminal" | "explain";
```

### diff_card
A file modification proposed by the agent. The user accepts or rejects per-hunk.

```typescript
interface DiffCardBlock {
  type: "diff_card";
  filePath: string;
  originalContent: string;             // Pre-edit content
  newContent: string;                  // Post-edit content
  hunks: DiffHunk[];                   // Individual change hunks
  status: "pending" | "accepted" | "rejected" | "partially_accepted";
}

interface DiffHunk {
  id: string;
  startLine: number;
  endLine: number;
  original: string;
  modified: string;
  status: "pending" | "accepted" | "rejected";
}
```

### tool_call_card
Collapsed by default. Shows what tool the agent called and with what arguments.

```typescript
interface ToolCallBlock {
  type: "tool_call";
  toolCallId: string;
  toolName: string;                    // e.g. "mcp__github__search_issues"
  serverName?: string;                 // MCP server that provides this tool
  arguments: Record<string, unknown>;  // Tool arguments
  status: "running" | "completed" | "errored" | "awaiting_approval";
  startedAt: number;
  completedAt?: number;
  durationMs?: number;
}
```

### tool_result_card
The result of a tool call. Linked to its parent tool_call by toolCallId.

```typescript
interface ToolResultBlock {
  type: "tool_result";
  toolCallId: string;                  // Links to parent tool_call
  isError: boolean;
  content: string;                     // Truncated if >10KB, with "Show full output" button
  contentType: "text" | "json" | "html" | "image" | "binary";
  sizeBytes: number;
  truncated: boolean;
}
```

### thinking_block
Agent's reasoning trace. Collapsed by default.

```typescript
interface ThinkingBlock {
  type: "thinking";
  content: string;                     // The reasoning text
  tokensConsumed: number;
  collapsed: boolean;                  // UI state: expanded or collapsed
}
```

### permission_request
When an agent needs human approval to use a tool. Displays as a prominent card with Approve/Deny buttons.

```typescript
interface PermissionRequestBlock {
  type: "permission_request";
  requestId: string;
  toolName: string;
  serverName?: string;
  arguments: Record<string, unknown>;
  reason: string;                      // Why the agent wants to use this tool
  riskLevel: "low" | "medium" | "high";
  status: "pending" | "approved" | "denied" | "approved_for_session";
}
```

### file_reference
A reference to a file or symbol. Renders as a clickable chip that opens the file.

```typescript
interface FileReferenceBlock {
  type: "file_reference";
  filePath: string;
  symbolName?: string;                 // If referencing a specific symbol
  lineStart?: number;
  lineEnd?: number;
  preview?: string;                    // First few lines for hover preview
}
```

### plan_card
When an agent proposes a multi-step plan before executing.

```typescript
interface PlanCardBlock {
  type: "plan_card";
  title: string;                       // e.g. "Plan: Refactor authentication module"
  steps: PlanStep[];
  estimatedTokens?: number;
  status: "proposed" | "approved" | "rejected" | "in_progress" | "completed";
}

interface PlanStep {
  id: string;
  description: string;
  status: "pending" | "in_progress" | "completed" | "skipped" | "failed";
  toolCalls?: string[];                // Associated tool call IDs
}
```

### context_summary
Auto-generated summary when older messages are compressed near the context limit.

```typescript
interface ContextSummaryBlock {
  type: "context_summary";
  summary: string;                     // AI-generated summary of compressed messages
  originalMessageCount: number;        // How many messages were summarized
  originalTokenCount: number;          // Tokens before compression
  compressedTokenCount: number;        // Tokens after compression
  expandable: boolean;                 // User can click to expand and see original messages
}
```

### error_block
Rendered when something goes wrong with a message or tool call.

```typescript
interface ErrorBlock {
  type: "error";
  errorCode: string;                   // e.g. "RATE_LIMITED", "AUTH_FAILED", "TOOL_NOT_FOUND"
  message: string;
  recoverable: boolean;                // Can the user retry?
  suggestedAction?: string;            // e.g. "Check your API key in Settings"
}
```

## 3.3 Full Message Shape

```typescript
interface Message {
  id: string;
  conversationId: string;
  branchId: string;
  parentId: string | null;
  role: MessageRole;
  content: ContentBlock[];             // Ordered array of typed blocks
  tokenCount: number | null;
  cost: number | null;
  model: string | null;
  createdAt: number;
  editedAt: number | null;
  isPinned: boolean;
}

type ContentBlock =
  | TextBlock
  | CodeBlock
  | DiffCardBlock
  | ToolCallBlock
  | ToolResultBlock
  | ThinkingBlock
  | PermissionRequestBlock
  | FileReferenceBlock
  | PlanCardBlock
  | ContextSummaryBlock
  | ErrorBlock;
```

## 4. Rendering Specification

## 4.1 Message Layout

Each message renders as a card with:
- **Role avatar** (left): User icon for user, agent icon + model name for assistant, tool icon for tool calls, warning icon for errors.
- **Timestamp** (right): Relative time ("2m ago") with absolute on hover.
- **Content area**: Ordered list of content blocks rendered per their type (see below).
- **Message actions** (hover): Copy, Edit (user only), Branch from here, Pin/Unpin, Delete.
- **Branch indicator**: If this is a fork point, a small branch icon with branch count.

## 4.2 Block Rendering

### Text Block
- Rendered as **GitHub-flavored Markdown**.
- Syntax highlighting via CodeMirror for fenced code blocks within markdown.
- Links open in external browser (`target="_blank"`).
- Images rendered inline with lightbox on click.
- Tables, lists, blockquotes, headings all supported.
- Math via KaTeX (fenced with `$$`).
- Mermaid diagrams rendered inline.

### Code Block
- Rendered as a **card within the message** with a header bar.
- Header shows: language icon + name + filename (if applicable).
- Header actions: Copy, Insert at Cursor, Apply to File, Create New File, Run in Terminal, Explain (sends "Explain this code" as follow-up).
- Code content uses CodeMirror in read-only mode for syntax highlighting.
- Line numbers on left. Word wrap toggle.
- Maximum height: 400px with scroll. "Expand" button for full height.

### Diff Card
- **Unified diff view** as default. Toggle to side-by-side.
- Green background for additions, red for deletions.
- **Per-hunk controls**: Accept (✓) / Reject (✗) buttons per hunk.
- **Bulk controls**: Accept All / Reject All at the top of the card.
- Line numbers show both old and new positions.
- After accepting: file is written to disk, diff card collapses to a summary ("3 hunks applied to src/main.rs").
- After rejecting: card shows strikethrough, "Undo" button for 30 seconds.
- Multi-file diff: one diff card per file, grouped under a "Batch change" header.

### Tool Call Card
- **Collapsed by default**: Shows tool name + icon + status spinner.
- **Expanded**: Shows arguments as syntax-highlighted JSON.
- Status transitions: running (spinner) → completed (green check) → errored (red X).
- If `awaiting_approval`, card is expanded and prominent with Approve/Deny buttons.
- Duration shown after completion (e.g. "Took 1.2s").

### Tool Result Card
- Linked to parent tool call by indent + connector line.
- **Collapsed if >500 chars**. "Show output" button to expand.
- Content type icon: text icon, JSON icon, image thumbnail, binary download link.
- Truncation indicator: "Showing first 10KB of 450KB. [Show full output]".
- Error results: red border, error icon, error message highlighted.
- "Copy output" button always available.

### Thinking Block
- **Collapsed by default** with label: "Agent reasoning (2.4K tokens) — click to expand".
- Expanded: monospaced font, dimmed background, distinct from main response.
- Scrollable with max height 300px.

### Permission Request
- **Always expanded** — cannot be collapsed (it requires user action).
- Amber border, warning icon.
- Shows: tool name, server, arguments (formatted), agent's reason, risk level.
- Two buttons: **Approve** (green) and **Deny** (red).
- Third option: "Approve for this session" (remembers approval for all future calls to this tool in this conversation).
- Timeout indicator: "Awaiting approval for 45s..." (configurable auto-deny timeout).

### File Reference
- Renders as a **clickable chip**: `📄 src/main.rs:42-58`.
- Hover: shows file preview (first 10 lines with syntax highlighting).
- Click: opens file in editor at the referenced line range.
- In user messages, file references come from `@file` autocomplete.
- In assistant messages, they come from the agent referencing project files.

### Plan Card
- **Expandable card** with title and step count.
- Expanded shows checklist of steps with status icons (○ pending, ◐ in progress, ✓ done, ✗ failed, — skipped).
- Animated transition when step status changes.
- "Approve Plan" button for proposed plans.
- Progress bar at top showing completed/total.

### Context Summary
- **Collapsed by default**: "📋 Context compressed: 47 messages summarized into 850 tokens. [Expand to see original]".
- Expanded shows the AI-generated summary text + list of original message IDs (not full content — that's in the DB).
- Visual divider (dashed line) separating the summary from newer messages.

### Error Block
- Red left border, error icon.
- Error message in monospace.
- If recoverable: "Retry" button.
- Suggested action text below the error.

## 4.3 Streaming Behavior

When the agent is generating a response:
- **Text blocks** stream character-by-character (like Cursor/ChatGPT). Cursor blinks at end.
- **Code blocks** render progressively as the fence is detected.
- **Tool call cards** appear as soon as the tool call JSON is parsed, with running status.
- **Thinking blocks** stream progressively if the model supports streaming thinking.
- **Diff cards** render after the full diff is received (not streamable).
- A **"Stop" button** appears during generation. Stopping sends a cancellation to the provider.
- Regeneration: "Regenerate" button appears after completion. Replaces the last assistant message.

## 4.4 Virtual Scrolling

The message list uses virtual scrolling for performance at scale:
- Only ~20 messages rendered in DOM at any time.
- Smooth scroll with no jank at 10k+ messages.
- Scroll position preserved when switching tabs.
- "Scroll to bottom" floating button when user scrolls up during streaming.
- "Jump to top" button for very long conversations.
- Estimated message heights pre-calculated from content block counts.

## 5. Input System

## 5.1 Input Area

The input area is a multi-line text editor (not a single-line input). It is a full CodeMirror instance in markdown mode.

### Features:
- **Enter = newline**, **Ctrl/Cmd+Enter = send**. Configurable swap in settings.
- **Shift+Enter = newline** (always, regardless of config).
- **Auto-resize**: grows from 2 lines to max 20 lines, then scrolls.
- **Markdown preview toggle**: shows rendered preview above the input.
- **Character count** (bottom right): "1,247 / ∞" (no hard limit, but shows).
- **Attachment bar** (above input): shows attached files, images, and context chips.

## 5.2 Autocomplete System

As the user types, an autocomplete popup appears triggered by `@`:

### @file — File Reference
- Fuzzy search across all workspace files.
- Shows: file icon + path + last modified.
- Selecting inserts: `@file:src/main.rs` which renders as a File Reference chip.
- Supports line ranges: `@file:src/main.rs:42-58`.

### @symbol — Symbol Reference
- Fuzzy search across workspace symbols (functions, classes, types).
- Requires LSP index. Fallback: Tree-sitter symbol extraction.
- Shows: symbol kind icon + name + file location.
- Selecting inserts: `@symbol:MyClass`.

### @skill — Skill Reference
- Lists available skills (global + project).
- Shows: skill icon + name + first line of description.
- Selecting inserts: `@skill:pdf-processing` — injects skill body as context.

### @agent — Agent Mention
- Lists configured agents (ACP-connected).
- Selecting routes the message to a specific agent instead of the default.

### @conversation — Cross-Conversation Reference
- Lists other conversations in the workspace.
- Selecting links to that conversation (for context sharing).

## 5.3 Attachments

- **Drag and drop** files from explorer or OS file manager onto the input area.
- **Paste images** from clipboard — stored as base64 in message, rendered inline.
- **+ Attachment button** — opens file picker.
- Attachments appear as chips above the input with an X to remove.
- Supported: images (PNG, JPG, GIF, WebP), text files, PDFs (extracted text sent to model).

## 5.4 Prompt Enhancer

A toggle button (wand icon) next to the send button. When enabled:

- On send, the draft prompt is rewritten by a **fast, cheap model** (e.g., Haiku or Flash) before being sent to the main model.
- Two modes (configurable per conversation):
  - **Expand**: Adds detail, clarifies vague statements, adds context.
  - **Specify**: Makes the request more precise, adds constraints and edge cases.
- The enhanced version is shown in a preview card with "Use enhanced" / "Send original" buttons.
- The original draft is always preserved and accessible.
- Cost of enhancement is tracked separately (typically <1% of main model cost).

## 5.5 Send Behavior

When the user sends:

1. Input is captured as a `Message` with `role: "user"`.
2. If prompt enhancer is on, the draft is sent to the enhancer model. Result shown as preview.
3. On confirmation, the message is persisted to SQLite.
4. Context assembly begins (§6):
   - System prompt (base + conversation override + skill injections)
   - Pinned context files/symbols
   - Active editor selection (if auto-context is on)
   - Recent message history (within context budget)
   - Compressed summaries of older messages (if near limit)
5. The full context + user message is sent to the provider.
6. Streaming response begins.
7. On completion, the assistant message is persisted with token count and cost.
8. Context window visualizer updates.

### Cancellation
- During streaming: "Stop" button cancels the request.
- Partial response is preserved as the assistant message.
- User can "Continue" to resume or "Regenerate" to replace.

## 6. Context Management

## 6.1 Context Window Visualizer

A real-time token bar at the top of the conversation:

```
┌─────────────────────────────────────────────────────────┐
│ Context: ████████████░░░░░░░░░░  14,230 / 200,000 (7%)  │
│ System: 1.2K | Pinned: 3.8K | History: 8.4K | Draft: 850│
└─────────────────────────────────────────────────────────┘
```

### Visual States:
- **Green (<50%)**: Normal. Bar is green.
- **Yellow (50-80%)**: Warning. Bar is amber. "Consider compressing" hint.
- **Red (>80%)**: Critical. Bar is red. Prominent "Compress context" button.
- **Red (>95%)**: Near limit. Bar flashes. Auto-compression may trigger (if enabled).

### Breakdown Tooltip:
Hovering the bar shows a detailed breakdown:
- System prompt: 1,200 tokens
- Pinned files (3): 3,800 tokens
- Skills injected (2): 950 tokens
- Message history (47 messages): 8,400 tokens
- Current draft: 850 tokens
- Total: 15,200 / 200,000

## 6.2 Context Assembly Algorithm

Every time a message is sent, context is assembled in this order of priority:

```
Priority 1 — Always included (non-negotiable):
├── Base system prompt
├── Conversation-level system prompt override (if set)
├── Active skill injections (from @skill references)
└── Pinned context (files, symbols, notes)

Priority 2 — Auto-context from editor:
├── Active editor selection (cursor position + surrounding function)
├── Open editor tabs (filenames only, not contents)
└── Recently viewed files (last 5, filenames only)

Priority 3 — Message history:
├── Most recent messages first (reverse chronological)
├── Fill remaining budget up to 80%
└── If not all history fits:
    ├── Compress oldest messages into a context_summary
    └── Insert context_summary at the boundary

Priority 4 — Bonus space:
└── Remaining 20% of budget reserved for model response
```

## 6.3 Pinned Context

- **Pin a message**: Right-click → "Pin to context". Adds entire message content.
- **Pin a file**: Drag from explorer to the pinned context bar, or `@file` and toggle pin.
- **Pin a symbol**: `@symbol` and toggle pin.
- **Pinned context bar**: Horizontal bar above the input showing all pinned items as chips. Each chip shows type icon + label + token count + X to unpin.
- **Pinned items persist** across conversation restarts.

## 6.4 Smart Context Compression

When message history exceeds the available budget:

1. **Identify boundary**: Find the oldest messages that can be compressed to fit within budget.
2. **Generate summary**: Send those messages to a fast model with prompt: "Summarize this conversation segment. Preserve key decisions, file changes, and outstanding tasks."
3. **Insert context_summary**: Replace compressed messages with a single ContextSummaryBlock.
4. **Preserve original messages**: They remain in SQLite. The summary is an overlay — the user can expand to see originals.
5. **Compression indicator**: Dashed line in the message list showing where compression happened.

### Compression Settings:
- **Auto-compress** (toggle): Automatically compress when >90% budget.
- **Aggressiveness**: Conservative (keep more messages) vs Aggressive (compress earlier).
- **Manual compress**: "Compress context" button in the context bar. Compresses everything except the last N messages (user-configurable, default 10).

## 6.5 Context Clearing

- **Clear context**: Button in conversation header. Preserves system prompt and pinned context. Clears all message history. Confirmation dialog: "Clear all 47 messages? Pinned context will be preserved."
- **Reset conversation**: Clears everything including pinned context. Fresh start.
- Both actions log a `system` message recording what was cleared.

## 7. Provider Orchestration

## 7.1 Provider Abstraction

The provider layer (`packages/ai-provider/`) exposes a single interface:

```typescript
interface AiProvider {
  readonly id: string;
  readonly name: string;
  readonly models: ModelInfo[];
  
  sendMessage(
    model: string,
    messages: Message[],
    systemPrompt: string,
    tools: ToolDefinition[],
    options: SendOptions
  ): AsyncGenerator<StreamEvent>;
  
  countTokens(model: string, messages: Message[]): Promise<number>;
  estimateCost(model: string, inputTokens: number, outputTokens: number): number;
  validateApiKey(): Promise<boolean>;
}

interface SendOptions {
  maxTokens?: number;
  temperature?: number;
  topP?: number;
  stopSequences?: string[];
  enableThinking?: boolean;
  thinkingBudget?: number;
}

type StreamEvent =
  | { type: "text_delta"; text: string }
  | { type: "tool_call_start"; toolCallId: string; toolName: string }
  | { type: "tool_call_delta"; toolCallId: string; argumentsDelta: string }
  | { type: "tool_call_end"; toolCallId: string; arguments: Record<string, unknown> }
  | { type: "thinking_delta"; text: string }
  | { type: "thinking_end" }
  | { type: "done"; finishReason: string; usage: TokenUsage }
  | { type: "error"; code: string; message: string };
```

## 7.2 Supported Providers

### Anthropic (Claude)
- Models: Claude Opus 4.8, Sonnet 5, Haiku 4.5, Fable 5
- Auth: API key in OS keychain
- Features: Extended thinking, native tool use, 200K context
- Streaming: SSE with `text_delta`, `content_block_start/stop`, `ping`

### OpenAI (GPT / Codex)
- Models: GPT 5.5, GPT-4o, Codex CLI models
- Auth: API key in OS keychain
- Base URL override for Azure / compatible APIs
- Streaming: SSE with `chat.completion.chunk`

### Google (Gemini)
- Models: Gemini 3.5 Flash, Gemini Pro
- Auth: API key in OS keychain
- Features: 1M token context window, native tool use
- Streaming: SSE

### OpenRouter
- Unified access to 200+ models
- Auth: API key in OS keychain
- Per-model pricing metadata from OpenRouter API
- Fallback: automatically retry with a different model if primary fails

### Local (Ollama / llama.cpp)
- No API key, no network required
- Base URL: `http://localhost:11434` (Ollama) or custom
- Model list from local registry
- Streaming: Ollama chat API
- No cost tracking (free)

## 7.3 Model Switching

- **Per-conversation model**: Set when creating a conversation, changeable mid-conversation.
- **Model dropdown**: In conversation header. Shows current model + provider icon.
- **Mid-conversation switch**: Changes the model for all future messages in that conversation. Previous messages keep their model attribution.
- **Cost preview**: When switching, show estimated cost difference: "Switching from Claude Sonnet 5 ($3/$15 per M tokens) to GPT 5.5 ($2.50/$10 per M tokens)".
- **Context limit warning**: If switching to a model with smaller context window, warn: "GPT 5.5 has 128K context limit. Your current context is 142K tokens."

## 7.4 Cost Tracking

- **Per-message token count**: Provider returns `usage.input_tokens` and `usage.output_tokens` in the `done` event.
- **Per-message cost**: `inputCost = inputTokens * provider.inputPrice + outputTokens * provider.outputPrice`.
- **Cost display**: Shown in the message footer: "1.2K in / 850 out · $0.0032".
- **Conversation total**: Shown in header: "Total: 142K tokens · $0.47".
- **Daily/Project dashboards**: Phase 2.
- **Spending limits**:
  - Soft cap: Warning when approaching monthly limit (configurable per provider).
  - Hard cap: Blocks further API calls when limit reached. Shows "Limit reached" error.
  - Per-conversation cap: Optional maximum budget for a single conversation.
  - Notification: System message when approaching limits.

## 8. Chat Panel UI Layout

## 8.1 Full Layout

```
┌──────────────────────────────────────────────────────────────┐
│ ← Conversation List │ 💬 Project Setup · Claude Sonnet 5 ▼   │ ← Header
│                     │ Branch: main ▼  │ 14.2K/200K  │  ...  │
├─────────────────────┴────────────────────────────────────────┤
│ Context: ████░░░░░░░░  14,230/200K  [Compress] [Clear]       │ ← Context bar
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌─ User ───────────────────────────── 2m ago ────────────┐ │
│  │                                                        │ │
│  │ Can you refactor the auth module to use Argon2         │ │
│  │ instead of bcrypt? @file:src/auth/handler.rs           │ │
│  │                                                        │ │
│  │ [📄 src/auth/handler.rs]                               │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ┌─ Assistant ──────────────────────── 1m ago ───────────┐ │
│  │                                                        │ │
│  │ I'll refactor the auth module. Let me first read the   │ │
│  │ current implementation...                              │ │
│  │                                                        │ │
│  │ ┌─ 🔧 mcp_filesystem_read_file ────────────────── ┐   │ │
│  │ │ src/auth/handler.rs · Completed · 0.8s           │   │ │
│  │ └──────────────────────────────────────────────────┘   │ │
│  │                                                        │ │
│  │ Here's the refactored code:                            │ │
│  │                                                        │ │
│  │ ┌─ 📝 rust · src/auth/handler.rs ─────────────────┐   │ │
│  │ │ [Copy] [Apply to File] [Explain]                  │   │ │
│  │ │ ────────────────────────────────────────────      │   │ │
│  │ │ 1  use argon2::{Argon2, PasswordHasher,...};      │   │ │
│  │ │ 2  use crate::errors::AuthError;                  │   │ │
│  │ │ 3                                                  │   │ │
│  │ │ 4  pub async fn hash_password(...                  │   │ │
│  │ └──────────────────────────────────────────────────┘   │ │
│  │                                                        │ │
│  │ ┌─ 📊 Diff: src/auth/handler.rs ──────────────────┐   │ │
│  │ │ [Accept All] [Reject All]                         │   │ │
│  │ │ ────────────────────────────────────────────      │   │ │
│  │ │  - use bcrypt::{hash, verify};                    │   │ │
│  │ │  + use argon2::{Argon2, PasswordHasher,...};      │   │ │
│  │ │  [✓ Accept] [✗ Reject]                            │   │ │
│  │ │ ────────────────────────────────────────────      │   │ │
│  │ │  - let hash = hash(password, 12)?;                │   │ │
│  │ │  + let hash = Argon2::default()                   │   │ │
│  │ │  +     .hash_password(password.as_bytes(),...)?;  │   │ │
│  │ │  [✓ Accept] [✗ Reject]                            │   │ │
│  │ └──────────────────────────────────────────────────┘   │ │
│  │                                                        │   │
│  │ 1.2K in · 850 out · $0.0032                            │  │ ← Cost footer
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
│  ▾ Agent reasoning (2.4K tokens)                             │ ← Thinking (collapsed)
│                                                              │
│  ───── 📋 Context compressed: 47→850 tokens [Expand] ────── │ ← Compression boundary
│                                                              │
│  ┌─ User ──────────────────────────── 12m ago ───────────┐ │ ← Older messages
│  │ Earlier conversation about database schema...          │ │
│  └────────────────────────────────────────────────────────┘ │
│                                                              │
├──────────────────────────────────────────────────────────────┤
│ Pinned: [📄 schema.rs 1.2K ×] [🔧 rust-analyzer 450 ×]       │ ← Pinned context
├──────────────────────────────────────────────────────────────┤
│ ┌──────────────────────────────────────────────────────┐     │
│ │ Your message...                           1,247 chars │     │ ← Input area
│ │ @file:sr│  ← autocomplete popup                       │     │
│ │   src/auth/handler.rs                                │     │
│ │   src/auth/middleware.rs                             │     │
│ │   src/models/user.rs                                 │     │
│ └──────────────────────────────────────────────────────┘     │
│  [📎] [🪄 Enhance]                       [Ctrl+Enter Send]  │ ← Input toolbar
└──────────────────────────────────────────────────────────────┘
```

## 8.2 Split View Behavior

- Chat panel can be **split** like any editor tab: horizontal split = two conversations side by side.
- **Detach**: Pop out a conversation into a floating window.
- **Dock with editor**: Split the main area with editor on left, chat on right — standard AI-coding layout.

## 8.3 Mobile Layout

On mobile (<768px width):
- Conversation list slides in from left (hamburger menu).
- Context bar collapses to a single token ring with percentage.
- Message actions move to a long-press menu.
- Input area is full-width, sticky at bottom.
- Autocomplete renders as a bottom sheet.
- Code blocks and diffs scroll horizontally.

## 8.4 Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Ctrl/Cmd+Enter | Send message |
| Ctrl/Cmd+K | Focus conversation search |
| Ctrl/Cmd+Shift+N | New conversation |
| Ctrl/Cmd+Shift+[ | Previous conversation tab |
| Ctrl/Cmd+Shift+] | Next conversation tab |
| Ctrl/Cmd+L | Focus chat input |
| Ctrl/Cmd+Shift+C | Clear context |
| Ctrl/Cmd+Shift+B | Branch from current message |
| Escape | Close conversation list / close autocomplete |

## 9. Data Flow — Send Message Sequence

```
User clicks Send (or Ctrl+Enter)
        │
        ▼
┌───────────────────┐
│ 1. CAPTURE INPUT   │
│ Parse input area   │
│ Resolve @refs      │
│ Process attachments│
│ Create Message obj │
└───────┬───────────┘
        │
        ▼
┌───────────────────┐
│ 2. ENHANCE (opt)   │
│ Send to fast model │
│ Show preview card  │
│ Wait for confirm   │
└───────┬───────────┘
        │
        ▼
┌───────────────────┐
│ 3. PERSIST         │
│ INSERT into SQLite │
│ UPDATE conversation│
│ Update FTS index   │
└───────┬───────────┘
        │
        ▼
┌───────────────────┐
│ 4. ASSEMBLE CONTEXT│
│ Load system prompt │
│ Inject skills      │
│ Load pinned context│
│ Load auto-context  │
│ Load history (fit) │
│ Compress if needed │
│ Count tokens       │
└───────┬───────────┘
        │
        ▼
┌───────────────────┐
│ 5. CHECK BUDGET    │
│ Within limits?     │
│ Hard cap reached?  │
│ Soft cap warning?  │
└───────┬───────────┘
        │ OK
        ▼
┌───────────────────┐
│ 6. SEND TO PROVIDER│
│ provider.sendMsg() │
│ Start streaming    │
└───────┬───────────┘
        │
        ▼ (streaming loop)
┌───────────────────┐
│ 7. HANDLE EVENTS   │
│ text_delta →       │──→ Append to text block
│   render char      │
│ tool_call_start →  │──→ Create tool_call card
│   show card        │
│ tool_call_delta →  │──→ Accumulate arguments JSON
│   update card      │
│ tool_call_end →    │──→ Finalize card
│   mark running     │
│ thinking_delta →   │──→ Append to thinking block
│   show collapsed   │
│ done →             │──→ Finalize message
│   persist          │
│ error →            │──→ Show error block
│   mark errored     │
└───────┬───────────┘
        │
        ▼ (on done)
┌───────────────────┐
│ 8. FINALIZE        │
│ Persist assistant  │
│   message to SQLite│
│ Update token count │
│ Update cost        │
│ Update context bar │
│ Focus input        │
└───────────────────┘
```

## 10. Implementation Notes

## 10.1 New Files to Create

```
packages/common/src/
├── chat-types.ts          # All chat TypeScript types (§2, §3)
└── chat-defaults.ts       # Default system prompts, model configs

packages/ai-provider/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts           # Provider registry
│   ├── types.ts           # AiProvider interface, StreamEvent, etc.
│   ├── anthropic.ts       # Anthropic Claude provider
│   ├── openai.ts          # OpenAI / Azure / compatible provider
│   ├── google.ts          # Google Gemini provider
│   ├── openrouter.ts      # OpenRouter provider
│   ├── local.ts           # Ollama / llama.cpp provider
│   ├── context-assembler.ts  # §6.2 algorithm
│   ├── context-compressor.ts # §6.4 summarization
│   └── token-counter.ts   # Token counting (tiktoken for OpenAI, Claude tokenizer, etc.)

packages/ui/src/lib/layout/
├── ChatPanel.svelte        # Main chat panel container
├── ChatMessage.svelte      # Single message renderer (dispatches to block renderers)
├── ChatInput.svelte        # Multi-line input with autocomplete
├── ChatContextBar.svelte   # Token bar + compression controls
├── ConversationList.svelte # Sidebar conversation browser
├── blocks/
│   ├── TextBlock.svelte
│   ├── CodeBlock.svelte
│   ├── DiffCardBlock.svelte
│   ├── ToolCallBlock.svelte
│   ├── ToolResultBlock.svelte
│   ├── ThinkingBlock.svelte
│   ├── PermissionRequestBlock.svelte
│   ├── FileReferenceBlock.svelte
│   ├── PlanCardBlock.svelte
│   ├── ContextSummaryBlock.svelte
│   └── ErrorBlock.svelte
└── stores/
    └── chatStore.svelte.ts   # Central chat state management

apps/desktop/src-tauri/
└── src/
    └── chat_db.rs           # Rust SQLite layer for chat persistence
```

## 10.2 Svelte 5 Store Design (chatStore)

```typescript
// packages/ui/src/lib/stores/chatStore.svelte.ts
class ChatStore {
  // State
  conversations = $state<Conversation[]>([]);
  activeConversationId = $state<string | null>(null);
  messages = $state<Map<string, Message[]>>(new Map());  // conversationId → messages
  isStreaming = $state<boolean>(false);
  streamingMessageId = $state<string | null>(null);
  
  // Derived
  activeConversation = $derived(
    this.conversations.find(c => c.id === this.activeConversationId)
  );
  activeMessages = $derived(
    this.activeConversationId 
      ? this.messages.get(this.activeConversationId) ?? []
      : []
  );
  contextUsage = $derived(
    this.computeContextUsage(this.activeConversation, this.activeMessages)
  );

  // Actions
  async sendMessage(text: string, attachments: Attachment[]): Promise<void>;
  async regenerateMessage(messageId: string): Promise<void>;
  async editMessage(messageId: string, newText: string): Promise<void>;
  async branchFromMessage(messageId: string, branchName: string): Promise<void>;
  async switchBranch(branchId: string): Promise<void>;
  async compressContext(): Promise<void>;
  async clearContext(): Promise<void>;
  
  pinContext(item: PinnedContext): void;
  unpinContext(id: string): void;
  
  acceptDiffHunk(messageId: string, hunkId: string): void;
  rejectDiffHunk(messageId: string, hunkId: string): void;
  acceptAllDiffHunks(messageId: string): void;
  
  approveToolCall(requestId: string, forSession: boolean): void;
  denyToolCall(requestId: string): void;
  
  createConversation(workspacePath: string, options?: CreateOptions): Promise<string>;
  deleteConversation(id: string): Promise<void>;
  renameConversation(id: string, title: string): Promise<void>;
}
```

## 10.3 Performance Considerations

- **Virtual scrolling**: Use a library like `@tanstack/svelte-virtual` or custom implementation. Each message estimates height from content block count.
- **SQLite WAL mode**: Enable WAL for concurrent reads during streaming writes.
- **FTS5 batching**: Batch FTS index updates — don't update on every streaming chunk, only on message finalization.
- **Content JSON compression**: Messages with large content_json can optionally be compressed with zstd before storage.
- **Image handling**: Images in messages stored as separate blob files referenced by ID, not base64 in JSON.
- **Lazy block rendering**: Content blocks below the viewport are not rendered — only their height placeholder is calculated.
- **Provider connection pool**: Reuse HTTP connections for streaming. Don't reconnect per message.
- **Token counting**: Use WASM-based tokenizers (tiktoken for OpenAI models) running in a Web Worker to avoid blocking the UI thread during context assembly.
