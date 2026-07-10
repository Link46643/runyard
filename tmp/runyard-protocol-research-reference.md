# Runyard — Protocol Research Reference

Deep-dive research on the four protocols Runyard must implement: ACP, A2A, MCP, and Anthropic Skills Protocol. Includes spec versions, SDK availability, transport options, auth patterns, and implementation guidance.

## ACP — Agent Client Protocol (editor↔agent)

### Official Resources

| Resource | URL |
|----------|-----|
| **Official Website** | https://agentclientprotocol.com/ |
| **Protocol Overview (v1)** | https://agentclientprotocol.com/protocol/v1/overview |
| **Schema Documentation (v1)** | https://agentclientprotocol.com/protocol/v1/schema |
| **v2 RFD** | https://agentclientprotocol.com/rfds/v2/overview |
| **Streamable HTTP & WebSocket Transport RFD** | https://agentclientprotocol.com/rfds/streamable-http-websocket-transport |
| **Main GitHub Repo** | https://github.com/agentclientprotocol/agent-client-protocol |
| **Zed Mirror** | https://github.com/zed-industries/agent-client-protocol |
| **JetBrains Repo** | https://github.com/jetbrains/agent-client-protocol |
| **ACP Registry** | https://github.com/agentclientprotocol/registry |
| **v1 JSON Schema** | https://github.com/agentclientprotocol/agent-client-protocol/blob/main/schema/v1/schema.json |
| **v2 JSON Schema** | https://github.com/agentclientprotocol/agent-client-protocol/blob/main/schema/v2/schema.json |
| **Zed ACP Docs** | https://zed.dev/acp and https://zed.dev/docs/ai/external-agents |
| **JetBrains ACP Docs** | https://www.jetbrains.com/acp/ and https://www.jetbrains.com/help/ai-assistant/acp.html |
| **Rust SDK Docs** | https://docs.rs/agent-client-protocol and https://agentclientprotocol.github.io/rust-sdk/ |
| **Updates Blog** | https://agentclientprotocol.com/updates |
| **Registry Announcement (Zed)** | https://zed.dev/blog/acp-registry |
| **Registry Announcement (JetBrains)** | https://blog.jetbrains.com/ai/2026/01/acp-agent-registry |

### Official SDKs

| Language | Package | Repository | Status |
|----------|---------|------------|--------|
| **Rust** | `agent-client-protocol` (runtime) + `agent-client-protocol-schema` + `agent-client-protocol-tokio` + `agent-client-protocol-rmcp` + `agent-client-protocol-conductor` | https://github.com/agentclientprotocol/rust-sdk | **Official, most complete** |
| **TypeScript** | `@agentclientprotocol/sdk` (npm) | https://github.com/agentclientprotocol/typescript-sdk | **Official** |
| **Python** | `agent-client-protocol` (PyPI) | https://github.com/agentclientprotocol/python-sdk | **Official** |
| **Kotlin** | `com.agentclientprotocol:acp` (Maven) | https://github.com/agentclientprotocol/kotlin-sdk | **Official** |
| **Java** | `com.agentclientprotocol` (Maven) | https://github.com/agentclientprotocol/java-sdk | **Official** |
| **Go** | `acp-go-sdk` (pkg.go.dev) | https://github.com/agentclientprotocol/go-sdk | **Official** |

### Community SDKs & Libraries

| Library | Language | URL | Description |
|---------|----------|-----|-------------|
| `acp-client` | TypeScript | https://github.com/goddard-ai/acp-client | Minimal typed client helpers; registry launch, stdio, Zod schemas |
| `acpx` | Rust | https://github.com/imumesh18/acpx | Thin Rust client for launching ACP agents via stdio; registry-aware |
| `use-acp` | React/TypeScript | https://github.com/marimo-team/use-acp | React hooks for connecting to ACP servers |
| `acp.el` | Emacs Lisp | Community repo | Emacs integration |
| `acp-web-client` | Python/TypeScript | https://github.com/shon-yuan/acp-web-client | WebSocket bridge + browser client |
| `eino-contrib/acp` | Go | https://github.com/eino-contrib/acp | CloudWeGo Hertz implementation with HTTP/WS/WebSocket |

### What ACP Is

ACP is a **JSON-RPC 2.0 protocol** that standardizes communication between an editor/IDE (the client) and an AI coding agent (the server). It is co-developed by Zed Industries and JetBrains, licensed under Apache 2.0. Think of it as "LSP for AI agents" — it decouples the IDE from any specific LLM provider, model, or agent implementation.

**The core insight:** The IDE should not know what Claude, GPT-4, Gemini, or Ollama are. The IDE only knows how to speak ACP. The agent handles all provider-specific logic: API keys, streaming formats, token counting, tool schemas, context assembly, and cost tracking. This is how Zed's assistant works — the IDE is an ACP client, the built-in agent is an ACP server.

### Transport Methods

| Transport | Status | Use Case | Details |
|-----------|--------|----------|---------|
| **stdio (JSON-RPC over NDJSON)** | Stable / Default | Local agents | Editor spawns agent as subprocess; stdin/stdout for newline-delimited JSON-RPC. Logs go to stderr. This is the primary transport for local agents. |
| **HTTP (Streamable HTTP)** | Draft RFD (April 2026) | Remote agents | POST for client→server, GET for SSE streams, DELETE to close. Requires HTTP/2. Based on MCP Streamable HTTP pattern. |
| **WebSocket** | Draft RFD (April 2026) | Remote agents, full-duplex | Upgrade via `GET /acp` with `Upgrade: websocket`. All messages are JSON-RPC text frames. Persistent bidirectional messaging. |
| **TCP** | Mentioned | Underlying | Some implementations use TCP as underlying transport. |

**HTTP/WebSocket Headers (Draft RFD):**
- `Acp-Connection-Id`: transport-level ID returned at `initialize`
- `Acp-Session-Id`: session-level ID returned at `session/new`
- `Acp-Protocol-Version`: protocol version

### Core Message Types (JSON-RPC 2.0)

ACP uses two categories of JSON-RPC messages:
1. **Methods**: Request-response pairs expecting a result or error
2. **Notifications**: One-way messages without a response

#### Client → Agent Methods

| Method | Purpose | Key Parameters | Response |
|--------|---------|---------------|----------|
| `initialize` | Establish connection, negotiate capabilities, protocol version | `protocolVersion`, `clientCapabilities`, `clientInfo` | `InitializeResponse` with `protocolVersion`, `agentCapabilities`, `authMethods` |
| `authenticate` | Auth handshake (if required by agent) | Varies by `authMethods` | Auth result |
| `session/new` | Create a new session | `cwd`, `mcpServers`, `configOptions` | `sessionId` |
| `session/load` | Resume an existing session (v1) | `sessionId` | Session state |
| `session/resume` | Resume without replay (v2; stabilized) | `sessionId`, `replayFrom` | Session state |
| `session/prompt` | Send user message/prompt | `sessionId`, `message` | `PromptResponse` with `stopReason` |
| `session/cancel` | Interrupt ongoing processing | `sessionId` | Acknowledgment |
| `session/close` | Close session (stabilized) | `sessionId` | Acknowledgment |
| `session/list` | List sessions (stabilized) | — | Session list |
| `session/set_mode` | Switch agent operating mode | `sessionId`, `mode` | Acknowledgment |
| `session/set_config_option` | Set session config (model, reasoning, etc.) | `sessionId`, `key`, `value` | Acknowledgment |
| `logout` | End authenticated state (stabilized) | — | Acknowledgment |

#### Agent → Client Methods (Client Capabilities Required)

| Method | Purpose | Required Capability |
|--------|---------|---------------------|
| `fs/read_text_file` | Read file contents | `fs:readTextFile` |
| `fs/write_text_file` | Write file contents | `fs:writeTextFile` |
| `terminal/create` | Create terminal | `terminal` |
| `terminal/output` | Get terminal output | `terminal` |
| `terminal/wait_for_exit` | Wait for terminal command to exit | `terminal` |
| `terminal/kill` | Kill terminal command | `terminal` |
| `terminal/release` | Release terminal | `terminal` |
| `request_permission` | Request user authorization for tool call | — |

#### Agent → Client Notifications (`session/update`)

The agent streams real-time progress via `session/update` notifications:

| Update Type | Description |
|-------------|-------------|
| `AgentMessageChunk` | Text/content chunk from the agent |
| `UserMessageChunk` | Echo of user message |
| `ThoughtChunk` | Reasoning/thinking content |
| `toolCall` | Pending tool call request |
| `toolCallUpdate` | Completed tool call with content/locations |
| `plan_update` | Agent's execution plan update |
| `available_commands_update` | Updated list of available commands |
| `session_info_update` | Metadata update (title, etc.) |

### Session Lifecycle

```
1. Connection     → Client launches agent (stdio) or connects (HTTP/WS)
2. Initialize     → `initialize` request → `InitializeResponse`
3. Authenticate   → `authenticate` (if required)
4. Session Setup  → `session/new` or `session/resume` → `sessionId`
5. Prompt Turn    → `session/prompt` → agent processes → streams `session/update` notifications → final `PromptResponse`
6. Cleanup        → `session/close` or connection termination
```

### Capabilities Negotiation

Exchanged during `initialize` via `clientCapabilities` and `agentCapabilities`. In v2, capabilities are unified to a single `capabilities` field with role-agnostic `info`.

**v1 Capabilities:**
- `fs`: `readTextFile`, `writeTextFile`
- `terminal`: `create`, `output`, `wait`, `kill`, `release`
- `sessionCapabilities`: `list`, `resume`, `close`
- `promptCapabilities`: `streaming`, `toolCalls`
- `authMethods`: `none`, `bearer`, `apiKey`

**Important:** Capabilities are objects (`{}` means supported), not booleans. Omission or `null` means unsupported.

### Tool Calling Model

- Agent requests tool execution via `request_permission` (client-side method)
- Client renders permission UI; user approves/denies
- Client executes the tool (fs, terminal) and returns result
- Agent receives result and continues processing
- This is **different from MCP** where the agent directly calls tools. In ACP, the IDE (client) is the gatekeeper for all filesystem and terminal access.

### Context Management

- **Sessions**: Named conversation contexts with `sessionId`. Persistent across reconnects.
- **Session List**: Clients can list/resume sessions (`session/list`, `session/resume`)
- **Session Config Options**: Agents expose model selectors, reasoning levels, modes via `session/set_config_option`
- **MCP Servers**: Passed via `mcpServers` parameter in `session/new`; enables agent to access MCP tools within the session
- **Working Directory**: Sessions tied to `cwd` (current working directory)

### Extensibility

- **Custom methods**: Prefix with `_` (e.g., `_myMethod`)
- **Custom data**: Via `_meta` fields
- **Custom capabilities**: Advertised during initialization

### ACP Registry

Launched January 2026. Central registry for discovering, installing, and configuring ACP agents.
- PR-based submission: add your agent → available in all ACP clients
- Zed and JetBrains IDEs have built-in registry browsers
- URL: https://agentclientprotocol.com/overview/agents

### Supported Agents (as of July 2026)

| Agent | ID | Status | Notes |
|-------|-----|--------|-------|
| Claude Code | `claude-acp` | Stable | Official Anthropic adapter |
| Codex CLI | `codex-acp` | Stable | OpenAI Codex CLI |
| Gemini CLI | `gemini` | Stable | Google's CLI agent |
| GitHub Copilot | `github-copilot` | Public Preview | |
| Goose | `goose` | Stable | Block's agent framework |
| Cline | `cline` | Stable | VS Code extension agent |
| OpenCode | `opencode` | Stable | OpenCode runtime |
| Qwen Code | `qwen-code` | Stable | Alibaba's agent |
| Kimi CLI | `kimi` | Stable | Moonshot's agent |
| Junie (JetBrains) | `junie` | Stable | JetBrains' built-in agent |
| Roo Code | `roo` | Stable | VS Code extension |
| Cursor | `cursor` | Stable | Cursor IDE agent |

### Supported Clients/Editors

Zed (reference implementation), JetBrains IDEs, VS Code (community), Neovim, Emacs, Obsidian, Jupyter/marimo, Unity, Chrome, mobile apps (iOS/Android), messaging bridges (Discord, Telegram, Slack, WeChat).

### Governance & Status

- **Lead Maintainers (BDFL):** Ben Brandt (Zed), Sergey Ignatov (JetBrains)
- **Core Maintainers:** Agus Zubiaga, Anna Zhdan, Niko Matsakis
- **Decision Process:** Bi-weekly meetings; RFD (Request for Dialog) process; Zulip for async
- **Status:** Project approved by Technical Committee, pending final Governing Board vote (as of May 2026)
- **Current Version:** v1.4.0 (July 2026)
- **Stars:** ~3,600+ | **Forks:** ~300 | **Contributors:** 120+ | **Releases:** 63+
- **License:** Apache 2.0

### Key for Runyard

**ACP is the ONLY protocol Runyard's IDE needs to speak to agents.** The IDE implements the ACP client. Every agent — Claude Code, Gemini CLI, Codex, Roo, Goose, custom team agents — is an ACP server that Runyard discovers, launches, and communicates with.

- Use the Rust SDK (`agent-client-protocol` crate) for the client implementation
- Use the `conductor` feature for proxy chaining (middleware: logging, permission interception, tool filtering, cost tracking)
- Use `agent-client-protocol-rmcp` for the ACP↔MCP bridge (enables ACP agents to call MCP tools)
- The canonical stack: Runyard IDE (ACP Client) → Agent (ACP Server + MCP Client) → MCP Tools
- ACP reuses MCP JSON shapes where possible — the protocols are designed to be interoperable

## A2A — Agent-to-Agent Protocol (agent↔agent)

### Official Resources

| Resource | URL |
|----------|-----|
| **Official Website** | https://a2a-protocol.org/ |
| **v1.0.0 Specification** | https://a2a-protocol.org/v1.0.0/specification/ |
| **Latest Spec (dev)** | https://a2a-protocol.org/latest/specification/ |
| **Main GitHub Repository** | https://github.com/a2aproject/A2A |
| **Spec Markdown (GitHub)** | https://github.com/a2aproject/A2A/blob/main/docs/specification.md |
| **v1.0 Announcement** | https://github.com/a2aproject/A2A/blob/main/docs/announcing-1.0.md |
| **Protocol Buffers Schema** | https://github.com/a2aproject/A2A/blob/main/specification/a2a.proto |
| **a2a-lf Crate (crates.io)** | https://crates.io/crates/a2a-lf |
| **a2a-lf Docs (docs.rs)** | https://docs.rs/a2a-lf/latest/a2a/ |
| **a2a-rs Workspace** | https://github.com/a2aproject/a2a-rs |

### Governance & History

- **Origin:** Developed by Google Cloud, announced April 9, 2025
- **Donation:** Transferred to the Linux Foundation on June 23, 2025 (Open Source Summit North America)
- **License:** Apache License 2.0
- **Technical Steering Committee (TSC):** AWS, Cisco, Google, IBM Research, Microsoft, Salesforce, SAP, and ServiceNow
- **IBM ACP Merge:** IBM's Agent Communication Protocol (ACP) merged into A2A in late 2025 to avoid fragmentation. IBM's REST-first philosophy influenced A2A's design.
- **Important Note:** There were briefly two protocols called "ACP" — IBM's ACP (Agent Communication Protocol, now merged into A2A) and Zed/JetBrains' ACP (Agent Client Protocol, which is the editor-to-agent protocol). When people say "ACP" today, they mean the Zed/JetBrains protocol unless context clearly indicates A2A.

### Three-Layer Architecture

The spec is organized into three layers:
1. **Layer 1 (Canonical Data Model):** Core data structures defined via Protocol Buffers
2. **Layer 2 (Abstract Operations):** Fundamental behaviors and agent capabilities (transport-agnostic)
3. **Layer 3 (Protocol Bindings):** Concrete mappings to HTTP/REST, gRPC, JSON-RPC 2.0, and SSE

### Core Concepts

#### Agent Card Discovery
The **Agent Card** is a self-describing JSON manifest containing identity, metadata, capabilities, skills supported, wire protocols, security schemes, and requirements.

**Discovery mechanisms:**
1. **Well-Known URI:** `https://{agent-domain}/.well-known/agent-card.json` (RFC 8615)
2. **Curated Registries:** Directory-based search by skills/tags/capabilities
3. **Direct Configuration:** Pre-configured endpoints

**Security:** Cards can be digitally signed using JWS (JSON Web Signature) with JCS (JSON Canonicalization Scheme, RFC 8785) to prevent tampering.

#### Task Lifecycle
A2A distinguishes between stateless `Message` exchanges (trivial/transactional turns) and stateful `Task` processes (complex, trackable execution).

**Task States (`TaskState` enum):**
| State | Description |
|-------|-------------|
| `TASK_STATE_SUBMITTED` | Initial ingestion |
| `TASK_STATE_WORKING` | Active processing |
| `TASK_STATE_INPUT_REQUIRED` | Paused, needs user input/data |
| `TASK_STATE_AUTH_REQUIRED` | Authentication needed |
| `TASK_STATE_COMPLETED` | Terminal — success |
| `TASK_STATE_FAILED` | Terminal — error |
| `TASK_STATE_CANCELED` | Terminal — canceled |
| `TASK_STATE_REJECTED` | Terminal — rejected |

**Important:** Once a task reaches a terminal state, it becomes **immutable**. Follow-up work must start a new task with the same `contextId`.

#### Message Types
- **Message:** Communication unit with `messageId`, optional `contextId`/`taskId`, sender `role` (USER or AGENT), and `parts`
- **Part:** Container for text, raw bytes, URL, or arbitrary JSON with MIME types
- **Artifact:** Structured task output with `artifactId` and one or more `parts`

#### Streaming & Asynchronous Operations
- **Transport:** SSE (Server-Sent Events) with `Content-Type: text/event-stream`
- **Entry points:** `SendStreamingMessage`, `SubscribeToTask` (for non-terminal tasks)
- **SSE payload types:**
  1. `Task` — current work state
  2. `TaskStatusUpdateEvent` — state transitions
  3. `TaskArtifactUpdateEvent` — real-time data delivery
- **Note:** In v1.0, the `final` boolean field was removed from `TaskStatusUpdateEvent`; completion is determined solely by terminal task states.

#### Artifact Handling & Chunking
For large data transfers, artifacts support chunked streaming:
- `append` (boolean): Append to previously cached artifact with same ID
- `last_chunk` (boolean): Signals completion of transmission

### SDKs & Implementations

| Language | Package | Status | Notes |
|----------|---------|--------|-------|
| **Rust** | `a2a-lf` (crates.io) + `a2a-rs` workspace | Active | `a2a-lf` v0.3 (core types), `a2a-rs` (full SDK with client, server, gRPC, CLI) |
| **Python** | `a2a-sdk` (v1.1.0) | Most mature | Official Python SDK |
| **TypeScript** | `@a2a-js/sdk` | v0.3 stable | 1.0 work in progress |
| **Java** | `org.a2aproject.sdk` (Maven) | 1.0.0.Final GA | 6 months, 7 pre-releases, 17 contributors |
| **Go** | `a2a-go` | 1.0 GA | Official SDK |
| **.NET** | `A2A` (NuGet) | Preview | Tracking 1.0 spec |

**Rust a2a-rs Workspace Crates:**
| Crate | Import Name | Purpose |
|-------|-------------|---------|
| `a2a-lf` | `a2a` | Core types, errors, events, JSON-RPC types, serde |
| `a2a-client-lf` | `a2a-client` | Async client with transport abstraction |
| `a2a-server-lf` | `a2a-server` | Async server framework (REST/JSON-RPC on Axum) |
| `a2a-pb` | `a2a-pb` | Protobuf schemas + conversion helpers |
| `a2a-grpc` | `a2a-grpc` | gRPC client/server (Tonic) |
| `a2a-slimrpc` | `a2a-slimrpc` | SLIMRPC bindings |
| `a2acli` | `a2a-cli` | Standalone CLI for inspecting agent cards, managing tasks |

**Supported bindings:** JSON-RPC 2.0 over HTTP, REST/HTTP+JSON, gRPC (Tonic), SLIMRPC, SSE for streaming.
**Enhancements beyond spec:** Gap-free stream resumption via W3C SSE `Last-Event-ID` headers, ConnectRPC as alternative transport.

### Security Model
- **Transport:** HTTPS/TLS mandatory for production
- **Authentication:** OAuth 2.0, API keys, mTLS, Bearer tokens (declared in Agent Card)
- **Access Control:** At skill level, data level, and action level
- **Anti-Enumeration:** Returns `404 Not Found` (not `403`) for unauthorized access attempts to prevent scanning
- **Version Negotiation:** `A2A-Version` header per interface; single agent can support multiple protocol versions simultaneously

### Integration Test Kit (ITK)
Cross-SDK interoperability validation framework:
- Quarkus-based reference agent running predefined scenarios
- Daily snapshot dashboard of known-good cross-SDK combinations
- Validates Java, Python, TypeScript, and other SDK implementations

### Adoption Metrics (July 2026)
- **Organizations:** 150+ (up from 50+ in April 2025)
- **GitHub Stars:** 22,000+ on core repo
- **Industries:** Supply chain, financial services, insurance, IT operations
- **Cloud integrations:** Microsoft Azure AI Foundry + Copilot Studio, AWS Amazon Bedrock AgentCore Runtime, Google native integration

### Key for Runyard
- A2A is used for **Orchestrator ↔ subagent task delegation** within Runyard's agent system
- Cross-agent result passing and parallel routing
- Signed Agent Cards (JWS) for identity verification
- The Orchestrator agent (Phase 2) will use A2A to discover and delegate to other agents
- Runyard does NOT need to implement A2A in the IDE itself — it's an agent-level protocol

## MCP — Model Context Protocol (agent↔tools)

### Official Resources

| Resource | URL |
|----------|-----|
| **Official Website** | https://modelcontextprotocol.io |
| **Intro Blog Post (Anthropic)** | https://www.anthropic.com/news/model-context-protocol (November 2024) |
| **Main Specification Repo** | https://github.com/modelcontextprotocol/modelcontextprotocol |
| **Stable Spec (2025-11-25)** | https://modelcontextprotocol.io/specification/2025-11-25/index |
| **Draft Spec (2026-07-28 RC)** | https://modelcontextprotocol.io/specification/draft/basic/versioning |
| **MCP Blog** | https://blog.modelcontextprotocol.io |
| **SDK Tiering Docs** | https://modelcontextprotocol.io/community/sdk-tiers.md |
| **SDK Documentation** | https://modelcontextprotocol.io/docs/sdk |
| **Python SDK v2 Beta** | https://py.sdk.modelcontextprotocol.io/v2/ |
| **TypeScript SDK v2 Beta** | https://ts.sdk.modelcontextprotocol.io/v2/ |
| **Python SDK Repo** | https://github.com/modelcontextprotocol/python-sdk |
| **TypeScript SDK Repo** | https://github.com/modelcontextprotocol/typescript-sdk |
| **Rust SDK Repo (rmcp)** | https://github.com/modelcontextprotocol/rust-sdk |
| **C# SDK Repo** | https://github.com/modelcontextprotocol/csharp-sdk |
| **Go SDK Repo** | https://github.com/modelcontextprotocol/go-sdk |
| **Java SDK Repo** | https://github.com/modelcontextprotocol/java-sdk |
| **Kotlin SDK Repo** | https://github.com/modelcontextprotocol/kotlin-sdk |
| **Reference Servers** | https://github.com/modelcontextprotocol/servers |
| **Registry** | https://github.com/modelcontextprotocol/registry |
| **v2 SDK Beta Blog Post** | https://blog.modelcontextprotocol.io/posts/sdk-betas-2026-07-28/ |

### What MCP Is

MCP is a protocol that standardizes how an **agent** connects to **tools, data sources, and external services**. It was originated by Anthropic, donated to the Linux Foundation Agentic AI Foundation (AAIF) in December 2025, and is now an open standard with 5,800+ servers and 97M+ monthly SDK downloads. Think of MCP as "USB-C for AI tools" — it provides a universal plug for agents to access filesystems, databases, APIs, browsers, and any other tool.

**The core insight:** The agent should not hardcode tool integrations. Instead, it connects to MCP servers that expose tools, resources, and prompts via a standardized JSON-RPC interface. The agent discovers what tools are available, calls them with schema-validated arguments, and receives structured results. New tools are added by starting new MCP servers, not by modifying the agent.

**In Runyard's architecture:** The IDE (ACP client) connects to the agent (ACP server). The agent (ACP server) connects to MCP servers (MCP client → MCP server). The IDE does not speak MCP directly — it only speaks ACP. The agent handles all MCP discovery, tool calling, and result processing.

### Protocol Versions

| Version | Status | URL | Key Features |
|---------|--------|-----|-------------|
| **2025-11-25** | Stable | https://modelcontextprotocol.io/specification/2025-11-25/index | Tasks, JSON Schema 2020-12, Streamable HTTP |
| **2026-07-28** | Release Candidate (RC) | https://github.com/modelcontextprotocol/modelcontextprotocol/releases/tag/2026-07-28-RC | Stateless design, v2 SDK alignment |
| **DRAFT** | In development | https://modelcontextprotocol.io/specification/draft/basic/versioning | Evolving spec |

### Three Roles

```
Host → Client → Server
```

- **Host:** The application (e.g., Runyard's agent) that manages multiple MCP clients
- **Client:** The MCP client within the host that connects to a specific MCP server
- **Server:** The external process that exposes tools, resources, and prompts

Multiple clients can connect to the same server. The host (agent) orchestrates which clients are active.

### Transport Options

| Transport | Description | Status | Notes |
|-----------|-------------|--------|-------|
| **stdio** | Client launches server as subprocess; newline-delimited JSON-RPC over stdin/stdout. Logs go to stderr. | **Standard, stable** | Primary local transport. No ports, no auth, no CORS. |
| **Streamable HTTP** | POST for client→server; server replies with `application/json` or initiates SSE stream. Supports `Last-Event-ID` for resumability. | **New standard remote transport** | Replaced deprecated HTTP+SSE. Replaces legacy SSE. |
| **SSE (legacy)** | Server-Sent Events over HTTP. | **Deprecated** | Still supported for backward compatibility. New servers must NOT use. |
| **WebSocket** | Full-duplex persistent connection. | **Not primary** | Not adopted as primary built-in transport. Custom implementations possible. |

### Server-Side Primitives

| Feature | Description | Discovery | Invocation | Notifications |
|---------|-------------|-----------|------------|--------------|
| **Tools** | Callable functions exposed to LLMs | `tools/list` (paginated) | `tools/call` | `notifications/tools/list_changed` |
| **Resources** | Static/dynamic data (files, schemas, logs) | `resources/list`, `resources/templates/list` | `resources/read` | `notifications/resources/updated`, `notifications/resources/list_changed` |
| **Prompts** | Reusable prompt templates/workflows | `prompts/list` | `prompts/get` (with args) | `notifications/prompts/list_changed` |

### Client-Side Primitives

| Feature | Description | Key Behavior |
|---------|-------------|-------------|
| **Sampling** | Server requests LLM completions from client | Server sends `sampling/createMessage`; client runs it through its LLM and returns result. Keeps client in control of LLM access, credentials, and security. |
| **Roots** | Client tells server which directories/projects it's working in | `file://` URIs. Servers query roots to scope their work. |
| **Elicitation** | Interactive user prompts forwarded from server to client UI | Forms, confirmations, multi-step inputs. |
| **Tasks** | Long-running tool invocations with progress tracking | Server reports progress, client can cancel. |

### Lifecycle & Handshake

```
1. Initialization Request → Client sends `initialize` with protocol version, capabilities, `clientInfo`
2. Initialization Response → Server replies with protocol version, capabilities, `serverInfo`, optional instructions
3. Initialized Notification → Client sends `notifications/initialized`
4. Normal Operations → Only after step 3 can regular requests flow
```

### Version Negotiation
- Client proposes its highest supported version
- Server replies with its highest compatible version
- If client cannot support server's version → disconnect
- Over HTTP: `MCP-Protocol-Version` header on subsequent requests

### Capability Negotiation
- **Client capabilities:** `roots`, `sampling`, `elicitation`, `tasks`
- **Server capabilities:** `prompts`, `resources`, `tools`, `logging`

### Error Handling
- Standard JSON-RPC 2.0 error codes: `PARSE_ERROR` (-32700), `INVALID_REQUEST` (-32600), `METHOD_NOT_FOUND` (-32601), `INVALID_PARAMS` (-32602), `INTERNAL_ERROR` (-32603)
- MCP-specific codes: `HeaderMismatch` (-32020), `MissingRequiredClientCapability` (-32021), `UnsupportedProtocolVersion` (-32022)
- Range `-32020` to `-32099`: exclusively managed by MCP spec
- Custom codes: outside `-32768` to `-32000`

### Authentication (OAuth 2.1)

**Critical design:** MCP server = Resource Server, NOT Authorization Server.

**Flow:**
1. Client requests → Server returns 401
2. Server provides Protected Resource Metadata (RFC 9728)
3. Client discovers Authorization Server Metadata (RFC 8414)
4. Client registers via DCR or URL-based registration
5. Auth Code + PKCE flow
6. Token with Resource Indicators (RFC 8707) — `aud` bound to target server

**Standard scopes:** `mcp:tools`, `mcp:prompts`, `mcp:resources`

### Rust SDKs

| Crate | Tier | Version | Features | Repository |
|-------|------|---------|----------|------------|
| **`rmcp`** | Official Tier 2 | v2.2.0 | `#[tool]` macros, tokio, stdio/HTTP, OAuth2, SSE, `tower-service` integration | https://github.com/modelcontextprotocol/rust-sdk |
| **`rust-mcp-sdk`** | Community | v0.9.0 | Stdio, Streamable HTTP, SSE, OAuth (Keycloak/WorkOS/Scalekit), DNS rebinding, telemetry, multi-client, batch messages, resumability | https://github.com/rust-mcp-stack/rust-mcp-sdk |
| **`mcp-protocol-sdk`** | Community | v0.5.1 | Stdio, HTTP, WebSocket, connection pooling, retry logic, ~45% faster than standard HTTP, built-in metrics & health checks | https://github.com/mcp-protocol-sdk/mcp-protocol-sdk |
| **`model-context-protocol`** | Community | v0.2.2 | Full JSON-RPC 2.0, `McpHub` for multi-server, `#[mcp_server]`/`#[mcp_tool]` macros, stdio & HTTP (Actix-Web), dynamic Tool Groups | https://github.com/tsharp/model-context-protocol |

### SDK Maturity Tiering (MCP Official)

MCP uses a 3-tier evaluation system based on conformance testing, maintenance guarantees, documentation quality, and feature completion.

**Tier 1 (Fully Supported):**
| SDK | Stars | Version | Notes |
|-----|-------|---------|-------|
| Python | ~23,500 | Stable 1.x; v2.0.0b1 (beta) | Reference implementation. FastMCP → `MCPServer` in v2. |
| TypeScript | ~12,800 | Stable 1.x; v2.0.0-beta.1 | Split into `@modelcontextprotocol/server` + `@modelcontextprotocol/client`. Multi-Round-Trip Requests (MRTR). Standard Schema support. |
| C# | ~4,000 | v1.4.0 stable; v2.0.0-preview.1 | Microsoft collaboration. Enterprise SSO (ID-JAG). |
| Go | Active | v1.7.0+ | Client-side OAuth. Supports 2026-07-28 spec. |
| Rust (`rmcp`) | Active | v2.2.0 | Official, tokio-based. |

**Tier 2 (Commitment to Full Support):**
| SDK | Stars | Version | Notes |
|-----|-------|---------|-------|
| Java | ~3,500 | v2.0.0 GA (June 2026) | Spring AI collaboration. Spring Boot starters. OAuth 2.0/API key. |

**Tier 3 (Experimental):**
| SDK | Stars | Version | Notes |
|-----|-------|---------|-------|
| Kotlin | ~1,300 | 0.13.x / 0.14.x | JetBrains collaboration. KMP (JVM, Native, JS, Wasm). Tasks, typed elicitation, DNS rebinding. |

### v2 SDK Beta Wave (June–July 2026)
All official SDKs released beta versions aligned with the 2026-07-28 spec RC:
- Python v2.0.0b1: https://github.com/modelcontextprotocol/python-sdk/releases/tag/v2.0.0b1
- TypeScript v2.0.0-beta.1: https://github.com/modelcontextprotocol/typescript-sdk/releases/tag/v2.0.0-beta.1
- C# v2.0.0-preview.1: https://github.com/modelcontextprotocol/csharp-sdk/releases
- Blog post: https://blog.modelcontextprotocol.io/posts/sdk-betas-2026-07-28/

### ACP ↔ MCP Bridge

The `agent-client-protocol-rmcp` crate provides the official Rust bridge between ACP and MCP:
- **Crate:** https://crates.io/crates/agent-client-protocol-rmcp (v0.13.1)
- **Docs:** https://docs.rs/agent-client-protocol-rmcp/latest/agent_client_protocol_rmcp/
- **Repo:** https://github.com/agentclientprotocol/rust-sdk
- **Description:** Integrates ACP clients/agents with the `rmcp` MCP SDK. Part of the ACP Rust SDK workspace.

**Other Bridges:**
| Bridge | Repo | Description |
|--------|------|-------------|
| `mcacp` | https://github.com/Oortonaut/mcacp | MCP server that acts as universal bridge. Connects MCP clients (Claude Code, Zed, VS Code) to ACP coding agents. 22 tools, 5 groups. Session persistence, permission policies. |
| `acp-gateway` | https://github.com/agentrq/acp-gateway | Node.js bridge. Connects ACP-compatible agents to MCP server. Streamable HTTP, auto-reconnect, per-task session isolation. |
| `AgentPool ToolManagerBridge` | https://github.com/phil65/agentpool | In-process MCP server bridge exposing internal toolsets to external ACP agents. |

### Key for Runyard
- MCP is an **agent-level protocol**, not an IDE-level protocol. The IDE (Runyard) does NOT speak MCP directly.
- The agent (ACP server that Runyard connects to) is the MCP client. It discovers and calls MCP servers.
- The IDE only sees MCP tool calls as ACP `session/update` notifications (agent reporting what tools it called) or ACP `toolCall` requests (agent requesting tool execution via the IDE's permission system).
- Runyard's **Agent Runner** (the built-in agent, Phase 2) will use `rmcp` or `rust-mcp-sdk` to connect to MCP servers.
- Runyard's **MCP Server Manager UI** (Phase 1) lets users configure and manage MCP servers that their agents will connect to. The UI stores config; the agent uses it at runtime.
- The canonical stack: Runyard IDE (ACP Client) → Runyard Agent (ACP Server + MCP Client) → MCP Servers (GitHub, PostgreSQL, Filesystem, Browser, etc.)

## Anthropic Skills Protocol (agent↔instructions)

### Official Resources

| Resource | URL |
|----------|-----|
| **Official Skills Repository** | https://github.com/anthropics/skills |
| **Agent Skills Specification** | https://agentskills.io/specification.md |
| **Client Implementation Guide** | https://agentskills.io/client-implementation/adding-skills-support |
| **Claude Code Skills Docs** | https://code.claude.com/docs/en/skills |
| **Claude.ai Skills Docs** | https://claude.com/docs/skills/how-to |
| **Claude Platform API Docs** | https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview |
| **Skills API Quickstart** | https://platform.claude.com/docs/en/agents-and-tools/agent-skills/quickstart |
| **Claude API Reference (Skills)** | https://platform.claude.com/docs/en/api/beta/skills |
| **Skills Cookbook** | https://platform.claude.com/cookbook/skills-notebooks-03-skills-custom-development |
| **Best Practices** | https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices |
| **Official Plugin Dev Skills** | https://github.com/anthropics/claude-plugins-official |
| **Engineering Blog** | https://anthropic.com/engineering/equipping-agents-for-the-real-world-with-agent-skills |
| **agentskills SDK (Python)** | https://github.com/phronetic-ai/agentskills |
| **skillkit (Python)** | https://github.com/maxvaega/skillkit |
| **claude-skills-cli (npm)** | https://registry.npmjs.org/claude-skills-cli |
| **claude-skills-toolkit** | https://github.com/lakarpusky/claude-skills-toolkit |
| **MCP Discussion (Skills replacing prompts)** | https://github.com/modelcontextprotocol/modelcontextprotocol/discussions/1779 |

### What the Skills Protocol Is

The Skills Protocol is an open standard (agentskills.io) for defining reusable, file-based instructions that make AI agents smarter. A skill is a directory containing a `SKILL.md` file with YAML frontmatter + markdown body, plus optional scripts, references, and assets. Skills are portable across clients — they work in Claude Code, Cursor (with MDC hybrid frontmatter), OpenAI Codex, Microsoft Agent Framework, and any client that implements the open standard.

**The core insight:** Instead of stuffing all agent knowledge into a single system prompt, skills are modular, discoverable, context-managed instruction modules. They load progressively: only the metadata is in context until the agent decides a skill is relevant, then the full body loads, and references/scripts load on demand. This saves 85-95% on token costs compared to loading everything upfront.

**Skills are agent-level, not IDE-level.** The IDE manages the skill files (create, edit, delete, organize), but the agent (ACP server) is what reads, triggers, and executes skills. The IDE passes skill metadata to the agent via ACP `session/new` or `session/set_config_option`; the agent handles skill loading and execution.

### Directory Structure & Format

```
skill-name/                    # Must match `name` in frontmatter
├── SKILL.md                   # REQUIRED: metadata + instructions
├── scripts/                   # Optional: executable code (Python, Bash, JS, etc.)
├── references/                # Optional: docs loaded on demand
│   ├── REFERENCE.md
│   ├── FORMS.md
│   └── domain-specific.md
└── assets/                    # Optional: templates, images, data files
```

**Skill Storage Locations & Precedence:**

| Level | Path | Scope | Precedence |
|-------|------|-------|------------|
| **Enterprise** | Managed settings | Global org | Highest |
| **Personal** | `~/.claude/skills/[skill-name]/SKILL.md` | All projects | High |
| **Project** | `.claude/skills/[skill-name]/SKILL.md` | Current project only | Medium |
| **Nested/Subdirectory** | `apps/web/.claude/skills/[skill-name]/SKILL.md` | Subdirectory scope (monorepos) | Medium |
| **Plugin** | `/skills/[skill-name]/SKILL.md` | Plugin-specific only | Low |

**Backward Compatibility:** Legacy `.claude/commands/deploy.md` files are treated identically to `.claude/skills/deploy/SKILL.md`. Directories take precedence if naming conflicts occur.

**Cross-Client Convention:** The `.agents/skills/` path has emerged as a widely-adopted convention for cross-client skill sharing across different AI tools.

### SKILL.md Format with YAML Frontmatter

```markdown
---
name: my-skill-name
description: A clear description of what this skill does and when to use it
when_to_use: Additional trigger scenarios or trigger phrases
argument-hint: "[param1] [param2]"
arguments:
  - param1
  - param2
disable-model-invocation: false
user-invocable: true
allowed-tools: Read Grep Bash
disallowed-tools: AskUserQuestion
model: claude-3-5-sonnet
effort: high
context: fork
agent: Explore
shell: bash
license: Apache-2.0
compatibility: Requires Node.js 18+
metadata:
  category: development
  author: team-name
---

# My Skill Name

[Instructions that Claude follows when this skill is active]

## Examples
- Example usage 1
- Example usage 2

## Guidelines
- Guideline 1
- Guideline 2
```

### Frontmatter Field Reference

| Field | Required | Constraints | Description |
|-------|----------|-------------|-------------|
| `name` | Yes | 1-64 chars, lowercase alphanumeric + hyphens, no consecutive/leading/trailing hyphens, must match directory name | Unique identifier |
| `description` | Yes | 1-1024 chars (200 chars for Claude.ai), non-empty | What skill does + when to use it; primary trigger mechanism |
| `when_to_use` | No | — | Additional trigger scenarios/phrases |
| `argument-hint` | No | — | Explains arguments to CLI autocomplete |
| `arguments` | No | — | Positional arguments mapped to placeholders |
| `disable-model-invocation` | No | Default: `false` | Set `true` to prevent auto-triggering; manual `/command` only |
| `user-invocable` | No | Default: `true` | Set `false` to hide from CLI `/` menu |
| `allowed-tools` | No | Space-separated string | Pre-approved tools for skill execution |
| `disallowed-tools` | No | Space-separated string | Tools restricted during skill execution |
| `model` | No | — | Override session model for current turn |
| `effort` | No | `low`, `medium`, `high`, `xhigh`, `max` | Override reasoning effort level |
| `context: fork` | No | — | Run skill in isolated subagent context |
| `agent` | No | `Explore`, `Plan` | Subagent type when `context: fork` is active |
| `shell` | No | `bash`, `powershell` | Shell processor for inline command injection |
| `license` | No | — | License name or bundled license file reference |
| `compatibility` | No | Max 500 chars | Environment requirements (packages, OS, etc.) |
| `metadata` | No | Key-value mapping | Arbitrary additional metadata |

**Size Guidelines:**
- Keep SKILL.md body under **500 lines** (recommended < 5,000 words / ~5,000 tokens)
- Move detailed reference material to `references/` directory
- Metadata (name + description) always loaded: ~100 words/~100 tokens

### Progressive Disclosure Model (Three-Phase Loading)

| Phase | What's Loaded | When | Token Cost |
|-------|--------------|------|------------|
| **1. Catalog/Metadata** | `name` + `description` | Session start | ~50–100 tokens per skill |
| **2. Instructions** | Full `SKILL.md` body | When skill is activated | < 5,000 tokens (recommended) |
| **3. Resources** | Scripts, references, assets | When instructions reference them | Varies |

**Activation Mechanisms:**
1. **Model-driven:** The model reads the catalog and decides to activate; uses `file-read` tool or dedicated `activate_skill` tool
2. **User-driven:** User types `/skill-name` — harness intercepts and injects skill content directly

**Critical Design Rule:** All "when to use" info goes in `description` — NOT in the body. Body is only loaded after triggering. Claude tends to "undertrigger" — make descriptions slightly pushy.

### Dynamic Context Injection (Pre-execution Shell)

Before Claude processes the skill, embedded shell commands are executed to inject real-time context. Use `!` prefix on backticks or code blocks:

```markdown
## Live Diff
!`git diff HEAD`

## System Environment
```!
node --version
git status --short
```
```

### Built-in Variables

| Variable | Description |
|----------|-------------|
| `$ARGUMENTS` | Full argument string passed to skill |
| `$ARGUMENTS[N]` or `$N` | Specific argument by index (e.g., `$0`, `$1`) |
| `$name` | Named parameter from `arguments` frontmatter |
| `${CLAUDE_SESSION_ID}` | Current active session ID |
| `${CLAUDE_EFFORT}` | Current reasoning effort level |
| `${CLAUDE_SKILL_DIR}` | Path to the skill's directory |
| `${CLAUDE_PROJECT_DIR}` | Root path of the current project |

**Security Setting:** Set `"disableSkillShellExecution": true` in user settings to globally disable dynamic command execution.

### SDKs & Tooling

**Official Python SDK (Anthropic):**
```python
from anthropic import Anthropic
from anthropic.lib import files_from_dir

client = Anthropic(
    api_key=ANTHROPIC_API_KEY,
    default_headers={"anthropic-beta": "skills-2025-10-02"}
)

skill = client.beta.skills.create(
    display_title="My Skill",
    files=files_from_dir("path/to/skill")
)

response = client.beta.messages.create(
    container={"skills": [{"type": "custom", "skill_id": skill.id, "version": "latest"}]},
    messages=[{"role": "user", "content": "Create a presentation..."}],
    tools=[{"type": "code_execution_20260521", "name": "code_execution"}]
)
```

**API Endpoints:**
- `POST /v1/skills` — Create skill
- `GET /v1/skills` — List skills (with `source` filter: `custom` or `anthropic`)
- `GET /v1/skills/{skill_id}/versions/{version}` — Download skill content

**Required Beta Header:** `anthropic-beta: skills-2025-10-02`

**claude-skills-cli (npm) Validation Modes:**
```bash
pnpx claude-skills-cli validate .claude/skills/my-skill           # Default: 50 lines max
pnpx claude-skills-cli validate .claude/skills/my-skill --lenient # 150 lines
pnpx claude-skills-cli validate .claude/skills/my-skill --loose   # 500 lines (official limit)
pnpx claude-skills-cli validate .claude/skills/my-skill --strict  # Fail on warnings
pnpx claude-skills-cli validate .claude/skills/my-skill --format json
pnpx claude-skills-cli package .claude/skills/my-skill            # Package as ZIP
pnpx claude-skills-cli add-hook --type forced-eval                # Add activation hook (84% success)
```

**Hook Types:**
| Hook Type | Success Rate | Description |
|-----------|-------------|-------------|
| `forced-eval` | 84% | Mandatory 3-step evaluation (default) |
| `llm-eval` | 80% | Claude API pre-evaluation (~$0.0004/prompt) |
| `simple-script` | 20% | Basic script file |
| `simple-inline` | 20% | Echo in settings.json |

### Bundled Skills in Claude Code
- `/doctor` — System diagnostics
- `/code-review` — Code review workflow
- `/batch` — Batch processing
- `/debug` — Debugging assistance
- `/loop` — Iterative refinement
- `/claude-api` — API integration
- `/run` — Launch and drive your app
- `/verify` — Build and run verification
- `/run-skill-generator` — Teach run/verify how to build your project

### Comparison with Other Skill/Prompt Systems

| Dimension | **Claude Skills** | **ChatGPT Custom Instructions** | **ChatGPT GPTs** | **Cursor Rules** |
|-----------|-------------------|--------------------------------|------------------|------------------|
| Format | `SKILL.md` + YAML frontmatter + Markdown | Two text fields in web UI | Web-based config + knowledge files + Actions | `.cursorrules` or `.cursor/rules/*.mdc` |
| Max Size | Unlimited (context-managed) | ~1,500 chars per field | ~8,000 chars | ~4,000 tokens recommended |
| Versioning | Native Git | None | None | Git-tracked |
| Collaboration | Shared repo / Git | Difficult | Shareable URL | Git-based |
| Multi-editor | Yes (Claude Code, Codex CLI, OpenClaw, Cursor partial) | No | No | Cursor only |
| Filesystem Access | Full local access | None | None | Project-level |
| Terminal Execution | Yes | No | No | Limited |
| API Actions | Via scripts/MCP | No | Yes (Actions) | Via commands/plugins |
| Composability | Full (multiple skills chain) | No | Limited | No |
| Auto-activation | Semantic description matching | N/A | N/A | `alwaysApply` or `globs` |
| Progressive Disclosure | Yes (3-tier loading) | No | No | No |
| Open Standard | agentskills.io | OpenAI-only | OpenAI-only | Cursor-only |

**Best Practice Hybrid Approach:**
- Use **Cursor Rules** for repo-wide always-on standards (naming, linting, architecture)
- Use **Claude Skills** for task-specific, multi-step workflows (deploy, review, generate docs)
- Use **Custom Instructions** for quick personal preferences in web UI
- They can coexist — a project can have both `.cursorrules` and `CLAUDE.md` without conflict

**MCP Integration:** Claude Skills can reference MCP tools using fully qualified names: `ServerName:tool_name`. There is an active proposal to replace MCP "prompts" with "skills" for agent-driven workflow execution: https://github.com/modelcontextprotocol/modelcontextprotocol/discussions/1779

### Key for Runyard
- Skills are **managed by the IDE but executed by the agent**. The IDE's responsibility: file management, CRUD UI, validation, directory scanning, progressive disclosure metadata.
- The agent's responsibility: triggering, loading, execution, shell command injection, variable substitution.
- Runyard's Skills UI (Phase 1) will let users create, edit, and manage skills. The IDE passes skill directories to the agent via ACP `session/new` or `session/set_config_option`.
- Cross-compatibility: Runyard skills should be loadable from `.claude/skills/`, `.cursor/skills/`, `.agents/skills/`, and standard formats should work in other clients.
- The `claude-skills-cli` validation rules should be integrated into Runyard's skill editor for real-time validation.

## Protocol Architecture — How They Stack

### The Protocol Stack

```
┌─────────────────────────────────────────────────────────────────┐
│  RUNYARD IDE (Phase 1)                                          │
│                                                                 │
│  ┌─────────────────┐    ACP (JSON-RPC/stdio/WS)    ┌─────────┐ │
│  │  Editor/Chat     │◄──────────────────────────►│ Agent   │ │
│  │  (ACP Client)    │                            │(Server) │ │
│  └─────────────────┘                              └────┬────┘ │
│                                                       │      │
│  ┌───────────────────────────────────────────────────┘      │
│  │                                                              │
│  │  MCP Server Manager UI (IDE configures, Agent connects)      │
│  │  Skills UI (IDE manages files, Agent loads/executes)         │
│  │  Agent Runner UI (IDE discovers/launches, monitors status)   │
│  └──────────────────────────────────────────────────────────────┘
│                                                                 │
│  Phase 2: Built-in Agent (ACP Server + MCP Client + A2A Client) │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Runyard Built-in Agent                                  │  │
│  │  ├─ ACP Server (speaks to IDE)                          │  │
│  │  ├─ MCP Client (connects to tools via rmcp)              │  │
│  │  ├─ A2A Client (delegates to other agents)               │  │
│  │  ├─ Provider SDK (Anthropic, OpenAI, Google, etc.)       │  │
│  │  ├─ Context Assembly & Token Management                  │  │
│  │  └─ Cost Tracking & Budget Enforcement                   │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  EXTERNAL AGENTS (ACP Servers, discovered by IDE)               │
│                                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │ Claude   │  │ Gemini   │  │ Codex    │  │ Roo      │       │
│  │ Code     │  │ CLI      │  │ CLI      │  │ Code     │       │
│  │(ACP Srv) │  │(ACP Srv) │  │(ACP Srv) │  │(ACP Srv) │       │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘       │
│       │             │             │             │              │
│       └─────────────┴─────────────┴─────────────┘              │
│                     ACP (stdio/HTTP/WS)                         │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  MCP TOOLS (MCP Servers, connected by Agent)                    │
│                                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │ GitHub   │  │ Postgres │  │ Browser  │  │ Filesystem│       │
│  │ Server   │  │ Server   │  │ Server   │  │ Server    │       │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘       │
│       │             │             │             │              │
│       └─────────────┴─────────────┴─────────────┘              │
│                     MCP (JSON-RPC/stdio/HTTP)                     │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│  A2A AGENTS (Agent-to-Agent, used by Orchestrator)            │
│                                                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                     │
│  │ Agent A  │  │ Agent B  │  │ Agent C  │                     │
│  │(A2A Peer)│  │(A2A Peer)│  │(A2A Peer)│                     │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘                     │
│       └─────────────┴─────────────┘                             │
│                     A2A (HTTP/gRPC/SSE)                         │
└─────────────────────────────────────────────────────────────────┘
```

### Protocol Roles in Runyard

| Protocol | Layer | Direction | Runyard Role | Analogy |
|----------|-------|-----------|-------------|---------|
| **ACP** | Editor ↔ Agent | Client → Server | **IDE is ACP Client** | "LSP for AI agents" |
| **MCP** | Agent ↔ Tools | Client → Server | **Agent is MCP Client** | "USB-C for AI tools" |
| **A2A** | Agent ↔ Agent | Peer-to-peer | **Orchestrator is A2A Client** | "Wi-Fi for agents" |
| **Skills** | Agent ↔ Instructions | File-based | **IDE manages files, Agent executes** | "Agent knowledge modules" |

### Critical Architecture Principles

1. **The IDE NEVER speaks directly to LLM providers.** The IDE only speaks ACP. All provider-specific logic (API keys, streaming, token counting, context assembly) lives in the agent — whether external (Claude Code, Gemini CLI) or the built-in Runyard agent (Phase 2).

2. **The IDE NEVER speaks MCP directly.** MCP is an agent-level protocol. The IDE has a UI for managing MCP server configurations (add, edit, remove), but the agent is what connects to and calls MCP servers. The IDE learns about tool calls through ACP `session/update` notifications.

3. **The IDE NEVER speaks A2A directly.** A2A is an agent-to-agent protocol. The orchestrator agent (Phase 2) uses A2A to delegate to other agents. The IDE only sees the orchestrator's results through ACP.

4. **Skills are file-based but agent-executed.** The IDE manages the skill files (create, edit, delete, organize), validates frontmatter, and scans directories. The agent is what reads the SKILL.md, triggers activation, executes shell commands, and handles variable substitution.

5. **ACP is the MOST IMPORTANT protocol.** Without ACP, the IDE cannot talk to any agent. Without MCP, the agent has fewer tools. Without A2A, the orchestrator cannot delegate. Without skills, the agent is less capable. But the IDE can still edit code without MCP, A2A, or skills — it cannot work without ACP.

### The ACP-First Implementation Order

For Runyard's MVP (Phase 1), the implementation order is:

1. **ACP Client in IDE** — MUST be complete and robust. This is the foundation. The IDE discovers, launches, and communicates with any ACP-compatible agent. Every ACP feature (all methods, all notifications, all transports, session lifecycle, tool calling, permission requests, streaming, config options) must be implemented.

2. **Agent Discovery & Runner UI** — The IDE detects installed agents on the user's machine (Claude Code, Gemini CLI, Codex, Roo, etc.) and lets the user choose, configure, and switch between them. The agent can be a local executable (stdio), a remote URL (HTTP/WebSocket), or a command to spawn.

3. **MCP Server Manager UI** — The IDE provides UI for adding, configuring, and managing MCP servers. The configuration is passed to the agent via ACP `session/new` or `session/set_config_option`. The agent (not the IDE) connects to MCP servers.

4. **Skills Management UI** — The IDE provides UI for creating, editing, and organizing skills. The skill directories are passed to the agent via ACP. The agent handles loading and execution.

5. **A2A Client in Orchestrator** — Phase 2. The built-in Runyard agent uses A2A to delegate to other agents.

### What This Means for the Engineering TODO

- **Phase 1 (MVP):** The IDE implements a complete ACP client. It discovers and runs external ACP agents (Claude Code, Gemini CLI, Codex, Roo, etc.). No built-in agent yet. The agent handles all LLM/provider logic.
- **Phase 2 (Built-in Agent):** Runyard ships its own built-in agent. This agent is an ACP server that the IDE connects to. It includes provider SDK integration, MCP client, A2A client, context assembly, and cost tracking. External agents still work — the user can switch between built-in and external agents.
- **Phase 3+ (Orchestrator):** The built-in agent gains orchestrator capabilities using A2A to delegate to subagents.
