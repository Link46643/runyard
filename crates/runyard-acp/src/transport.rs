use serde::{Deserialize, Serialize};

/// How to reach an ACP agent. Stdio is the stable, default, primary
/// transport per the protocol spec and covers every agent in the "Supported
/// Agents" list (Claude Code, Gemini CLI, Codex, Goose, etc.) - all of them
/// run as local subprocesses. HTTP and WebSocket are still Draft RFD status
/// in the protocol itself (as of this spec version), so they're implemented
/// best-effort here rather than treated as equally mature.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentTransportConfig {
    /// Spawn a local agent process. `command` is a shell-style command
    /// string, e.g. `"claude --acp"` or `"gemini --experimental-acp"`.
    Stdio { command: String },
    /// Connect to a remote agent over Streamable HTTP (POST + SSE).
    Http { url: String },
    /// Connect to a remote agent over WebSocket.
    WebSocket { url: String },
}

impl AgentTransportConfig {
    pub fn stdio(command: impl Into<String>) -> Self {
        Self::Stdio { command: command.into() }
    }
    pub fn http(url: impl Into<String>) -> Self {
        Self::Http { url: url.into() }
    }
    pub fn websocket(url: impl Into<String>) -> Self {
        Self::WebSocket { url: url.into() }
    }

    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Stdio { .. } => "stdio",
            Self::Http { .. } => "http",
            Self::WebSocket { .. } => "websocket",
        }
    }
}
