# Runyard — Engineering TODO (v2)

Living engineering task list for Runyard. Granular, dependency-ordered, handoff-ready. Not a rigid waterfall — reorder and split as reality demands.

## How to Use This Document

This is a living task list. Tasks are not a rigid waterfall — they indicate direction, dependencies, and handoff-ready units. Reorder, split, merge, or reprioritize as engineering reality demands. The only invariant is the dependency graph: a `\[P]` task must be at least partially done before its downstream tasks can start.

### Tags

* `\[P]` = Prerequisite — must be done before downstream tasks
* `\[DX]` = Developer experience / polish — can be done any time, but often saves time early
* `\[PERF]` = Performance-critical — schedule before features that depend on it
* `\[CROSS]` = Must work on Windows, macOS, Linux, and mobile browsers
* `\[STRETCH]` = Nice to have, not blocking MVP

### Handoff Style

Each task is a self-contained unit an AI coding agent or engineer can pick up and run. Include: what file(s) to touch, what the deliverable looks like, and what "done" means. If a task gets too big, split it into sub-tasks and update this doc.

## Phase 1 — Core MVP

### 1.1 Sub-Service Daemon (Rust)

> \*\*Goal:\*\* The sub-service is a headless Rust daemon that serves as a "remote brain" for any client. After this section, a single `cargo run --bin subservice` should boot a WebSocket server on port 7820, accept authenticated connections, and serve a fully functional Svelte 5 IDE in any browser. Remote clients can browse files, run terminals, manage Git, and get LSP completions — all over WebSocket — with the exact same API signatures as the desktop Tauri backend. Session isolation and auto-reconnect mean a dropped connection is invisible to the user.

* \[✓] **1.1.1** `\[P]` WebSocket server scaffold — `crates/subservice/src/main.rs`: completed via `tokio-tungstenite` listener on configurable port (default 7820). JSON-RPC 2.0 message envelope. Bearer token auth via `Authorization` header. **Done =** `cargo run --bin subservice` starts and a test client can connect.
* \[✓] **1.1.2** `\[P]` FS proxy — `fs\_list`, `fs\_read`, `fs\_write`, `fs\_watch` over WebSocket. Same signatures as `runyard-core` Tauri commands. Watch events pushed as server notifications. **Done =** a remote client can browse, read, write, and receive file change events.
* \[✓] **1.1.3** `\[P]` PTY proxy — `terminal\_create`, `terminal\_write`, `terminal\_resize`, `terminal\_close`, `terminal\_list` over WebSocket. Session-scoped PTY IDs. Output streamed as server push events. **Done =** remote terminal sessions behave identically to local Tauri PTY.
* \[✓] **1.1.4** `\[P]` Git proxy — All `git\_ops.rs` commands exposed over WebSocket. **Done =** remote git status, stage, commit, branch, worktree operations work via WebSocket.
* \[✓] **1.1.5** `\[P]` LSP proxy — LSP servers run on sub-service host. Proxy JSON-RPC between client and server processes. `lsp:start`, `lsp:send`, `lsp:stop`. Diagnostics pushed async. **Done =** TypeScript server on a remote machine serves completions to the local client.
* \[✓] **1.1.6** Config file — `subservice.toml` parsing: port, auth\_token (bcrypt hash), allowed\_roots, service toggles. Generate on first run with random token. Print connection URL + token on startup. **Done =** first run creates config, subsequent runs read it.
* \[✓] **1.1.7** `\[P] \[CROSS]` Web frontend serving — Embed the built Svelte 5 bundle (`rust-embed`). Serve at root path. Browser loads bundle and connects to same-origin WebSocket. **Done =** `http://localhost:7820` loads the IDE in any browser.
* \[✓] **1.1.8** Session lock — Single active session per auth token. Reject duplicates with session takeover. Return session ID on connect. **Done =** two clients with same token cannot connect simultaneously.
* \[✓] **1.1.9** `\[CROSS]` Auto-reconnect — Client-side reconnection logic: 2s retry fallback in `websocket-client.ts`. Connection state in status bar and re-establishing PTY sessions is still pending. **Done =** disconnect → reconnect happens automatically.
* \[✓] **1.1.10** `\[STRETCH]` Sub-service health endpoint — `GET /health` returns uptime, active sessions, memory usage. **Done =** `curl http://localhost:7820/health` returns JSON.



### 1.2 Remote Connections \& Mobile

> \*\*Goal:\*\* A user can connect the desktop app to any machine running the sub-service daemon, or open the IDE directly in a mobile browser. The connection manager stores credentials securely in the OS keychain (or encrypted localStorage on mobile). The status bar shows connection health at a glance. Remote files appear seamlessly in the explorer, and the mobile layout adapts fully with bottom tabs, collapsible sidebars, and touch-friendly targets. SSH bootstrapping means one-click setup from the desktop to a new remote machine.

* \[✓] **1.2.1** `\[CROSS]` Connection manager — Add/edit/remove remote connections. Stores: host, port, auth token, display name. Credentials in OS keychain (`keyring` crate on desktop, encrypted localStorage fallback on mobile browsers). **Done =** UI panel with add/remove/edit, credentials never in plaintext files.
* \[✓] **1.2.2** `\[CROSS]` Connection status indicator — Status bar: green (connected), yellow (connecting), red (disconnected). Click opens connection manager. **Done =** visual indicator works on desktop and mobile.
* \[✓] **1.2.3** Remote FS in explorer — When connected to remote sub-service, explorer shows remote filesystem. Same UX as local. **Done =** file tree browses remote files, opens remote files in editor.
* \[✓] **1.2.4** `\[CROSS]` Mobile browser layout — Responsive adaptations: collapsible sidebars, bottom tab bar on <768px, touch targets ≥44px, swipe gestures, horizontal scroll for code/diff blocks. **Done =** usable on iPhone Safari and Chrome Android at 320px width.
* \[✓] **1.2.5** SSH bootstrap — Desktop app SSHs into remote machine, downloads/starts sub-service binary, switches to WebSocket. `ssh2` crate. **Done =** one-click remote setup from desktop app.

### 1.3 Chat Panel — Data \& Persistence

> \*\*Goal:\*\* Every chat conversation is durable. SQLite WAL-mode database with FTS5 full-text search stores conversations, messages, branches, and pinned context. The TypeScript type system is the source of truth for all 11 message block types (text, code, diff, tool, etc.). The Rust backend exposes all CRUD operations via Tauri IPC, and the Svelte 5 ChatStore is reactive — loading from SQLite on init, streaming in real time, and persisting every message. No data is lost on crash or restart.

* \[✓] **1.3.1** `\[P]` Chat types — `packages/common/src/chat-types.ts`: `Conversation`, `Branch`, `PinnedContext`, `Message`, `ContentBlock` union (all 11 block types from Chat Panel Spec). **Done =** TypeScript compiles, no `any`.
* \[✓] **1.3.2** `\[P]` Chat SQLite schema — `chat.db`: tables `conversations`, `messages` (content\_json TEXT), `branches`, `pinned\_context`, `messages\_fts` (FTS5). WAL mode. **Done =** schema created, basic CRUD works.
* \[✓] **1.3.3** `\[P]` Rust chat DB layer — `apps/desktop/src-tauri/src/chat\_db.rs`: `chat\_conversation\_list`, `chat\_conversation\_create`, `chat\_conversation\_delete`, `chat\_messages\_load` (paginated, 50/page), `chat\_message\_insert`, `chat\_message\_update`, `chat\_search`, `chat\_branch\_create`, `chat\_pinned\_context\_load/save`. **Done =** all commands callable from frontend via Tauri IPC.
* \[✓] **1.3.4** `\[P]` ChatStore (Svelte 5) — `packages/ui/src/lib/stores/chatStore.svelte.ts`: conversation list, active conversation, messages map, streaming state, context usage derived. **Done =** store reactive, loads from SQLite on init.
* \[✓] **1.3.5** `\[P]` Conversation list sidebar operations — `packages/ui/src/lib/stores/chatStore.svelte.ts`: search/filter/sort by workspace, sort by recent/name/tokens. Rename (double-click title or right-click). Delete with confirmation showing message count and cost. Move conversation to different workspace. Conversation tabs (like editor tabs, each tab = one conversation). **Done =** all list operations work.
* \[✓] **1.3.6** Conversation export — Export as Markdown, JSON, or HTML. Includes all messages, tool traces, and diffs. **Done =** export works for all formats.
* \[✓] **1.3.7** Branch persistence and operations — `chat_db.rs`: `chat_branch_create`, `chat_branch_merge`, `chat_branch_compare`. SQLite schema for branches. Branch tree visualizer data model. **Done =** branches persist, merge and compare work.
* \[✓] **1.3.8** `\[PERF]` FTS batching and WAL optimization — Batch FTS index updates (don't update on every streaming chunk, only on message finalization). SQLite WAL mode for concurrent reads during writes. **Done =** no FTS overhead during streaming.
* \[✓] **1.3.9** `\[PERF]` Content compression and image blob storage — `zstd` compress large content_json before storing. Images stored as separate blob files referenced by ID, not base64 in JSON. **Done =** large content efficient, images separate.
* \[✓] **1.3.10** `\[P]` Pinned context persistence — `pinned_context` table in SQLite. CRUD: `chat_pinned_context_load`, `chat_pinned_context_save`. `PinnedContext` type support. **Done =** pinned context persists across restarts.
* \[✓] **1.3.11** Conversation move and workspace reassignment — Reassign conversation to different workspace. Update all references. **Done =** move works.




### 1.4 Chat Panel — UI \& Rendering

> \*\*Goal:\*\* The chat panel is a first-class tab — not a sidebar widget. It displays a conversation list sidebar + active message stream. Messages render natively: GFM markdown with KaTeX math and Mermaid diagrams, code blocks with copy/insert/apply/explain actions, diffs with per-hunk accept/reject, tool calls with collapsible JSON, thinking blocks with token counts, permission requests with inline approve/deny buttons, file references with clickable chips, and plan cards with progress bars. Streaming is character-by-character, code blocks are detected progressively, and 10,000+ messages scroll smoothly via virtual scrolling. No blank screens, no jank.

* \[ ] **1.4.1** Chat panel container — `ChatPanel.svelte`: conversation list sidebar + active conversation view. Header: model dropdown, branch selector, context bar, clear button. **Done =** panel renders, switches conversations.
* \[ ] **1.4.2** Virtual scrolling — Message list with virtual scrolling. \~20 messages in DOM at once. Smooth scroll at 10k+ messages. **Done =** no jank with 10k messages.
* \[ ] **1.4.3** Text block renderer — GFM markdown with KaTeX math and Mermaid diagrams. **Done =** markdown renders correctly, math and diagrams work.
* \[ ] **1.4.4** Code block renderer — Header bar (language icon, filename, actions: Copy, Insert, Apply, Explain). CodeMirror read-only. Max 400px with expand. **Done =** code blocks look like the spec.
* \[ ] **1.4.5** Diff card renderer — Unified/side-by-side toggle. Per-hunk Accept/Reject. Accept All/Reject All. Green/red colors. Write to disk on accept. **Done =** diff from agent can be accepted and file is modified.
* \[ ] **1.4.6** Tool call \& result renderers — Tool call: collapsed by default, expandable JSON. Tool result: linked to parent, collapsible, truncated if >500 chars. **Done =** tool execution flow is visible and navigable.
* \[ ] **1.4.7** Thinking \& permission blocks — Thinking: collapsed with token count. Permission: always expanded, amber border, Approve/Deny/Approve for Session. **Done =** HIL approval works inline.
* \[ ] **1.4.8** File reference \& plan blocks — File reference: clickable chip with hover preview. Plan card: expandable checklist with progress bar. **Done =** both block types render and are interactive.
* \[ ] **1.4.9** Context summary \& error blocks — Context summary: dashed divider, expandable. Error: red border, error code, retry button. **Done =** both render correctly.
* \[ ] **1.4.10** Streaming behavior — Text streams char-by-char. Code blocks detected progressively. Tool calls appear on detection. Stop button during generation. Regenerate after completion. **Done =** streaming feels responsive, stop works.
* [ ] **1.4.11`[DX]` Message hover actions — Each message card: avatar, timestamp, Copy, Edit (user only), Branch from here, Pin/Unpin, Delete. Message actions on hover. **Done =** all actions work.
* [ ] **1.4.12** Image rendering with lightbox — Images rendered inline in markdown. Click opens lightbox. **Done =** images viewable.
* [ ] **1.4.13`[DX]` Code block word wrap toggle — Word wrap toggle in code block header. **Done =** toggle works.
* [ ] **1.4.14`[DX]` Diff card reject with undo and multi-file batch — After rejecting diff card: strikethrough with 30-second undo button. Multi-file diffs grouped under "Batch change" header. **Done =** reject undo and batch work.
* [ ] **1.4.15** Tool call duration and result size — Show duration (e.g. "Took 1.2s") after completion. Tool result shows size ("Showing first 10KB of 450KB"). Binary download link. **Done =** duration and size visible.
* [ ] **1.4.16`[DX]` Chat panel split, detach, and dock layout — Split view: two conversations side by side. Detach: pop out to floating window. Dock with editor: editor left, chat right. **Done =** all layout modes work.
* [ ] **1.4.17`[DX]` Scroll to bottom and jump to top buttons — Floating "scroll to bottom" button during streaming. "Jump to top" button for long conversations. **Done =** scroll buttons work.
* [ ] **1.4.18`[PERF]` Lazy block rendering — Content blocks below the viewport not rendered — only height placeholder. **Done =** no rendering overhead for off-screen blocks.
* [ ] **1.4.19`[DX]` Markdown table, list, blockquote rendering — Full GFM support: tables, lists, blockquotes, headings. **Done =** all markdown elements render.
* [ ] **1.4.20`[DX]` Message card format — Avatar (left), timestamp (right), content area, branch indicator if fork point. **Done =** message cards look like spec.


### 1.5 Chat Panel — Input \& Context

> \*\*Goal:\*\* The input area is a real editor, not a chat box — multi-line CodeMirror with markdown support, Enter for newlines, Cmd+Enter to send. @-mentions autocomplete files, symbols, skills, agents, and conversations. Drag-and-drop attachments support images, text, and PDF. The prompt enhancer can expand or specify a draft via a fast model. The context window visualizer shows a live token bar (green/amber/red), with auto-compression that summarizes old messages to stay within budget. Context is assembled intelligently: system + skills + pinned context first, then auto-gathered context (editor, tabs, recent files), then history up to 80%, leaving 20% for the response. Branching and conversation search are fully functional. Model switching works mid-conversation with cost preview.

* \[ ] **1.5.1** Input area — Multi-line CodeMirror in markdown mode. Enter=newline, Ctrl/Cmd+Enter=send. Auto-resize 2-20 lines. Character count. **Done =** input feels like a real text editor, not a chat box.
* \[ ] **1.5.2** `@` autocomplete — @file (fuzzy workspace files), @symbol (LSP/Tree-sitter), @skill (available skills), @agent (configured agents), @conversation (cross-reference). Renders as chips. **Done =** all five autocomplete types work.
* \[ ] **1.5.3** Attachments — Drag-drop, clipboard paste, + button. Image, text, PDF support. Chips above input with remove. **Done =** files can be attached and sent.
* \[ ] **1.5.4** Prompt enhancer — Toggle (wand icon). Sends draft to fast/cheap model. Expand/Specify modes. Preview card with Use/Original buttons. **Done =** enhancement works, original preserved.
* \[ ] **1.5.5** Context window visualizer — Token bar: green/amber/red states. Hover breakdown. Auto-compression toggle. Manual Compress and Clear buttons. **Done =** user always knows context usage.
* \[ ] **1.5.6** Context assembly — Priority-ordered builder: system + skills + pinned → auto (editor, tabs, recent) → history (fill 80%) → reserve 20% for response. **Done =** context is assembled correctly every send.
* \[ ] **1.5.7** Context compression — Identify oldest messages exceeding budget. Summarize with fast model. Replace with ContextSummaryBlock. Preserve originals. Expandable. **Done =** compression works, originals retrievable.
* \[ ] **1.5.8** Conversation branching — Fork from any message. Branch tree visualizer. Branch indicator in header. Named branches. **Done =** branching works, tree is navigable.
* \[ ] **1.5.9** Conversation search — FTS5 full-text search across all conversations. Highlight matches. Jump to message. **Done =** search finds messages, jumps correctly.
* \[ ] **1.5.10** Model switching — Per-conversation dropdown. Changeable mid-conversation. Cost preview. Context limit warning. **Done =** switching models works, warnings show.
* [ ] **1.5.11`[DX]` Markdown preview toggle in input — Toggle button in input toolbar shows rendered preview above input. **Done =** preview toggle works.
* [ ] **1.5.12`[DX]` Prompt enhancer cost tracking — Cost of enhancement tracked separately (typically <1% of main model cost). Shows in message footer. **Done =** enhancement cost visible.

* [ ] **1.5.14`[DX]` Context compression settings — Auto-compress toggle (compress when >90% budget). Aggressiveness: Conservative vs Aggressive. Manual compress button. **Done =** settings work.
* [ ] **1.5.15`[DX]` Context clear and reset — Clear context button (preserves system + pinned). Reset conversation (clears everything). Confirmation dialogs. **Done =** clear/reset work.
* [ ] **1.5.16** Conversation move to workspace — Reassign active conversation to different workspace. **Done =** move works.
* [ ] **1.5.17`[DX]` Mobile chat layout — <768px: conversation list slides from left (hamburger). Context bar collapses to token ring. Message actions on long-press. Input sticky at bottom. Autocomplete as bottom sheet. Code blocks/diffs scroll horizontally. **Done =** chat usable on mobile.
* [ ] **1.5.18`[DX]` Chat keyboard shortcuts — Ctrl/Cmd+Enter = send. Ctrl/Cmd+K = focus conversation search. Ctrl/Cmd+Shift+N = new conversation. Ctrl/Cmd+Shift+[ ] = prev/next conversation tab. Ctrl/Cmd+L = focus chat input. Ctrl/Cmd+Shift+C = clear context. Ctrl/Cmd+Shift+B = branch from current message. Escape = close conversation list / autocomplete. **Done =** all shortcuts work.
* [ ] **1.5.19`[DX]` Prompt enhancer preview card — Shows enhanced version with "Use enhanced" / "Send original" buttons. Original draft always preserved. **Done =** preview works.
* [ ] **1.5.20`[DX]` Conversation tab system — Conversation tabs like editor tabs. Each tab = one conversation. Switchable. **Done =** tabs work.


### 1.6 ACP Agent Discovery & Integration

> **Goal:** Runyard is an ACP client, not a provider client. The IDE discovers and connects to ACP-compatible agents installed on the user's machine (or provided by the user). The user can browse available agents, configure them, start/stop them, and switch between them seamlessly. The IDE never speaks directly to LLM providers — all provider-specific logic (API keys, streaming, token counting, tool schemas, context assembly) lives in the agent. This is the Zed model: the IDE is the shell, the agent is the brain. For a YC-stage company, this is the only sane architecture — we build the IDE once, and any ACP-compatible agent plugs in.

> **Architecture Note:** Runyard supports EVERY feature of ACP. All transports (stdio, HTTP, WebSocket). All methods (`initialize`, `authenticate`, `session/new`, `session/load`, `session/resume`, `session/prompt`, `session/cancel`, `session/close`, `session/list`, `session/set_mode`, `session/set_config_option`, `logout`). All notifications (`session/update` with all chunk types: `AgentMessageChunk`, `UserMessageChunk`, `ThoughtChunk`, `toolCall`, `toolCallUpdate`, `plan_update`, `available_commands_update`, `session_info_update`). All capability negotiations. All tool calling flows. All permission request flows. Without a complete ACP implementation, Runyard cannot talk to any agent. ACP is the foundation everything else is built on.

> **Supported Agents (as of July 2026):** Claude Code (`claude-acp`), Codex CLI (`codex-acp`), Gemini CLI (`gemini`), GitHub Copilot (`github-copilot`), Goose (`goose`), Cline (`cline`), OpenCode (`opencode`), Qwen Code (`qwen-code`), Kimi CLI (`kimi`), Junie (`junie`), Roo Code (`roo`), Cursor (`cursor`). Plus any custom ACP agent the user builds or installs.

> **Discovery:** Runyard detects installed agents by scanning well-known paths (e.g., `claude` in PATH, `gemini` in PATH), checking the ACP Registry (`agentclientprotocol.com/overview/agents`), and allowing manual configuration (executable path, spawn command, or remote URL). The user can also provide an executable or configure a remote HTTP/WebSocket endpoint.

- [ ] **1.6.1** `[P]` ACP agent registry data model — `packages/common/src/acp-agent-types.ts`: `AcpAgentConfig`, `AcpAgentStatus`, `AcpAgentCapability`, `AcpAgentDiscoverySource`. SQLite schema: `acp_agents` table with id, name, agent_id, executable_path, spawn_command, remote_url, transport (stdio|http|websocket), env_vars_json, capabilities_json, is_builtin, is_active, created_at, updated_at. **Done =** types defined, schema created, CRUD works.
- [ ] **1.6.2** `[P]` ACP agent discovery engine — Auto-detect installed agents on user's machine: scan PATH for known executables (`claude`, `gemini`, `codex`, `goose`, `goose`, etc.), check ACP Registry API, parse `~/.config/` directories for agent configs. Allow manual add (executable path, spawn command, remote URL + port). Store discovered agents in SQLite. **Done =** common agents auto-detected, manual add works.
- [ ] **1.6.3** `[P]` ACP agent configuration UI — `AcpAgentPanel.svelte`: list of discovered/configured agents. Per-agent: name, agent ID, transport, status (disconnected|connecting|connected|error), capabilities badge. Add new agent button (manual config dialog). Edit agent config (executable, env vars, remote URL). Remove agent. Test connection button. **Done =** agent panel works, all agents manageable.
- [ ] **1.6.4** `[P]` ACP agent launcher — Spawn local agents via stdio (subprocess with piped stdin/stdout). Connect to remote agents via HTTP/WebSocket. Handle agent lifecycle: start → initialize → authenticate (if required) → ready. Monitor process health. Auto-restart on crash. Kill on IDE exit. **Done =** agents launch and connect reliably.
- [ ] **1.6.5`[P]` ACP agent switcher — Per-conversation agent selection. Dropdown in chat panel header: "Claude Code", "Gemini CLI", "Custom Agent...". Switch mid-conversation (starts new session with new agent). Remember last-used agent per project. Default agent setting. **Done =** switching agents works seamlessly.
- [ ] **1.6.6`[P]` ACP agent status monitoring — Real-time status: idle, initializing, ready, processing, error, disconnected. Status in agent panel, chat panel header, and status bar. Error details with retry button. Health check heartbeat. **Done =** agent status visible everywhere.
- [ ] **1.6.7`[P]` ACP agent capabilities display — Show agent capabilities from `InitializeResponse`: supported transports, auth methods, session features, tool capabilities, config options. Badge per capability. Hover for details. **Done =** capabilities visible.
- [ ] **1.6.8`[P]` ACP agent environment variables — Configure env vars per agent (API keys, base URLs, model preferences). Secure storage in OS keychain or encrypted localStorage. Masked display (******). Override at session level. **Done =** env vars configurable and secure.
- [ ] **1.6.9`[P]` ACP agent log viewer — Real-time logs from agent stderr. Filter by level (info, warn, error). Search. Export. Collapsible panel. **Done =** agent logs visible for debugging.
- [ ] **1.6.10`[P]` ACP agent import/export — Export agent config as JSON. Import from file or URL. Share agent configs with team. **Done =** configs portable.

### 1.7 ACP Client Implementation (Rust)

> **Goal:** Runyard implements a complete ACP client in Rust using the official `agent-client-protocol` crate. This is the single most important piece of code in the entire AI layer — without it, Runyard cannot talk to any agent. The client handles ALL ACP protocol features: all transports, all methods, all notifications, all capability negotiations, all tool calling flows, all permission request flows. It is production-grade, fault-tolerant, and exhaustively tested.

> **Why this matters:** ACP is a JSON-RPC 2.0 protocol. The client sends requests, receives responses, and handles notifications. It manages connection lifecycle (stdio subprocess, HTTP, WebSocket), session lifecycle (create, resume, close, list), and streaming (real-time `session/update` notifications). It is the bridge between the IDE and the agent. Every other AI feature (chat, tool calls, diff application, permission requests) flows through this client.

> **Official SDK:** `agent-client-protocol` crate (https://docs.rs/agent-client-protocol, https://github.com/agentclientprotocol/rust-sdk). Includes `agent-client-protocol-schema` (wire types), `agent-client-protocol-tokio` (stdio spawning), `agent-client-protocol-rmcp` (MCP bridge), and `agent-client-protocol-conductor` (proxy chaining).

> **Reference Implementation:** Zed's `crates/agent_servers/src/acp.rs` — spawns agent as subprocess, creates `ClientSideConnection`, sends `InitializeRequest`, handles `session/update` notifications, implements permission requests, terminal lifecycle, file I/O.

- [ ] **1.7.1** `[P]` ACP client crate — `crates/runyard-acp/`: import `agent-client-protocol` crate. Define `RunyardAcpClient` struct wrapping the official client. **Done =** crate compiles, client struct exists.
- [ ] **1.7.2`[P]` ACP stdio transport — Spawn agent as subprocess via `tokio::process::Command`. Pipe stdin/stdout. NDJSON (newline-delimited JSON-RPC) framing. Handle stderr separately (logs). Process health monitoring. Auto-restart on crash. Graceful shutdown (SIGTERM, then SIGKILL). **Done =** stdio transport works with any local ACP agent.
- [ ] **1.7.3`[P]` ACP HTTP transport — Connect to remote ACP agent over HTTP. POST for client→agent. GET with SSE for agent→client streaming. Handle `Acp-Connection-Id`, `Acp-Session-Id`, `Acp-Protocol-Version` headers. Auto-reconnect with exponential backoff. **Done =** HTTP transport works with remote agents.
- [ ] **1.7.4`[P]` ACP WebSocket transport — Connect to remote ACP agent over WebSocket. Upgrade via `GET /acp` with `Upgrade: websocket`. JSON-RPC text frames. Full-duplex. Auto-reconnect. Heartbeat/ping-pong. **Done =** WebSocket transport works with remote agents.
- [ ] **1.7.5`[P]` ACP initialize method — Send `initialize` request with `protocolVersion` ("1.0"), `clientCapabilities` (fs, terminal, session, prompt), `clientInfo` (name: "Runyard", version). Receive `InitializeResponse` with `protocolVersion`, `agentCapabilities`, `agentInfo`, `authMethods`. Validate version compatibility. **Done =** initialization handshake works with all agents.
- [ ] **1.7.6`[P]` ACP authenticate method — Handle auth if required by agent. Support `none`, `bearer`, `apiKey` methods. Store credentials securely. Retry on auth failure. **Done =** auth works for agents that require it.
- [ ] **1.7.7`[P]` ACP session lifecycle — `session/new`: create session with `cwd`, `mcpServers`, `configOptions`. Receive `sessionId`. `session/load`: resume existing session. `session/resume`: resume without replay (v2). `session/list`: list all sessions. `session/close`: close session. `logout`: end auth. Handle session expiration. **Done =** full session lifecycle works.
- [ ] **1.7.8`[P]` ACP session/prompt method — Send `session/prompt` with `sessionId` and user message. Receive `PromptResponse` with `stopReason`. Handle all stop reasons: `stop`, `max_tokens`, `tool_call`, `content_filter`, `error`. **Done =** prompts send and receive responses.
- [ ] **1.7.9`[P]` ACP session/cancel method — Send `session/cancel` to interrupt ongoing processing. Immediate effect. Clean up partial state. **Done =** cancellation works reliably.
- [ ] **1.7.10`[P]` ACP session/update notifications — Handle ALL `session/update` notification types in real-time:
  - `AgentMessageChunk`: text content from agent (stream to chat panel)
  - `UserMessageChunk`: echo of user message (confirm receipt)
  - `ThoughtChunk`: reasoning/thinking content (show in thinking block)
  - `toolCall`: pending tool call request (show permission request)
  - `toolCallUpdate`: completed tool call with content/locations (update tool card)
  - `plan_update`: agent's execution plan (update plan card)
  - `available_commands_update`: updated command list (update suggestions)
  - `session_info_update`: metadata updates (update title, etc.)
  **Done =** all notification types handled, UI updates in real-time. **NOTE:** No notification handling exists.
- [ ] **1.7.11`[P]` ACP tool call handling (agent → client) — Handle `request_permission` from agent. Show inline permission card in chat: tool name, arguments, affected files. User actions: Approve, Deny, Approve for Session. Execute tool via IDE (fs_read, fs_write, terminal_create, etc.). Return result to agent. **Done =** tool calls flow correctly.
- [ ] **1.7.12`[P]` ACP client capabilities — Advertise capabilities during `initialize`: `fs:readTextFile`, `fs:writeTextFile`, `terminal:create`, `terminal:output`, `terminal:wait_for_exit`, `terminal:kill`, `terminal:release`. `sessionCapabilities: list, resume, close`. `promptCapabilities: streaming, toolCalls`. **Done =** capabilities advertised correctly.
- [ ] **1.7.13`[P]` ACP config options — Handle `session/set_config_option` and `session/set_mode`. Support model switching, reasoning level changes, mode changes. Pass config changes to agent. Update UI to reflect agent's supported options. **Done =** config options work.
- [ ] **1.7.14`[P]` ACP connection pooling — Manage multiple agent connections simultaneously. Each connection independent. Pool limits (max 10 concurrent). Resource cleanup on disconnect. Connection health checks. **Done =** multiple agents work concurrently.
- [ ] **1.7.15`[P]` ACP error handling — Handle all JSON-RPC errors: parse errors, invalid requests, method not found, invalid params, internal errors. ACP-specific errors: version mismatch, capability mismatch, session not found, auth failed. Retry with backoff. User-friendly error messages. **Done =** errors handled gracefully.
- [ ] **1.7.16`[P]` ACP conductor integration — Use `agent-client-protocol-conductor` for proxy chaining. Middleware: request/response logging, permission interception, tool filtering, cost tracking. Chain multiple middlewares. **Done =** conductor middleware works.
- [ ] **1.7.17`[P]` ACP event bridge to frontend — Bridge ACP events from Rust backend to Svelte 5 frontend. Events: message chunks, tool calls, permission requests, status updates, errors. Use Tauri IPC events (desktop) or WebSocket (web). **Done =** frontend receives all ACP events in real-time.
- [ ] **1.7.18`[P]` ACP test suite — Unit tests for each method. Integration tests with mock agent. End-to-end tests with real agents (Claude Code, Gemini CLI). Test all transports. Test all notification types. Test error scenarios. **Done =** comprehensive test coverage.
- [ ] **1.7.19`[P]` ACP protocol version negotiation — Handle v1 (current stable). Detect v2 (draft RFD). Negotiate compatible version. Fallback to v1 if v2 not supported. Log version used. **Done =** version negotiation works.
- [ ] **1.7.20`[P]` ACP MCP bridge (agent-level) — Use `agent-client-protocol-rmcp` crate to bridge ACP agent to MCP servers. The IDE passes MCP server config to agent via `session/new`. The agent uses `rmcp` to connect to MCP servers. The IDE does NOT directly interact with MCP — it only sees ACP notifications about tool calls. **Done =** ACP agent can use MCP tools transparently.

### 1.8 MCP Server Manager UI (IDE Configuration Only)

> **Goal:** The IDE provides a UI for managing MCP server configurations. The IDE stores these configs and passes them to the agent via ACP `session/new` or `session/set_config_option`. The agent (not the IDE) connects to MCP servers using `rmcp` or `rust-mcp-sdk`. The IDE learns about MCP tool calls through ACP `session/update` notifications (agent reporting what it did) or ACP `toolCall` requests (agent requesting tool execution via IDE's permission system). This is configuration-only — the IDE does not speak MCP directly.

> **MCP is an agent-level protocol, not an IDE-level protocol.** The IDE's responsibility: let users add, edit, and remove MCP server configs. The agent's responsibility: connect to MCP servers, discover tools, call tools, handle results. The IDE sees the results through ACP.

> **Reference:** `rmcp` crate (https://crates.io/crates/rmcp, v2.2.0, official Rust SDK). `rust-mcp-sdk` (https://github.com/rust-mcp-stack/rust-mcp-sdk, v0.9.0, community). See Protocol Research Reference doc for full MCP details.

- [ ] **1.8.1`[P]` MCP config data model — `packages/common/src/mcp-config-types.ts`: `McpServerConfig`, `McpTransport` (stdio|http|websocket), `McpAuth`. SQLite schema: `mcp_servers` table with id, name, transport, command, url, env_vars_json, auth_json, is_global, is_active, project_id. **Done =** types defined, schema created.
- [ ] **1.8.2`[P]` MCP server manager UI — `McpServerPanel.svelte`: list of configured MCP servers. Per-server: name, transport, status (configured|active|error), health indicator. Add server button: dialog with transport selection, command/URL input, env vars, auth. Edit/remove servers. Test connection button. Import/export config. **Done =** MCP server panel works.
- [ ] **1.8.3`[P]` MCP server config → ACP pass-through — When user starts a chat session, pass `mcpServers` array to agent via ACP `session/new`. Agent receives configs and connects to MCP servers. IDE shows "MCP servers connected: 3" badge in chat header. **Done =** configs passed to agent.
- [ ] **1.8.4`[P]` MCP tool call display in chat — When agent calls MCP tools, agent sends `toolCall` via ACP `session/update`. IDE renders tool call card: tool name, server name, arguments, status (running|completed|failed). Expandable JSON. Linked to parent message. **Done =** tool calls visible in chat.
- [ ] **1.8.5`[P]` MCP tool permission integration — Agent may request permission for MCP tool calls via ACP `request_permission`. IDE shows permission card with tool details. User approves/denies. IDE records permission decision. **Done =** MCP tool permissions work.
- [ ] **1.8.6`[P]` MCP server health monitoring — Agent reports MCP server health via ACP `session/update`. IDE shows server status: green (connected), yellow (starting), red (error). Hover for details. Auto-retry on failure. **Done =** health monitoring visible.
- [ ] **1.8.7`[P]` MCP server global vs per-project — Global servers apply to all projects. Per-project servers only for specific project. UI toggle. SQLite stores scope. **Done =** scoping works.
- [ ] **1.8.8`[P]` MCP server import from registry — Browse MCP Registry (`github.com/modelcontextprotocol/registry`). One-click install. Auto-configure transport. Popular servers: GitHub, PostgreSQL, Filesystem, Browser, Slack. **Done =** registry import works.
- [ ] **1.8.9`[P]` MCP server config validation — Validate command exists (stdio). Validate URL reachable (HTTP). Validate auth credentials. Test connection on add. Show error with fix suggestion. **Done =** validation works.
- [ ] **1.8.10`[P]` MCP server env var security — Store API keys, tokens in OS keychain. Masked display. Never in plaintext. Secure pass-through to agent. **Done =** env vars secure.

### 1.9 Skills System

> **Goal:** Skills are self-contained instructions that make agents smarter — a `SKILL.md` file with YAML frontmatter + markdown body, plus optional scripts, references, and assets. The IDE manages the skill files (create, edit, delete, organize, validate). The agent reads, triggers, and executes skills. Skills load progressively: metadata (~100 tokens) always in context, full body on trigger, scripts on demand — so the agent is never bloated with unused skill text. Users trigger skills in chat via `@skill:<name>`. Skills are cross-compatible: load from `.claude/skills/`, `.cursor/skills/`, `.agents/skills/`, and work across Claude Code, Cursor, Codex, and other clients.

> **Reference:** agentskills.io/specification.md, github.com/anthropics/skills. See Protocol Research Reference doc for full details.

- [ ] **1.9.1`[P]` Skills data model — `packages/common/src/skills-types.ts`: `SkillManifest`, `SkillMetadata`, `SkillDirective`, `SkillScope` (global|project|nested). Directory structure: `SKILL.md` with YAML frontmatter + markdown body. `scripts/`, `references/`, `assets/` subdirs. SQLite schema: `skills` table with id, name, description, scope, directory_path, frontmatter_json, is_builtin, created_at, updated_at. **Done =** types defined, schema created, directory structure validated.
- [ ] **1.9.2`[P]` Skills CRUD UI — `SkillsPanel.svelte`: list by scope (global/project). Create from template (10+ templates: code-review, debug, refactor, test, docs, deploy, etc.). Edit with markdown editor + frontmatter form (live validation). Preview rendered skill. Delete with confirmation. File watcher auto-reload. **Done =** skills manageable in UI.
- [ ] **1.9.3`[P]` Skills chat integration — `@skill:<name>` autocomplete in chat input. Fuzzy match. Show description tooltip. Render as chip in input. On send, pass skill names to agent via ACP `session/prompt` metadata. Agent loads and triggers skills. **Done =** skills trigger and inject correctly.
- [ ] **1.9.4`[P]` Skills directory scanning — Scan `~/.claude/skills/`, `.claude/skills/` (project), `.agents/skills/`, `.cursor/skills/` on startup and file change. Parse `SKILL.md` frontmatter. Validate naming (lowercase, hyphens, max 64 chars). Detect duplicates. Load into SQLite. **Done =** scanning works, all directories covered.
- [ ] **1.9.5`[P]` Skills YAML frontmatter validation — Validate required fields: `name`, `description`. Validate constraints: name matches directory, 1-64 chars, lowercase alphanumeric + hyphens. Validate optional fields: `when_to_use`, `arguments`, `allowed-tools`, `model`, `effort`, `shell`, `license`. Size check: body < 500 lines. Lint errors shown in UI. **Done =** validation works, errors shown.
- [ ] **1.9.6`[P]` Skills progressive disclosure metadata — Extract Level 1 (metadata: name + description, ~100 tokens) from all skills. Send to agent as catalog via ACP `session/new` or `session/set_config_option`. Agent uses catalog to decide which skills to trigger. **Done =** metadata always available to agent.
- [ ] **1.9.7`[P]` Skills cross-compatibility loading — Load skills from `.claude/skills/` (Claude Code), `.cursor/skills/` (Cursor 2.0+ MDC hybrid), `.agents/skills/` (cross-client convention). Convert Cursor MDC frontmatter to standard format. Save in standard format. **Done =** skills portable across clients.
- [ ] **1.9.8`[P]` Skills template library — Built-in templates: `code-review`, `debug`, `refactor`, `test-generation`, `documentation`, `deployment`, `api-migration`, `security-audit`, `performance-optimize`. Each with pre-filled SKILL.md, scripts, references. **Done =** templates available.
- [ ] **1.9.9`[P]` Skills file watcher — Watch skill directories for changes. Auto-reload on add/edit/delete. Notify agent of changes. Update UI in real-time. **Done =** watcher works.
- [ ] **1.9.10`[P]` Skills CLI validation integration — Integrate `claude-skills-cli` (npm) for validation: `pnpx claude-skills-cli validate .claude/skills/my-skill`. Show validation results in UI. Support strict/lenient/loose modes. **Done =** CLI validation integrated.

### 1.10 Agent Manager (ACP Agent Monitoring)

> **Goal:** The agent manager is mission control for all ACP agent activity. A Kanban board shows agent tasks in columns: Queued, Running, Awaiting HIL (Human-in-the-Loop), Completed, Failed. Each card shows the agent name, task description, project, elapsed time, and current status. Multiple agents can run in parallel with per-project and global concurrency limits. The agent manager tracks costs reported by agents via ACP `session/update` notifications. Every task is tracked, every dollar is accounted for, and nothing is lost. This is for EXTERNAL ACP agents only — the built-in agent (Phase 2) has its own management system.

> **Note:** This is NOT the orchestrator. The orchestrator (Phase 2) is a built-in agent that uses A2A to delegate to subagents. This section is about managing the external ACP agents that the IDE connects to.

- [ ] **1.10.1`[P]` Agent task model — `packages/common/src/agent-task-types.ts`: `AgentTask`: id, agent_id, conversation_id, project, description, status (queued|running|awaiting_hil|completed|failed), timestamps, cost, current_tool, error. SQLite schema: `agent_tasks` table. `AgentManagerStore` in Svelte 5. **Done =** model defined, store reactive.
- [ ] **1.10.2`[P]` Kanban UI — `AgentManagerPanel.svelte`: columns: Queued, Running, Awaiting HIL, Completed, Failed. Cards: agent name, task, project, elapsed time, cost, current tool. Drag to reorder queued. Click card for details. **Done =** kanban board works, cards interactive.
- [ ] **1.10.3`[P]` Parallel agent management — Launch multiple ACP agents simultaneously. Per-project and global concurrency limits (default 3 concurrent). Kill/pause/requeue per task. Queue overflow handling. **Done =** multiple agents run concurrently, limits enforced.
- [ ] **1.10.4`[P]` Agent cost tracking — Parse cost information from ACP `session/update` notifications. Track per-task, per-conversation, per-agent, per-project. Running total. Budget alerts. **Done =** costs tracked and visible.
- [ ] **1.10.5`[P]` Agent HIL (Human-in-the-Loop) queue — When agent requests permission via ACP `request_permission`, show in Awaiting HIL column. Inline approve/deny in chat AND in agent manager. Batch approve multiple requests. **Done =** HIL queue manageable.
- [ ] **1.10.6`[P]` Agent task history — Completed and failed tasks stored. Search/filter. Replay: show what agent did step by step. Export as JSON. **Done =** history browsable and replayable.
- [ ] **1.10.7`[P]` Agent status dashboard — Global view: active agents, total tasks, total cost, average response time. Per-agent: uptime, tasks completed, success rate. Charts: tasks over time, cost over time. **Done =** dashboard visible.
- [ ] **1.10.8`[P]` Agent error handling and recovery — When agent fails: show error in Failed column. Retry button. Auto-retry on transient errors (max 3). Escalate to user on persistent failures. **Done =** errors handled gracefully.
- [ ] **1.10.9`[P]` Agent notification system — Desktop notifications for: task completed, HIL required, error occurred, cost threshold reached. Configurable per agent. Do Not Disturb mode. **Done =** notifications delivered.
- [ ] **1.10.10`[P]` Agent routing visualizer — Graph view: nodes = agents, edges = task handoffs (when one agent delegates to another). Collapsible sidebar. Zoom/pan. **Done =** graph shows agent communication.


### 1.11 Productivity Panels

> \*\*Goal:\*\* Three first-class panels for developer productivity: a Notes tab with a markdown editor + live preview (per-project, auto-saved), a TODO tab with checkbox items that agents can auto-populate from their plans, and a Diff viewer tab that handles side-by-side and unified diffs from any source — agent edits, Git diffs, or file comparisons. All three panels are persistent, polished, and feel like native parts of the IDE, not afterthoughts.

* \[ ] **1.11.1** Notes tab — Markdown editor with live preview. Per-project. Auto-save. Toolbar. **Done =** notes editable, persisted, preview works.
* \[ ] **1.11.2** TODO tab — Checkbox items. Agents auto-add. Manual add/edit/reorder/delete. Persisted in SQLite. **Done =** task list works, agent integration works.
* \[ ] **1.11.3** Diff viewer tab — Standalone side-by-side/unified diff. Per-hunk accept/reject. Syntax highlighting. Open from agent, git, or file comparison. **Done =** diff viewer works for all sources.

### 1.12 Editor Advanced Features

> \*\*Goal:\*\* The editor is a CodeMirror 6 powerhouse with Vim mode, Emacs mode, enhanced multi-cursor (Cmd+D, Cmd+Shift+L, Alt+Click), a scroll-synced minimap, code folding, breadcrumbs with clickable file path segments and symbol navigation, sticky scroll that pins the current scope to the top, Git worktree management, workspace management for multiple independent contexts, and inline diffs that show agent edits as gutter annotations with accept/reject per hunk. It feels like a native code editor, not a web toy.

* \[\~] **1.12.1** Vim mode — `@codemirror/vim` or custom. Toggle via settings/Cmd+Shift+V. Visual/insert/normal modes. Status bar indicator. **Done =** Vim keybindings work.
* \[ ] **1.12.2** Emacs mode — Emacs keybindings via CodeMirror. Toggle via settings. **Done =** Emacs keybindings work.
* \[ ] **1.12.3** Enhanced multi-cursor — `Cmd+D` (select next occurrence), `Cmd+Shift+L` (select all). `Alt+Click` visual multi-cursor. **Done =** multi-cursor editing works.
* \[ ] **1.12.4** Minimap — CodeMirror minimap extension. Toggle. Scroll-synced. **Done =** minimap visible, synced.
* \[ ] **1.12.5** Code folding — Fold by indentation + syntax. Fold all / unfold all commands. **Done =** code folds correctly.
* \[ ] **1.12.6** Breadcrumbs — File path above editor (clickable segments). Symbol breadcrumb at cursor. **Done =** breadcrumbs show and navigate.
* \[ ] **1.12.7** Sticky scroll — Pin current scope to top on scroll. CodeMirror extension. **Done =** scope sticks on scroll.
* \[\~] **1.12.8** `\[P]` Git worktree UI — Create, list, switch, remove worktrees. Dialog for create. Each worktree = separate workspace. Status bar indicator. **Done =** worktree management in UI.
* \[ ] **1.12.9** Workspace management — Multiple workspaces open. Switcher (Cmd+Shift+W). Each workspace: independent layout, terminals, open files. **Done =** multiple workspaces work.
* \[ ] **1.12.10** Inline diff in editor — Agent edits show as gutter annotations with accept/reject per hunk. Not just in chat panel. **Done =** inline diff works in the editor itself.

### 1.13 Keyboard & Commands

> \*\*Goal:\*\* Keyboard-driven power users are first-class. A fully configurable keybinding system stores custom bindings in `\~/.runyard/keybindings.json` with platform-aware defaults (Ctrl on Linux/Windows, Cmd on macOS). Import bindings from VS Code, JetBrains, or Sublime. Conflict detection warns before overwriting. The command palette is polished with recent commands scored higher, platform-correct keybindings displayed inline, and categories with icons. Quick file open (Cmd/Ctrl+P) is fuzzy, fast, and previews files on highlight. Every shortcut feels native to the platform.

* \[\~] **1.13.1** `\[CROSS]` Keybinding system — `\~/.runyard/keybindings.json`. Platform-aware defaults (Ctrl/Cmd). Import from VS Code/JetBrains/Sublime. Conflict detection. **Done =** keybindings configurable, imports work.
* \[\~] **1.13.2** `\[CROSS]` Command palette polish — Recent commands scored higher. Platform-correct keybindings inline. Recently used section. Categories with icons. **Done =** palette feels polished.
* \[\~] **1.13.3** `\[CROSS]` Quick file open — `Cmd/Ctrl+P`. Fuzzy search all files. Preview on highlight. Open in current tab or new split. **Done =** file open is fast and useful.

### 1.14 Performance & Polish

> \*\*Goal:\*\* The app is fast and stable. Tauri IPC calls are batched to reduce roundtrips. CodeMirror instances are created lazily (only when a tab is visible) and destroyed on close to keep RAM low. 10MB+ files open in under 100ms. Cold startup is under 1.5 seconds on all platforms. RAM stays under 400MB with 3 LSP servers, 5 open files, and a terminal running. Error boundaries catch failures gracefully — no blank screens, just inline error states with retry buttons. Loading states use skeleton screens and progress indicators, never infinite spinners. When a stretch goal is reached, the sub-service gains multi-user support with admin roles, per-project permissions, and audit logs.

* \[ ] **1.14.1** `\[PERF] \[CROSS]` IPC batching — Batch rapid Tauri IPC calls. Reduce LSP and file watcher roundtrips. **Done =** fewer IPC calls, lower latency.
* \[\~] **1.14.2** `\[PERF]` Editor lazy loading — Create CodeMirror only when tab visible. Destroy on close. Pool instances. **Done =** RAM drops when tabs closed.
* \[ ] **1.14.3** `\[PERF]` Large file performance — Verify <100ms open for 10MB+ files. Profile and optimize. **Done =** 10MB files open fast.
* \[ ] **1.14.4** `\[PERF] \[CROSS]` Startup optimization — Profile on all OSes. Lazy-load plugins. Defer non-critical init. Mobile: code split, minimize initial JS. **Done =** cold start <1.5s.
* \[ ] **1.14.5** `\[PERF] \[CROSS]` RAM profiling — Target <400MB with 3 LSP + 5 files + terminal. <150MB base. Mobile: memory pressure monitoring, tab unloading. **Done =** RAM within targets.
* \[\~] **1.14.6** `\[DX]` Error boundaries — Graceful errors in all panels. Inline states, not blank screens. Retry/reload buttons. **Done =** no blank screens on error.
* \[\~] **1.14.7** `\[DX]` Loading states — Skeleton screens for panels. Progress indicators for long ops. No infinite spinners. **Done =** loading feels responsive.
* \[ ] **1.14.8** `\[STRETCH]` Sub-service multi-user system — Admin user management, roles (admin/dev/viewer/agent-only), per-project permissions, session management, audit logs. **Done =** multiple users can connect to one sub-service with different permissions.

### 1.15 Agentic Workspaces

> \*\*Goal:\*\* The "Agentic Workspaces" feature is the core differentiator of Runyard as an Agentic AI Code Editor. Developers can assign complex, multi-step objectives (e.g., "migrate a legacy API endpoint", "write an entire integration test suite") to autonomous agents. Agents operate in a secure sandbox, read the codebase, write and refactor code, execute terminal commands, and self-correct based on error logs. They present a clean, unified pull request for review. This is not just chat assistance — it is autonomous software engineering.

#### 1.15.1 Workspace \& Objective Model

* \[ ] **1.15.1.1** `\[P]` Agentic Workspace data model — `packages/common/src/agentic-workspace-types.ts`: `Workspace`, `Objective`, `Task`, `AgentRun`, `ExecutionPlan`, `Artifact` (code, diff, test-result, error-log, PR). SQLite schema: `workspaces`, `objectives`, `tasks`, `agent\_runs`, `artifacts`, `execution\_logs` tables. **Done =** types compile, schema created, CRUD works.
* \[ ] **1.15.1.2** `\[P]` Workspace UI — `WorkspacesPanel.svelte`: list of active workspaces, each with objective description, status, progress, agent count, cost. New workspace button. Workspace cards: status badge, current phase, elapsed time, last agent action. **Done =** workspace list visible, cards interactive.
* \[ ] **1.15.1.3** `\[P]` Objective creation flow — Modal: objective description (free-form or templates), target scope (file, directory, entire project), agent type (code-writer, test-writer, migrator, architect), priority, deadline. Template library: "Migrate API endpoint", "Write integration tests", "Refactor to TypeScript", "Add error handling", "Extract reusable component". **Done =** objectives can be created with templates.
* \[ ] **1.15.1.4** `\[P]` Objective scope parser — Parse user description into structured plan. File targets from @mentions, directory targets from glob patterns, semantic search for relevant files. Auto-generate execution plan: steps, estimated files, dependencies. **Done =** objectives parsed into structured plans with file targets.
* \[ ] **1.15.1.5** `\[P]` Execution plan UI — Tree view of plan steps. Each step: description, status (pending|running|completed|failed), assigned agent, time spent, cost. Expandable details: file list, expected output, actual output. Progress bar per step and overall. **Done =** plan tree visible, progress tracked.
* \[ ] **1.15.1.6** Objective editing and cancellation — Edit objective description, scope, deadline. Cancel running objective with graceful agent shutdown. Save partial results. **Done =** objectives editable and cancellable.
* \[ ] **1.15.1.7** Workspace history and replay — Completed workspaces stored. Replay execution: show what agents did, step by step. Fork workspace: start new objective from same base. **Done =** history browsable, replay works.

#### 1.15.2 Multi-Agent Orchestration

* \[ ] **1.15.2.1** `\[P]` Agent registry and types — `AgentRegistry` in Rust. Agent types: `CodeWriter` (write/refactor code), `TestWriter` (generate tests), `CodeReviewer` (review code quality), `Architect` (plan structure), `Debugger` (fix errors), `MigrationAgent` (API migration), `DevOps` (terminal/CI). Each with capability score, cost model, tool access. **Done =** registry lists agents, types selectable.
* \[ ] **1.15.2.2** `\[P]` Orchestrator engine — `Orchestrator` in Rust. Receives objective, decomposes into tasks, assigns to agents, monitors progress, handles failures, coordinates handoffs. Uses A2A for agent communication. Bulletin board pattern (not direct chat). State machine: planning → assigning → executing → reviewing → self-correcting → finalizing. **Done =** orchestrator runs, tasks assigned, state machine works.
* \[ ] **1.15.2.3** `\[P]` Task decomposition — LLM-based decomposition of objectives into tasks. Each task: description, input files, expected output, success criteria, retry policy. Task dependency graph (DAG). Parallel execution where possible. **Done =** objectives decomposed into task DAGs.
* \[ ] \*\*1.15.2.4`\[P]` Agent assignment and scheduling — Match tasks to agent capabilities. Queue management. Per-workspace and global concurrency limits. Cost-based scheduling (cheaper first). Deadline-aware prioritization. **Done =** tasks assigned optimally, limits enforced.
* \[ ] **1.15.2.5** `\[P]` Agent handoff protocol — Structured context passing between agents. Include: file state, test results, error logs, previous decisions, rationale. Not free-form chat — structured JSON with context window limits. **Done =** handoffs carry full context efficiently.
* \[ ] **1.15.2.6** Parallel agent execution — Multiple agents run simultaneously within a workspace. Shared workspace directory (read-only or copy-on-write). No file conflicts: agent locks files during edit. Merge strategy for concurrent edits. **Done =** parallel execution safe, no conflicts.
* \[ ] **1.15.2.7** Agent conflict resolution — When two agents edit the same file, detect conflict. Three-way merge or manual resolution. Show conflict UI with both versions and merge suggestions. **Done =** conflicts detected and resolved.
* \[ ] **1.15.2.8** Workspace dashboard — Global view of all active workspaces. Agent activity feed (real-time). Cost tracking per workspace. Completion timeline. Agent utilization graph. **Done =** dashboard shows live workspace activity.
* \[ ] **1.15.2.9** Agent status indicators — Real-time status per agent: idle, planning, reading, writing, testing, waiting for HIL, error. Status in workspace panel, orchestrator UI, and status bar. **Done =** agent status visible everywhere.
* \[ ] **1.15.2.10** Workspace notifications — Push notifications for workspace events: objective completed, task failed, HIL required, cost threshold reached. Configurable per workspace. **Done =** notifications delivered.

#### 1.15.3 Code Reading, Writing \& Refactoring

* \[ ] **1.15.3.1** `\[P]` Codebase analysis engine — Read and parse entire codebase. Build AST index, dependency graph, call graph, type map. Identify entry points, API boundaries, data flows. Export analysis as structured JSON for agents. **Done =** codebase analyzed, indexed.
* \[ ] **1.15.3.2** `\[P]` Code reading agent — Agent reads files, directories, and symbols. Respects `.gitignore`. Follows imports to understand dependencies. Generates summary reports: "This file handles authentication. These 5 files depend on it." Uses AST for accurate symbol understanding. **Done =** agents read code accurately.
* \[ ] \*\*1.15.3.3`\[P]` Code writing agent — Agent writes new code based on objective. Generates complete files or precise edits. Follows project conventions (style, naming, patterns). Uses edit blocks for surgical changes. Preserves existing code structure. **Done =** agents write code that matches project style.
* \[ ] \*\*1.15.3.4`\[P]` Code refactoring agent — Agent modifies existing code. Refactor patterns: extract function, rename symbol, inline variable, move to module, convert to async, add TypeScript types, migrate API. Safe refactor: preserves behavior, runs tests to verify. **Done =** refactoring safe, behavior preserved.
* \[ ] \*\*1.15.3.5`\[P]` Code review agent — Agent reviews code for quality, bugs, security, performance. Checklist: error handling, input validation, race conditions, memory leaks, SQL injection, XSS. Generates review report with severity levels and suggestions. **Done =** code review reports generated.
* \[ ] \*\*1.15.3.6`\[P]` Test generation agent — Agent writes tests for existing code. Unit tests, integration tests, e2e tests. Covers edge cases, error paths, boundary conditions. Generates test plan before writing. Runs tests to verify they pass. **Done =** tests generated, run, and pass.
* \[ ] \*\*1.15.3.7`\[P]` API migration agent — Agent migrates legacy API to new version. Reads old API usage, understands new API docs, updates call sites, handles breaking changes, adds compatibility layers, updates types. **Done =** API migrations complete and verified.
* \[ ] \*\*1.15.3.8`\[P]` Documentation generation agent — Agent writes code comments, docstrings, README updates, API docs. Follows project documentation style. Updates docs when code changes. **Done =** docs generated and updated.
* \[ ] \*\*1.15.3.9`\[P]` Code search and retrieval — Semantic search over codebase for agents. Find relevant files, functions, symbols for a given task. Rank by relevance. Inject top results into agent context. **Done =** agents find relevant code efficiently.
* \[ ] \*\*1.15.3.10`\[P]` Code diff and apply — Generate unified diffs from agent edits. Show in diff viewer with per-hunk accept/reject. Apply accepted changes to workspace. Rollback rejected changes. **Done =** diffs generated, applied, rolled back.

#### 1.15.4 Terminal Execution \& Self-Correction

* \[ ] **1.15.4.1** `\[P]` Sandboxed terminal execution — Agents execute terminal commands in isolated sandbox. Commands: test runners, build tools, linters, formatters, package managers. Timeout: 5 minutes default. Kill on timeout. Capture stdout, stderr, exit code. **Done =** commands execute safely, output captured.
* \[ ] **1.15.4.2** `\[P]` Test execution loop — Agent writes code → runs tests → parses results. If tests pass: continue. If tests fail: read error logs, identify failing assertions, fix code, re-run tests. Max 5 retries per task. Report test results. **Done =** test loop self-corrects.
* \[ ] **1.15.4.3** `\[P]` Lint and build loop — Agent writes code → runs linter (eslint, clippy, etc.) → fixes errors. Runs build (tsc, cargo build, etc.) → fixes compilation errors. Max 5 retries. Report build status. **Done =** lint/build loop self-corrects.
* \[ ] \*\*1.15.4.4`\[P]` Error log parsing — Parse test output, stack traces, compiler errors, linter warnings. Extract: file, line, error type, message, suggestion. Feed structured errors to agent for self-correction. **Done =** errors parsed into structured format.
* \[ ] \*\*1.15.4.5`\[P]` Self-correction strategy — Agent receives error, identifies root cause, generates fix, applies fix, re-runs verification. If fix fails after 5 retries: escalate to user (HIL). Log all attempts. **Done =** self-correction works, escalation on failure.
* \[ ] \*\*1.15.4.6`\[P]` Verification agent — Separate verifier agent (not the maker) checks if task is complete. Criteria: tests pass, lint clean, build succeeds, no regressions, matches objective. Independent verification prevents self-grading bias. **Done =** verification independent and accurate.
* \[ ] \*\*1.15.4.7`\[P]` Dependency installation — Agent installs dependencies (npm, cargo, pip, etc.) in sandbox. Detects package.json, Cargo.toml, requirements.txt. Runs appropriate install command. Verifies installation. **Done =** dependencies installed automatically.
* \[ ] \*\*1.15.4.8`\[P]` Environment setup — Agent configures environment variables, config files, database migrations, seed data. Reproducible environment setup for each workspace. **Done =** environments reproducible.
* \[ ] \*\*1.15.4.9`\[P]` CI/CD simulation — Run CI pipeline locally (GitHub Actions, GitLab CI). Parse workflow files. Execute steps in sandbox. Report CI status. **Done =** CI simulated locally.
* \[ ] \*\*1.15.4.10`\[P]` Performance testing — Agent runs benchmarks, profiles code, identifies bottlenecks. Compare before/after performance. Report metrics: execution time, memory usage, CPU usage. **Done =** performance tests run and reported.

#### 1.15.5 Secure Sandbox

* \[ ] \*\*1.15.5.1`\[P]` Sandboxed execution environment — MicroVM (Firecracker/Kata) or gVisor for agent execution. Default-deny network egress. Read-only workspace mount (copy-on-write for writes). No access to host filesystem outside workspace. **Done =** agents cannot escape sandbox.
* \[ ] \*\*1.15.5.2`\[P]` Permission model — Per-agent permissions: read-files, write-files, execute-terminal, network-access, git-access. Per-workspace permissions: read-only, read-write, full. User approval for sensitive operations: network access, destructive commands, git push. **Done =** permissions enforced, approvals prompted.
* \[ ] \*\*1.15.5.3`\[P]` Credential isolation — Agent credentials (API keys, tokens) isolated from host. Short-lived credential brokering. No secrets in agent memory. Audit log of all credential usage. **Done =** credentials isolated and audited.
* \[ ] \*\*1.15.5.4`\[P]` Network sandboxing — Default-deny outbound network. Whitelist specific domains if needed (package registries, API docs). Proxy all traffic through host. Log all network requests. **Done =** network controlled and logged.
* \[ ] \*\*1.15.5.5`\[P]` Resource limits — CPU: 2 cores per agent. RAM: 2GB per agent. Disk: 5GB per workspace. Network: 100MB/hour. Time: 30 minutes per task. Enforce via cgroups. **Done =** resource limits enforced.
* \[ ] \*\*1.15.5.6`\[P]` Audit logging — Immutable log of all agent actions: command executed, file read/written, network request, API call. Log format: timestamp, agent ID, action, arguments, result, file hash. Store in append-only log. **Done =** all actions logged immutably.
* \[ ] \*\*1.15.5.7`\[P]` Workspace isolation — Each workspace is an isolated filesystem. Copy-on-write from base project. Agents work in workspace copy. No cross-workspace access. **Done =** workspaces isolated.
* \[ ] \*\*1.15.5.8`\[P]` Secure code execution — Sandboxed Python, JavaScript, Rust execution. No eval(), no exec(), no dynamic code loading. AST validation before execution. **Done =** code execution safe.
* \[ ] \*\*1.15.5.9`\[P]` Malicious pattern detection — Detect suspicious patterns in agent-generated code: infinite loops, file deletion, system calls, network requests, credential access. Flag and block. **Done =** malicious patterns blocked.
* \[ ] \*\*1.15.5.10`\[P]` Sandbox health monitoring — Monitor sandbox health: CPU, memory, disk, network. Kill unhealthy sandboxes. Auto-restart. Report health to orchestrator. **Done =** sandbox health monitored.

#### 1.15.6 Pull Request Generation \& Review

* \[ ] \*\*1.15.6.1`\[P]` PR generation — Generate clean, unified pull request from workspace changes. Diff against base branch. Include: title, description, change summary, affected files, test results, performance metrics. Auto-generate PR description from objective. **Done =** PRs generated with full context.
* \[ ] \*\*1.15.6.2`\[P]` PR diff review — Interactive diff viewer for generated PR. Side-by-side or unified. Per-file review. Line comments. Approve/reject per file. Overall approve/reject. **Done =** PR diff reviewable.
* \[ ] \*\*1.15.6.3`\[P]` PR merge readiness — Checklist: tests pass, lint clean, build succeeds, no conflicts, no secrets, review approved. Show merge readiness badge. **Done =** merge readiness assessed.
* \[ ] \*\*1.15.6.4`\[P]` PR publishing — Publish PR to GitHub/GitLab. Use OAuth/PAT. Create PR via API. Link to CI/CD pipeline. Update status on completion. **Done =** PRs published to remote.
* \[ ] \*\*1.15.6.5`\[P]` PR review simulation — Simulate code review before publishing. Review agent checks: code quality, tests, docs, security, performance. Generate review report. Address issues before publishing. **Done =** pre-publish review works.
* \[ ] \*\*1.15.6.6`\[P]` PR rollback — Revert all changes from a workspace. Restore base branch state. Clean workspace. **Done =** rollback works.
* \[ ] \*\*1.15.6.7`\[P]` PR history — Track all PRs generated by agents. Show status: draft, published, merged, rejected. Link to remote PR. Timeline of changes. **Done =** PR history tracked.
* \[ ] \*\*1.15.6.8`\[P]` PR description templates — Templates for different change types: feature, bugfix, refactor, test, docs. Auto-select based on objective. Customizable. **Done =** templates auto-applied.
* \[ ] \*\*1.15.6.9`\[P]` PR review request — Request human review from team members. Mention reviewers. Set labels. Set milestones. **Done =** review requests sent.
* \[ ] \*\*1.15.6.10`\[P]` PR metrics — Track: time to completion, cost, lines changed, files changed, tests added, coverage change. Compare to baseline. **Done =** PR metrics tracked.


## Phase 2 — Collaboration, Ecosystem \& Mobile

### 2.1 Real-Time Collaboration (Yjs CRDT)

> \*\*Goal:\*\* Multiple users (and agents) can edit the same file simultaneously, with no conflicts. Yjs CRDT is wired into CodeMirror 6 via `@codemirror/collab`, synced over the sub-service WebSocket. Offline edits are queued and sync on reconnect. Every collaborator has a colored cursor with their name, and agents show up as named presences too. The system feels like Google Docs for code — live, seamless, and conflict-free.
> \*\*Audit Note:\*\* No collaboration code exists. Yjs is not in package.json. The sub-service doesn't exist yet. This is a full build from scratch.

* \[ ] **2.1.1** Yjs integration — `@codemirror/collab` into CodeMirror 6. Yjs provider over sub-service WebSocket. Offline edits queued, sync on reconnect. **Done =** two users can edit same file simultaneously.
* \[ ] **2.1.2** Presence indicators — Colored cursors per collaborator. Avatar + name hover. Cursor labels. **Done =** presence visible.
* \[ ] **2.1.3** Collaboration permissions — Roles: viewer, commenter, editor, admin. Per-file overrides. **Done =** permissions enforced.
* \[ ] **2.1.4** Agent as collaborator — Agent edits as named presence. Per-hunk accept/reject. Agent activity indicator. **Done =** agent visible as collaborator.

### 2.2 Extensions

> \*\*Goal:\*\* Runyard is extensible via the Open VSX Registry — users can browse, install, update, and manage extensions just like VS Code. Extensions run in an isolated Node.js process with a VS Code API compatibility shim, so popular extensions work with minimal or no changes. A permission model ensures extensions can't access the filesystem or network without explicit user approval. Worker thread sandboxing keeps the main process safe even if an extension misbehaves.
> \*\*Audit Note:\*\* No extension system exists. No Node.js extension host. No Open VSX integration. This is a full build from scratch.

* \[ ] **2.2.1** Open VSX integration — Browse, install, update, uninstall, disable from Open VSX Registry. **Done =** extensions installable from registry.
* \[ ] **2.2.2** Extension host — Isolated Node.js process. VS Code API compatibility shim. Tabs, commands, language services, themes. **Done =** extensions run isolated.
* \[ ] **2.2.3** Extension security — Permission model (FS, network, process). User approval on install. Worker thread sandboxing. **Done =** extensions are sandboxed.

### 2.3 Orchestrator Agent

> \*\*Goal:\*\* The orchestrator is a first-party agent that thinks across projects. It delegates to subagents via A2A, manages the workspace through the sub-service, and maintains project memory in SQLite. The UI shows a global dashboard with project cards, active agents, issue counts, and cost totals. On mobile, the orchestrator is the primary interface — users interact with natural language, and the system simplifies everything to cards and status indicators. The orchestrator is the intelligence layer that connects all the pieces.
> \*\*Audit Note:\*\* No orchestrator exists. No A2A client. No agent system at all. This is a full build from scratch.

* \[ ] **2.3.1** Orchestrator core — First-party agent with cross-project access. A2A for subagent delegation. Sub-service for workspace access. SQLite project memory. **Done =** orchestrator runs and delegates.
* \[ ] **2.3.2** Orchestrator UI tab — Global tab. Project cards: status, agents, issues, activity, cost. Global task queue. Agent broadcasting. **Done =** orchestrator UI works.
* \[ ] **2.3.3** Orchestrator mobile UX — Primary mobile interface. Natural language commands. Simplified cards. Accessible via sub-service web frontend. **Done =** orchestrator works on mobile.

### 2.4 Native Mobile Apps (Tauri Mobile)

> \*\*Goal:\*\* Full native iOS and Android apps built on Tauri Mobile. Bottom tab bar for navigation, push notifications for agent completions and HIL requests, background refresh for status updates. The apps share the same Svelte 5 codebase as the desktop, so all features are present — just adapted for mobile form factors. These are companion apps, not cut-down versions.
> \*\*Audit Note:\*\* `apps/mobile` exists but is empty (README only). No Tauri Mobile configuration. No mobile-specific code.

* \[ ] **2.4.1** `\[CROSS]` iOS companion — Tauri Mobile shell. Bottom tab bar. APNs push notifications. Background refresh. **Done =** iOS app runs.
* \[ ] **2.4.2** `\[CROSS]` Android companion — Tauri Mobile shell. Bottom tab bar. FCM push notifications. Adaptive layouts. **Done =** Android app runs.

### 2.5 Cost \& Analytics

> \*\*Goal:\*\* Users always know what they're spending. Per-conversation, per-project, and per-day cost dashboards show actual spend vs budget, with model-by-model breakdowns. CSV export for accounting. Usage analytics reveal task volume, popular models and tools, and response times — all local, all opt-in. No surprise bills, no hidden costs.
> \*\*Audit Note:\*\* No cost tracking exists. No analytics. No dashboards. This is a full build from scratch.

* \[ ] **2.5.1** Cost dashboards — Per conversation/project/day/week/month. Model comparison. Budget vs actual. CSV export. **Done =** dashboards show spend.
* \[ ] **2.5.2** Usage analytics — Task volume, popular models/tools, response times. Local, opt-in telemetry. **Done =** analytics visible.

### 2.6 Additional Integrations

> \*\*Goal:\*\* Issue trackers beyond GitHub and Linear — Jira and GitLab are first-class integrations. Agents can browse, filter, and act on issues from both platforms. Combined with the existing GitHub and Linear support, Runyard covers the majority of development workflows. The SSH bootstrap feature is also fully operational, enabling one-click sub-service setup on any remote machine.
> \*\*Audit Note:\*\* No Jira, GitLab, or SSH integration exists. No issue tracker integrations beyond the git operations in the core crate.

* \[ ] **2.6.1** Jira issue integration — Browse, filter, pass to agent via A2A, auto-update status. **Done =** Jira accessible.
* \[ ] **2.6.2** GitLab issue integration — Browse, filter, pass to agent, auto-update status. **Done =** GitLab accessible.
* \[ ] **2.6.3** SSH bootstrap — Auto-install sub-service over SSH. No pre-install. **Done =** SSH bootstrap works.


### 2.7 Built-in Runyard Agent (ACP Server + MCP Client + A2A Client)

> **Goal:** Runyard ships with its own built-in agent that is an ACP server. The IDE connects to this agent via ACP (just like any external agent). The built-in agent handles ALL provider-specific logic: LLM API calls (Anthropic, OpenAI, Google, OpenRouter, local models), streaming, token counting, tool schemas, context assembly, and cost tracking. It also connects to MCP servers as an MCP client, and to other agents as an A2A client (for orchestration). The built-in agent is optional — users can always use external ACP agents instead. This gives Runyard a "batteries included" experience while maintaining the ACP architecture.

> **Why Phase 2, not Phase 1:** Phase 1 focuses on making Runyard a world-class ACP client that works with any external agent. Phase 2 adds the built-in agent for users who don't have an external agent installed or prefer an integrated experience. The built-in agent is a full ACP server implementation — it speaks ACP to the IDE, and uses SDKs to speak to LLM providers. It does not change the IDE architecture at all.

> **Architecture:** The built-in agent is a Rust crate (`crates/runyard-agent`) that runs as a subprocess (stdio) or as a background thread within the IDE. It exposes ACP on one side (the IDE connects to it) and uses official SDKs on the other side: `anthropic` crate for Claude, `async-openai` for OpenAI, `google-generative-ai` for Gemini, `openrouter` for OpenRouter, `ollama` for local models. It also uses `rmcp` for MCP client functionality and `a2a-lf` for A2A client functionality.

> **Provider SDKs to research:** models.dev (GitHub repo) for the unified provider abstraction. The built-in agent should support custom base URL, API key, and model for any provider.

- [ ] **2.7.1** `[P]` Built-in agent crate scaffold — `crates/runyard-agent/`: binary crate with `main.rs`. Struct `RunyardAgent` implementing ACP server. Use `agent-client-protocol` crate for ACP server role. **Done =** crate compiles, agent struct exists.
- [ ] **2.7.2`[P]` ACP server implementation — Implement ALL ACP server methods: `initialize`, `authenticate`, `session/new`, `session/load`, `session/resume`, `session/prompt`, `session/cancel`, `session/close`, `session/list`, `session/set_mode`, `session/set_config_option`, `logout`. Emit ALL `session/update` notification types. Support stdio transport (primary). **Done =** ACP server complete, IDE can connect.
- [ ] **2.7.3`[P]` Provider SDK integration — Research `models.dev` (GitHub repo) for unified provider abstraction. Integrate: `anthropic` crate (Claude), `async-openai` (OpenAI), `google-generative-ai` (Gemini), `openrouter` (200+ models), `ollama` (local). Each provider: `send_message`, `stream_message`, `count_tokens`, `list_models`. **Done =** all providers work.
- [ ] **2.7.4`[P]` Provider configuration — Per-provider config: API key (keychain), base URL override, default model, rate limits, timeout. Global config file `~/.runyard/agent-config.toml`. UI in settings panel. Test connection button. **Done =** providers configurable.
- [ ] **2.7.5`[P]` Streaming response handling — Receive SSE streams from providers. Parse chunks. Emit `AgentMessageChunk` via ACP `session/update`. Handle tool calls in stream. Handle stop reasons. **Done =** streaming works to IDE.
- [ ] **2.7.6`[P]` Tool call translation — Provider tool schemas → ACP tool call format. Handle tool calls from provider. Execute via MCP (if MCP server available) or request permission via ACP `request_permission`. Return results to provider. **Done =** tool calls flow end-to-end.
- [ ] **2.7.7`[P]` Context assembly — Assemble context from: system prompt, skills (progressive disclosure), pinned context, editor state (open files, tabs), recent files, conversation history (up to 80%), reserve 20% for response. Token counting per provider. Budget management. **Done =** context assembled correctly.
- [ ] **2.7.8`[P]` Context compression — When budget exceeded, identify oldest messages. Summarize with fast/cheap model. Replace with summary. Preserve originals. Agent decides when to compress. **Done =** compression works.
- [ ] **2.7.9`[P]` Cost tracking — Per-message token count and cost. Per-conversation total. Per-provider spend. Soft/hard caps. Report costs via ACP `session/update`. Store in SQLite. **Done =** costs tracked and reported.
- [ ] **2.7.10`[P]` MCP client integration — Use `rmcp` crate. Connect to MCP servers configured by user via IDE. Discover tools. Call tools. Handle resources and prompts. Report tool calls to IDE via ACP. **Done =** MCP client works.
- [ ] **2.7.11`[P]` A2A client integration — Use `a2a-lf` crate. For orchestration: discover other agents via Agent Cards. Delegate tasks. Handle task lifecycle. Return results to IDE via ACP. **Done =** A2A client works.
- [ ] **2.7.12`[P]` Skill loading and execution — Scan skill directories. Parse `SKILL.md` frontmatter. Progressive disclosure: metadata always, body on trigger, references on demand. Inject skills into context. Handle shell command injection (`!` prefix). Variable substitution (`$ARGUMENTS`, `$0`, `$1`, etc.). **Done =** skills loaded and executed.
- [ ] **2.7.13`[P]` Built-in agent UI toggle — Settings panel: "Use built-in agent" toggle. When enabled, IDE auto-connects to built-in agent on startup. When disabled, IDE uses last selected external agent. Seamless switch between built-in and external. **Done =** toggle works.
- [ ] **2.7.14`[P]` Built-in agent auto-start — When "Use built-in agent" is enabled, auto-start built-in agent on IDE launch. Spawn as subprocess (stdio). Monitor health. Restart on crash. Kill on IDE exit. **Done =** agent auto-starts.
- [ ] **2.7.15`[P]` Built-in agent error handling — Handle provider errors (rate limits, auth failures, network issues). Retry with backoff. Fallback to alternative provider (if configured). Report errors to IDE via ACP. **Done =** errors handled gracefully.
- [ ] **2.7.16`[P]` Built-in agent offline mode — When no network, use local models (Ollama/llama.cpp). Graceful degradation: reduced capabilities, no web tools, local-only skills. **Done =** offline mode works.
- [ ] **2.7.17`[P]` Built-in agent model selection — Per-conversation model selection. Dropdown: Claude 4, GPT-5, Gemini 2.5, etc. Model-specific features (extended thinking, 1M context). Cost preview. **Done =** model selection works.
- [ ] **2.7.18`[P]` Built-in agent reasoning modes — Support reasoning modes: `fast` (cheap, quick), `balanced` (default), `deep` (extended thinking, expensive). Per-message override. **Done =** reasoning modes work.
- [ ] **2.7.19`[P]` Built-in agent memory — Persistent memory across conversations. SQLite storage. Vector DB (`sqlite-vec` or `usearch`) for semantic search. Self-reflection: agent learns from its own work. **Done =** memory works.
- [ ] **2.7.20`[P]` Built-in agent test suite — Unit tests for each provider. Integration tests with mock providers. End-to-end tests with real providers (CI only). ACP conformance tests. **Done =** comprehensive test coverage.

## Phase 3 — Polish \& Platform

### 3.1 Theme Engine

> \*\*Goal:\*\* Runyard is visually yours. A full theme engine lets users create custom themes from a JSON format that maps to CSS custom properties, with a live theme editor and preview. Import and export themes. Browse a gallery of community themes. VS Code themes import directly — token scopes are mapped to CodeMirror styles automatically. The editor looks exactly the way you want it to.
> \*\*Audit Note:\*\* A basic light/dark toggle exists via `themeStore.svelte.ts` (stores in localStorage, applies `data-theme` attribute). The `AppearancePanel` component has Light/Dark buttons. This is a binary toggle, not a theme engine. No custom themes, no CSS custom properties for tokens, no import/export, no gallery. CodeMirror uses hardcoded `oneDark` theme.

* \[\~] **3.1.1** Full theme engine — User-created themes. JSON format → CSS custom properties. Theme editor with live preview. Import/export. Gallery. **Done =** users can create and share themes.
* \[ ] **3.1.2** VS Code theme import — `.vsix` from Open VSX. Map token scopes to CodeMirror styles. **Done =** VS Code themes importable.

### 3.2 Community \& Marketplace

> \*\*Goal:\*\* A community around Runyard. Skills and agent configurations are shared, rated, and reviewed in a marketplace. Verified publishers. Users can browse, install, and publish with a single click. The marketplace is where the ecosystem grows beyond what any single team can build.
> \*\*Audit Note:\*\* No marketplace exists. No server infrastructure for community sharing. No publishing flow.

* \[ ] **3.2.1** Skills marketplace — Browse, install, publish SKILL files. Ratings, reviews. Verified publishers. **Done =** skills marketplace works.
* \[ ] **3.2.2** Agent config marketplace — Community agent configs. Pre-built workflows. **Done =** agent configs sharable.

### 3.3 Advanced AI

> \*\*Goal:\*\* Local intelligence that knows your codebase. A vector search index (`usearch` or `sqlite-vec`) is built automatically when a project is opened, enabling semantic search from chat — "find where we handle authentication" — with relevant files and symbols injected into context automatically. Advanced context strategies go beyond simple budgeting: relevance-based selection, automatic file/symbol inclusion based on the current task, and budget allocation across parallel conversations. Agents remember everything — persistent memory across conversations, backed by a vector database, with self-reflection so they learn from their own work.
> \*\*Audit Note:\*\* No vector search exists. No RAG. No semantic indexing. No agent memory. No SQLite even for basic persistence. This is a full build from scratch.

* \[ ] **3.3.1** RAG over codebase — Local vector search (`usearch` or `sqlite-vec`). Index on open. Semantic search from chat. Auto-context retrieval. **Done =** semantic search works.
* \[ ] **3.3.2** Advanced context strategies — Relevance-based selection. Auto file/symbol inclusion. Budget allocation across conversations. **Done =** context smarter.
* \[ ] **3.3.3** Agent memory — Persistent across conversations. Vector DB-backed. Self-reflection. **Done =** agents remember.

### 3.4 Performance Hardening

> \*\*Goal:\*\* Every millisecond and every byte matters. IPC serialization is evaluated against MessagePack for potential gains. Svelte 5 reactivity is profiled and tuned — no unnecessary renders. Binary sizes are minimized: `cargo-bloat` analysis, debug symbol stripping, LTO. The desktop app is under 30MB, the sub-service under 10MB. Fast, lean, and efficient everywhere.
> \*\*Audit Note:\*\* No performance profiling has been done. No MessagePack evaluation. No cargo-bloat. Binary sizes unknown. The app builds and runs but no optimization work has been done.

* \[ ] **3.4.1** `\[PERF]` IPC upgrade — Evaluate MessagePack vs JSON. Profile serialization overhead. **Done =** decision made, implemented if beneficial.
* \[ ] **3.4.2** `\[PERF]` Rendering optimization — Profile Svelte 5 reactivity in large layouts. Fix unnecessary re-renders. **Done =** no wasted renders.
* \[ ] **3.4.3** `\[PERF]` Binary size — `cargo-bloat`. Strip debug symbols. LTO. Target <30MB app, <10MB sub-service. **Done =** binaries within targets.

### 3.5 Plugin SDK

> \*\*Goal:\*\* Third-party developers can build plugins for Runyard. A documented public API with TypeScript types, a scaffolding CLI to generate boilerplate, example plugins, and a publishing flow to the marketplace. The SDK makes Runyard a platform, not just a tool.
> \*\*Audit Note:\*\* No plugin SDK exists. No public API documentation. No scaffolding CLI. No example plugins.

* \[ ] **3.5.1** Public SDK — Documented API. Scaffolding CLI. TypeScript types. Example plugins. Publishing to marketplace. **Done =** third-party plugins possible.

### 3.6 Cross-Platform Polish

> \*\*Goal:\*\* Runyard ships everywhere. Windows gets a full test pass and an MSI installer. Linux gets AppImage, deb, and rpm packages. macOS is signed and notarized with a DMG. Every platform feels like a native app, not a web view in a box.
> \*\*Audit Note:\*\* Tauri is configured for all platforms (icons exist for Windows/macOS/Linux). No CI/CD for building. No signing or notarization. No installer generation. Tauri has the `bundle` config but it's not customized.

* \[ ] **3.6.1** `\[CROSS]` Windows build — Full test pass. MSI installer. **Done =** Windows app ships.
* \[ ] **3.6.2** `\[CROSS]` Linux build — Full test pass. AppImage + deb + rpm. **Done =** Linux app ships.
* \[ ] **3.6.3** `\[CROSS]` macOS signing — Apple cert. Notarization. DMG. **Done =** macOS app signed and notarized.

## Quick Wins

Small, high-impact tasks that can be done in any order. All are cross-platform by default.

> \*\*Audit Summary:\*\* Of the \~100 tasks across all phases, approximately \*\*16 are fully complete\*\*, \*\*8 are partially built\*\*, and \*\*75+ have not been started\*\*. The most solid areas are: the Rust core (FS, Git, Terminal, LSP, Settings all have production-ready implementations), the sub-service daemon (fully implemented with WebSocket proxies for all services and static asset serving), the Svelte 5 layout system (tabs, splits, stores, command palette), and the basic CodeMirror 6 editor with LSP integration. The biggest gaps are: the entire chat/AI system (no types, no DB, no UI), all agent protocols (ACP/MCP/A2A are stubs), mobile/web UI adaptations, and SQLite persistence.

* \[\~] **QW.1** `\[CROSS]` Window/tab title shows current project + open file (e.g., "main.rs — Runyard") — Currently hardcoded as "Runyard" in +layout.svelte. Needs dynamic update based on active file.
* \[ ] **QW.2** `\[CROSS]` `Ctrl+Tab` switches between recent tabs (last-used order) — Not implemented.
* \[ ] **QW.3** Drag-and-drop files from OS file manager into editor to open — Not implemented.
* \[ ] **QW.4** Right-click context menu on editor tabs (close, close others, close to right, reveal in explorer) — Not implemented. Tabs only have close button (X).
* \[ ] **QW.5** Right-click context menu on file tree (new file, new folder, rename, delete, copy path) — Not implemented. TreeNode only has click-to-open.
* \[\~] **QW.6** `\[CROSS]` Split editor shortcut (`Ctrl+\\` / `Cmd+\\`) — `view.splitHorizontal` and `view.splitVertical` commands exist in palette but not bound to `Ctrl+\\`. Split resizing works with pointer drag.
* \[ ] **QW.7** Line numbers relative to cursor (Vim-style `relativenumber`) — Not implemented. `basicSetup` has absolute line numbers.
* \[ ] **QW.8** Auto-detect indentation (tabs vs spaces, size) from file content — Not implemented. Tab size is fixed from settings (default 2).
* \[ ] **QW.9** File icon theme (matching `vscode-icons` or `material-icon-theme` conventions) — Not implemented. Only generic File/Folder icons from lucide-svelte.
* \[\~] **QW.10** `\[CROSS]` Quick file open (`Ctrl+P` / `Cmd+P`) — Command palette shows recent files but no dedicated quick-open with fuzzy file search across workspace. `file.open` command just focuses the explorer.
* \[✓] **QW.11** `\[CROSS]` Status bar shows current Git branch + ahead/behind count — **DONE.** Status bar fetches branch via `git\_branch` and Git panel shows ahead/behind counts.
* \[ ] **QW.12** `\[CROSS]` "Reveal in Finder/Explorer" on file tree right-click — Not implemented.
* \[ ] **QW.13** `\[CROSS]` Duplicate line (`Cmd/Ctrl+Shift+D`) — Not implemented.
* \[\~] **QW.14** `\[CROSS]` Toggle word wrap (`Alt+Z`) — Settings has `line\_wrap` toggle but it's not wired to CodeMirror's line wrapping extension. Just a checkbox in settings.
* \[ ] **QW.15** `\[CROSS]` Go to line (`Cmd/Ctrl+G`) — Not implemented.
* \[ ] **QW.16** `\[CROSS]` Zoom in/out editor font (`Cmd/Ctrl+=`, `Cmd/Ctrl+-`) — Not implemented. Font size is settings-only.
* \[ ] **QW.17** Show whitespace characters toggle (spaces as mid-dots, tabs as arrows) — Not implemented.
* \[ ] **QW.18** `\[CROSS]` Command to collapse all / expand all folders in explorer — Not implemented.
* \[ ] **QW.19** `\[CROSS]` Recent workspaces list in welcome panel — Not implemented. Welcome panel is a static hero with two shortcuts.
* \[ ] **QW.20** `\[CROSS]` Close editor tab on middle-click — Not implemented.

## Dependency Graph

```
Phase 1 Core Dependencies:
  1.1.1 WebSocket server → 1.1.2-1.1.5 (all proxies), 1.1.7 (web serving), 1.2.1-1.2.4 (remote UI)
  1.1.7 Web serving → 1.2.4 (mobile browser)
  1.3.1 Chat types → 1.3.2 (SQLite schema), 1.3.4 (ChatStore), 1.4.x (renderers), 1.5.x (input/context)
  1.3.2 SQLite schema → 1.3.3 (Rust DB layer)
  1.3.3 Rust DB layer → 1.3.4 (ChatStore loads from DB)
  1.3.4 ChatStore → 1.4.1 (ChatPanel UI), 1.5.1 (input), 1.5.5 (context viz)
  1.6.1 ACP agent registry → 1.6.2 (discovery), 1.6.3 (config UI), 1.6.4 (launcher), 1.6.5 (switcher)
  1.7.1 ACP client crate → 1.7.2-1.7.4 (transports), 1.7.5-1.7.7 (lifecycle), 1.7.8-1.7.10 (prompt/cancel/update), 1.7.11 (tool calls), 1.7.17 (event bridge)
  1.7.5 ACP initialize → 1.7.6 (auth), 1.7.7 (session), 1.7.12 (capabilities)
  1.7.7 ACP session lifecycle → 1.7.8 (prompt), 1.7.9 (cancel), 1.7.10 (notifications)
  1.7.10 ACP session/update → 1.4.x (all chat renderers), 1.5.1 (input), 1.6.6 (status monitoring)
  1.7.11 ACP tool calls → 1.4.6 (tool call renderer), 1.4.7 (permission blocks), 1.8.4 (MCP tool display)
  1.8.1 MCP config model → 1.8.2 (MCP UI), 1.8.3 (config pass-through), 1.8.4-1.8.6 (tool display/permissions/health)
  1.9.1 Skills data model → 1.9.2 (CRUD UI), 1.9.3 (chat integration), 1.9.4 (directory scanning), 1.9.5 (validation)
  1.10.1 Agent task model → 1.10.2 (kanban UI), 1.10.3 (parallel management), 1.10.4 (cost tracking), 1.10.5 (HIL queue)
  1.6.2 ACP discovery + 1.7.1 ACP client → 1.4.1 (ChatPanel with agent header), 1.5.1 (input with agent context)
  1.3.4 ChatStore + 1.7.17 ACP event bridge → 1.4.x (all chat rendering), 1.5.6 (context assembly), 1.5.7 (compression)

Cross-Phase Dependencies:
  Phase 1 everything → Phase 2 everything (Phase 1 is MVP)
  1.1.1 Sub-service → 2.1.1 (Yjs needs WebSocket), 1.14.8 (multi-user needs server)
  1.7.1 ACP client → 2.7.1 (built-in agent needs ACP server), 2.3.1 (Orchestrator needs ACP to connect to built-in agent)
  1.10.1 Agent task model → 2.3.2 (Orchestrator UI needs task model)
  1.2.4 Mobile browser → 2.4.1/2.4.2 (native apps build on mobile web)
  1.7.1 ACP client + 1.8.1 MCP config → 2.7.10 (built-in agent MCP client needs IDE config)
  1.9.1 Skills model → 2.7.12 (built-in agent skill loading)
  1.6.2 Agent discovery → 2.7.13 (built-in agent toggle in agent panel)

Protocol Stack (ACP-First Architecture):
  ┌─────────────────────────────────────────────────────────────┐
  │  RUNYARD IDE (Phase 1) — ACP Client Only                    │
  │  ┌─────────────┐    ACP (stdio/HTTP/WS)    ┌──────────────┐ │
  │  │  Editor     │◄─────────────────────────►│  Agent       │ │
  │  │  (Client)   │                           │  (Server)    │ │
  │  └─────────────┘                           └──────────────┘ │
  │                                                   │         │
  │  ┌──────────────────────────────────────────────┘         │
  │  │  MCP Config UI (1.8) — IDE stores config, passes         │
  │  │  to agent via ACP session/new                            │
  │  │  Skills UI (1.9) — IDE manages files, agent loads        │
  │  │  Agent Manager (1.10) — IDE monitors ACP sessions        │
  │  └─────────────────────────────────────────────────────────┘
  │                                                             │
  │  Phase 2: Built-in Agent (ACP Server + Provider SDKs)       │
  │  ┌──────────────────────────────────────────────────────┐   │
  │  │  Built-in Agent (2.7)                                  │   │
  │  │  ├─ ACP Server (speaks to IDE)                        │   │
  │  │  ├─ Provider SDKs (Anthropic, OpenAI, Google, etc.)    │   │
  │  │  ├─ MCP Client (rmcp) — connects to tools              │   │
  │  │  ├─ A2A Client (a2a-lf) — delegates to other agents    │   │
  │  │  ├─ Context Assembly & Token Management              │   │
  │  │  └─ Cost Tracking & Budget Enforcement                 │   │
  │  └──────────────────────────────────────────────────────┘   │
  │                                                             │
  │  External Agents (also ACP Servers):                        │
  │  Claude Code, Gemini CLI, Codex CLI, Goose, Roo, etc.     │
  └─────────────────────────────────────────────────────────────┘

Key Principles:
  1. IDE NEVER speaks directly to LLM providers. Only ACP.
  2. IDE NEVER speaks MCP directly. Only stores MCP config for agent.
  3. IDE NEVER speaks A2A directly. Only the built-in agent does.
  4. ACP is the ONLY protocol the IDE implements fully.
  5. Built-in agent is Phase 2, not Phase 1. Phase 1 uses external agents.
  6. Any ACP-compatible agent works with Runyard. No vendor lock-in.
```


## Changelog

### v3 (2025-07-05) — Audit-Based Reality Check

* **Full codebase audit** against the GitHub repo (commit `06567ab`). Read every Rust file, every TS file, every package.json, every Cargo.toml.
* **Completion markers added** — `\[✓]` = fully done, `\[\~]` = partially built / stub exists, `\[ ]` = not started
* **Audit notes added** to every task — describes what actually exists in the codebase vs. what the spec says
* **Key findings:**

  * **Solid (\~16 complete, \~8 partial):** Rust core (FS, Git, Terminal, LSP, Settings), Svelte 5 layout system (tabs, splits, stores, command palette), CodeMirror 6 editor with LSP integration, sub-service daemon (fully implemented with WebSocket proxies and asset serving), client-side mock-core/events for browser WebSocket connectivity, basic theme toggle, xterm.js terminal with instance caching, file watcher, basic error handling
  * **Missing (\~75+ not started):** Entire chat/AI system (no types, no DB, no UI), all agent protocols (ACP/MCP/A2A are stubs), mobile/web UI adaptations, SQLite persistence, multi-workspace, themes beyond binary toggle, no performance profiling, no build pipeline
* **Quick wins re-evaluated:** QW.11 (Git branch in status bar) marked complete. QW.1, QW.6, QW.10, QW.14 marked partial. Rest remain not started.
* **No new task numbers added** — the existing 80+ tasks already cover the full spec. The audit just reveals which ones are further along than expected.

### v2 (2025-07-05)

* **Restructured from 16 sections to 6** — removed redundant Feature Catalog and Codebase Audit documents, merged their content here
* **Added 6 missing task groups** from feature spec: sub-service multi-user, inline editor diff, agent config editing UI, theme format foundation, sub-service health, storage architecture
* **Granular tasks** — split large tasks into smaller handoff units (e.g., chat panel split into 1.3 data, 1.4 UI, 1.5 input/context)
* **Added `\[STRETCH]` tag** — for non-blocking nice-to-haves
* **Added 10 more quick wins** — from QW.11 to QW.20
* **Added Changelog section** — this doc tracks its own evolution
* **OS-agnostic language** — removed "Mac-first", added `\[CROSS]` tag consistently
* **Mobile in Phase 1** — sub-service web serving enables mobile browsers in Phase 1, native apps in Phase 2
* **Protocol research** — moved to separate Protocol Research Reference document (cmr6pcxyf1p0z07adv19hx61s)
* **Chat Panel Spec** — detailed specification in separate document (cmr7ikw1r1t7y06adcqyaotl8)
* **Added Goal sections** — descriptive paragraph after each major section (1.1, 1.2, etc.) describing the end-state vision

## Phase 1 — Agentic Workspaces (Hackathon Solution Statement)

