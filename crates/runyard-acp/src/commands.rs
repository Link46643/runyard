use serde_json::Value;
use tokio::sync::oneshot;

use crate::error::AcpClientError;

/// Commands accepted by a connection's background task. The real
/// `agent-client-protocol` API scopes an entire session inside one async
/// closure passed to `connect_with`, which is awkward for a long-lived GUI
/// app where Tauri commands need to poke a live connection whenever the user
/// acts. This channel is the bridge: `RunyardAcpClient`'s public methods
/// send a `ClientCommand` in and (usually) await a oneshot reply.
#[derive(Debug)]
pub enum ClientCommand {
    NewSession {
        cwd: String,
        mcp_servers: Vec<Value>,
        reply: oneshot::Sender<Result<String, AcpClientError>>,
    },
    LoadSession {
        session_id: String,
        cwd: String,
        reply: oneshot::Sender<Result<(), AcpClientError>>,
    },
    ResumeSession {
        session_id: String,
        cwd: String,
        reply: oneshot::Sender<Result<(), AcpClientError>>,
    },
    ListSessions {
        reply: oneshot::Sender<Result<Vec<String>, AcpClientError>>,
    },
    Authenticate {
        method_id: String,
        reply: oneshot::Sender<Result<(), AcpClientError>>,
    },
    CloseSession {
        session_id: String,
        reply: oneshot::Sender<Result<(), AcpClientError>>,
    },
    SendPrompt {
        session_id: String,
        text: String,
        reply: oneshot::Sender<Result<(), AcpClientError>>,
    },
    Cancel {
        session_id: String,
        reply: oneshot::Sender<Result<(), AcpClientError>>,
    },
    RespondPermission {
        request_id: String,
        option_id: Option<String>,
        reply: oneshot::Sender<Result<(), AcpClientError>>,
    },
    SetConfigOption {
        session_id: String,
        key: String,
        value: Value,
        reply: oneshot::Sender<Result<(), AcpClientError>>,
    },
    SetMode {
        session_id: String,
        mode: String,
        reply: oneshot::Sender<Result<(), AcpClientError>>,
    },
    Logout {
        reply: oneshot::Sender<Result<(), AcpClientError>>,
    },
    Shutdown {
        reply: oneshot::Sender<()>,
    },
}
