use thiserror::Error;

/// Errors surfaced by the Runyard ACP client. Every variant carries enough
/// context to build a user-facing message without needing to inspect the
/// underlying protocol error directly.
#[derive(Error, Debug)]
pub enum AcpClientError {
    #[error("failed to spawn agent process: {0}")]
    SpawnFailed(String),

    #[error("failed to connect to remote agent at {url}: {reason}")]
    ConnectFailed { url: String, reason: String },

    #[error("ACP protocol error: {0}")]
    Protocol(String),

    #[error("connection closed unexpectedly")]
    ConnectionClosed,

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("agent does not support required capability: {0}")]
    CapabilityMismatch(String),

    #[error("protocol version mismatch: client supports {client_version}, agent requires {agent_version}")]
    VersionMismatch {
        client_version: String,
        agent_version: String,
    },

    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("connection pool limit reached ({0} concurrent connections)")]
    PoolLimitReached(usize),

    #[error("request timed out")]
    Timeout,

    #[error("client is shutting down")]
    ShuttingDown,

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type AcpResult<T> = Result<T, AcpClientError>;
