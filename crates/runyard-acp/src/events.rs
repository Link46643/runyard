use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Stable event surface Runyard's Tauri bridge (and eventually the Svelte
/// frontend) consumes. This is deliberately Runyard's OWN shape, decoupled
/// from `agent_client_protocol`'s wire types - the mapping from the real
/// `SessionUpdate` enum into these variants lives in `client.rs`, so a
/// protocol schema change only requires updating one mapping function, not
/// every consumer of this crate.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AcpEvent {
    /// Connection established and initialize handshake completed.
    Connected {
        connection_id: String,
        agent_name: Option<String>,
        agent_capabilities: Value,
        auth_methods: Vec<String>,
    },
    /// Connection lost or closed (cleanly or not).
    Disconnected { connection_id: String, reason: String },
    /// A new session was created.
    SessionStarted { connection_id: String, session_id: String },
    /// A session was closed.
    SessionClosed { connection_id: String, session_id: String },
    /// Text/content chunk streamed from the agent's response.
    AgentMessageChunk {
        connection_id: String,
        session_id: String,
        text: String,
    },
    /// Echo of the user's own message (confirms receipt).
    UserMessageChunk {
        connection_id: String,
        session_id: String,
        text: String,
    },
    /// Reasoning / extended-thinking content.
    ThoughtChunk {
        connection_id: String,
        session_id: String,
        text: String,
    },
    /// Agent is requesting a tool call - surfaced before permission is asked.
    ToolCall {
        connection_id: String,
        session_id: String,
        tool_call_id: String,
        name: String,
        arguments: Value,
    },
    /// A previously-announced tool call finished (or updated) with content/locations.
    ToolCallUpdate {
        connection_id: String,
        session_id: String,
        tool_call_id: String,
        status: String,
        content: Value,
    },
    /// The agent's execution plan changed.
    PlanUpdate {
        connection_id: String,
        session_id: String,
        plan: Value,
    },
    /// The set of commands/tools the agent supports changed.
    AvailableCommandsUpdate {
        connection_id: String,
        session_id: String,
        commands: Value,
    },
    /// Session metadata changed (title, etc.).
    SessionInfoUpdate {
        connection_id: String,
        session_id: String,
        info: Value,
    },
    /// Agent is asking the user for permission before taking an action.
    PermissionRequested {
        connection_id: String,
        session_id: String,
        request_id: String,
        tool_name: String,
        arguments: Value,
        options: Vec<PermissionOption>,
    },
    /// The final response to a `session/prompt` call arrived.
    PromptCompleted {
        connection_id: String,
        session_id: String,
        stop_reason: String,
    },
    /// Something in the connection/session failed.
    Error {
        connection_id: String,
        session_id: Option<String>,
        code: String,
        message: String,
        recoverable: bool,
    },
    /// Best-effort connection health signal (used for reconnect backoff / status UI).
    StatusChanged {
        connection_id: String,
        status: ConnectionStatus,
    },
    /// A raw line sent to or received from the agent process (stdio
    /// transport only). Covers both the JSON-RPC traffic itself and, for
    /// `Stderr`, the agent's own log/diagnostic output - this is the raw
    /// material for Runyard's agent log viewer (1.6.9) and satisfies "handle
    /// stderr separately" (1.7.2) for real, not just on-crash.
    LogLine {
        connection_id: String,
        direction: LogDirection,
        line: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LogDirection {
    Stdin,
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Idle,
    Initializing,
    Ready,
    Processing,
    Error,
    Disconnected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionOption {
    pub option_id: String,
    pub label: String,
}
