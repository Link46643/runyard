//! Runyard's ACP (Agent Client Protocol) client.
//!
//! Deliberately Tauri-free: this crate depends only on `agent-client-protocol`,
//! tokio, and plain serialization crates, so it can be built and tested on
//! its own. The Tauri bridge (Tauri commands + events forwarding to the
//! Svelte frontend) lives in `apps/desktop/src-tauri` and depends on this
//! crate, not the other way around.
//!
//! # Architecture
//!
//! The real `agent-client-protocol` crate scopes an entire connection inside
//! one async closure passed to `Client::builder().connect_with(...)`. That's
//! awkward for a long-lived GUI app where Tauri commands need to interact
//! with a live connection over its whole lifetime, not just one linear
//! script. [`RunyardAcpClient::connect`] runs that closure in a background
//! tokio task and exposes a plain command/event channel API instead - see
//! [`commands::ClientCommand`] (in) and [`events::AcpEvent`] (out).

pub mod client;
pub mod commands;
pub mod error;
pub mod events;
pub mod pool;
pub mod transport;
pub mod updates;

#[cfg(feature = "test-utils")]
pub mod mock_agent;

pub use client::RunyardAcpClient;
pub use error::{AcpClientError, AcpResult};
pub use events::{AcpEvent, ConnectionStatus, PermissionOption};
pub use pool::AcpConnectionPool;
pub use transport::AgentTransportConfig;
