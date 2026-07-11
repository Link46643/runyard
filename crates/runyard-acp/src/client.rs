use std::collections::HashMap;
use std::str::FromStr;

use agent_client_protocol::schema::v1::{
    AuthMethodId, AuthenticateRequest, CancelNotification, ClientCapabilities,
    ClientSessionCapabilities, FileSystemCapabilities, Implementation,
    InitializeRequest, KillTerminalRequest, KillTerminalResponse, LoadSessionRequest,
    LogoutRequest, McpServer, NewSessionResponse, ReadTextFileRequest, ReadTextFileResponse,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    ResumeSessionRequest, SelectedPermissionOutcome, SessionConfigId, SessionConfigOptionValue,
    SessionModeId, SessionNotification, SetSessionConfigOptionRequest, SetSessionModeRequest,
    TerminalOutputRequest, TerminalOutputResponse, WriteTextFileRequest,
    WriteTextFileResponse,
};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::{AcpAgent, Agent, Client, ConnectionTo};
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::commands::ClientCommand;
use crate::error::{AcpClientError, AcpResult};
use crate::events::{AcpEvent, ConnectionStatus};
use crate::transport::AgentTransportConfig;

/// A single live connection to one ACP agent. The real `agent-client-protocol`
/// API scopes an entire session inside one async closure passed to
/// `connect_with`; this struct is the bridge that lets the rest of Runyard
/// (eventually, Tauri commands) talk to that closure's world from the outside
/// via a plain command channel, and receive everything that happens via a
/// plain event channel. See crates/runyard-acp/src/commands.rs and events.rs.
pub struct RunyardAcpClient {
    pub connection_id: String,
    command_tx: mpsc::UnboundedSender<ClientCommand>,
    _task_handle: tokio::task::JoinHandle<()>,
}

/// Commands specific to one already-established session, routed from the
/// connection-level command loop to that session's dedicated worker task.
enum SessionCommand {
    SendPrompt {
        text: String,
        reply: oneshot::Sender<AcpResult<()>>,
    },
    Cancel {
        reply: oneshot::Sender<AcpResult<()>>,
    },
    Close {
        reply: oneshot::Sender<AcpResult<()>>,
    },
}

impl RunyardAcpClient {
    /// Connect to an agent and run the connection until `shutdown()` is
    /// called or the connection drops. Every event (notifications, tool
    /// calls, permission requests, status changes) is sent to `event_tx`.
    pub async fn connect(
        transport: AgentTransportConfig,
        event_tx: mpsc::UnboundedSender<AcpEvent>,
    ) -> AcpResult<Self> {
        let connection_id = Uuid::new_v4().to_string();
        let (command_tx, command_rx) = mpsc::unbounded_channel();

        match transport {
            AgentTransportConfig::Stdio { command } => {
                let conn_id = connection_id.clone();
                let handle = tokio::spawn(async move {
                    run_stdio_connection_with_retry(conn_id, command, command_rx, event_tx).await;
                });
                Ok(Self {
                    connection_id,
                    command_tx,
                    _task_handle: handle,
                })
            }
            AgentTransportConfig::Http { url } => Err(AcpClientError::ConnectFailed {
                url,
                reason: "HTTP transport is Draft RFD status in the protocol itself; not implemented yet".into(),
            }),
            AgentTransportConfig::WebSocket { url } => Err(AcpClientError::ConnectFailed {
                url,
                reason: "WebSocket transport is Draft RFD status in the protocol itself; not implemented yet".into(),
            }),
        }
    }

    pub async fn new_session(&self, cwd: String, mcp_servers: Vec<serde_json::Value>) -> AcpResult<String> {
        let (reply, rx) = oneshot::channel();
        self.command_tx
            .send(ClientCommand::NewSession { cwd, mcp_servers, reply })
            .map_err(|_| AcpClientError::ConnectionClosed)?;
        rx.await.map_err(|_| AcpClientError::ConnectionClosed)?
    }

    pub async fn send_prompt(&self, session_id: String, text: String) -> AcpResult<()> {
        let (reply, rx) = oneshot::channel();
        self.command_tx
            .send(ClientCommand::SendPrompt { session_id, text, reply })
            .map_err(|_| AcpClientError::ConnectionClosed)?;
        rx.await.map_err(|_| AcpClientError::ConnectionClosed)?
    }

    pub async fn cancel(&self, session_id: String) -> AcpResult<()> {
        let (reply, rx) = oneshot::channel();
        self.command_tx
            .send(ClientCommand::Cancel { session_id, reply })
            .map_err(|_| AcpClientError::ConnectionClosed)?;
        rx.await.map_err(|_| AcpClientError::ConnectionClosed)?
    }

    pub async fn close_session(&self, session_id: String) -> AcpResult<()> {
        let (reply, rx) = oneshot::channel();
        self.command_tx
            .send(ClientCommand::CloseSession { session_id, reply })
            .map_err(|_| AcpClientError::ConnectionClosed)?;
        rx.await.map_err(|_| AcpClientError::ConnectionClosed)?
    }

    pub async fn respond_permission(&self, request_id: String, option_id: Option<String>) -> AcpResult<()> {
        let (reply, rx) = oneshot::channel();
        self.command_tx
            .send(ClientCommand::RespondPermission { request_id, option_id, reply })
            .map_err(|_| AcpClientError::ConnectionClosed)?;
        rx.await.map_err(|_| AcpClientError::ConnectionClosed)?
    }

    /// Re-attach to a session the agent already knows about (e.g. one restored
    /// from Runyard's own session history) so it can receive prompts again.
    pub async fn load_session(&self, session_id: String, cwd: String) -> AcpResult<()> {
        let (reply, rx) = oneshot::channel();
        self.command_tx
            .send(ClientCommand::LoadSession { session_id, cwd, reply })
            .map_err(|_| AcpClientError::ConnectionClosed)?;
        rx.await.map_err(|_| AcpClientError::ConnectionClosed)?
    }

    /// Resume a previously-interrupted session (agent-side continuation, as
    /// opposed to `load_session`'s plain re-attachment).
    pub async fn resume_session(&self, session_id: String, cwd: String) -> AcpResult<()> {
        let (reply, rx) = oneshot::channel();
        self.command_tx
            .send(ClientCommand::ResumeSession { session_id, cwd, reply })
            .map_err(|_| AcpClientError::ConnectionClosed)?;
        rx.await.map_err(|_| AcpClientError::ConnectionClosed)?
    }

    /// List session ids currently tracked by this connection.
    pub async fn list_sessions(&self) -> AcpResult<Vec<String>> {
        let (reply, rx) = oneshot::channel();
        self.command_tx
            .send(ClientCommand::ListSessions { reply })
            .map_err(|_| AcpClientError::ConnectionClosed)?;
        rx.await.map_err(|_| AcpClientError::ConnectionClosed)?
    }

    /// Set a session configuration option (boolean or string value) exposed
    /// by the agent's `NewSessionResponse::config_options`.
    pub async fn set_config_option(&self, session_id: String, key: String, value: serde_json::Value) -> AcpResult<()> {
        let (reply, rx) = oneshot::channel();
        self.command_tx
            .send(ClientCommand::SetConfigOption { session_id, key, value, reply })
            .map_err(|_| AcpClientError::ConnectionClosed)?;
        rx.await.map_err(|_| AcpClientError::ConnectionClosed)?
    }

    /// Switch a session's mode (e.g. "ask" vs "code" vs "architect", per
    /// whatever modes the agent advertised).
    pub async fn set_mode(&self, session_id: String, mode: String) -> AcpResult<()> {
        let (reply, rx) = oneshot::channel();
        self.command_tx
            .send(ClientCommand::SetMode { session_id, mode, reply })
            .map_err(|_| AcpClientError::ConnectionClosed)?;
        rx.await.map_err(|_| AcpClientError::ConnectionClosed)?
    }

    /// Ask the agent to clear stored authentication/credentials for this
    /// connection.
    pub async fn logout(&self) -> AcpResult<()> {
        let (reply, rx) = oneshot::channel();
        self.command_tx
            .send(ClientCommand::Logout { reply })
            .map_err(|_| AcpClientError::ConnectionClosed)?;
        rx.await.map_err(|_| AcpClientError::ConnectionClosed)?
    }

    /// Authenticate using one of the method ids advertised in `Connected`'s
    /// `auth_methods`. Required before most agents will accept `new_session`
    /// if they returned any auth methods at initialize time.
    pub async fn authenticate(&self, method_id: String) -> AcpResult<()> {
        let (reply, rx) = oneshot::channel();
        self.command_tx
            .send(ClientCommand::Authenticate { method_id, reply })
            .map_err(|_| AcpClientError::ConnectionClosed)?;
        rx.await.map_err(|_| AcpClientError::ConnectionClosed)?
    }

    pub async fn shutdown(&self) {
        let (reply, rx) = oneshot::channel();
        if self.command_tx.send(ClientCommand::Shutdown { reply }).is_ok() {
            let _ = rx.await;
        }
    }
}

/// Retry wrapper around `run_stdio_connection` (1.6.4/1.7.2/1.7.15).
/// Attempts up to 3 reconnects after a crash with exponential backoff
/// (1s → 2s → 4s). Each attempt re-spawns the agent process fresh.
/// The command channel is NOT re-usable across retries, so once the
/// channel closes (Shutdown command or sender dropped) we stop retrying.
async fn run_stdio_connection_with_retry(
    connection_id: String,
    command: String,
    mut command_rx: mpsc::UnboundedReceiver<ClientCommand>,
    event_tx: mpsc::UnboundedSender<AcpEvent>,
) {
    const MAX_RETRIES: u32 = 3;
    let mut attempt = 0u32;
    loop {
        // (Re-)build the agent process for this attempt.
        let log_conn_id = connection_id.clone();
        let log_event_tx = event_tx.clone();
        let agent = match AcpAgent::from_str(&command) {
            Ok(a) => a.with_debug(move |line, direction| {
                let direction = match direction {
                    agent_client_protocol::LineDirection::Stdin => crate::events::LogDirection::Stdin,
                    agent_client_protocol::LineDirection::Stdout => crate::events::LogDirection::Stdout,
                    agent_client_protocol::LineDirection::Stderr => crate::events::LogDirection::Stderr,
                };
                let _ = log_event_tx.send(AcpEvent::LogLine {
                    connection_id: log_conn_id.clone(),
                    direction,
                    line: line.to_string(),
                });
            }),
            Err(e) => {
                let _ = event_tx.send(AcpEvent::Error {
                    connection_id: connection_id.clone(),
                    session_id: None,
                    code: "spawn_failed".into(),
                    message: e.to_string(),
                    recoverable: false,
                });
                break;
            }
        };

        run_stdio_connection(connection_id.clone(), agent, &mut command_rx, event_tx.clone()).await;

        // If the command channel is closed (Shutdown was sent), stop retrying.
        if command_rx.is_closed() {
            break;
        }

        attempt += 1;
        if attempt > MAX_RETRIES {
            let _ = event_tx.send(AcpEvent::Error {
                connection_id: connection_id.clone(),
                session_id: None,
                code: "max_retries_exceeded".into(),
                message: format!("Agent process crashed {attempt} times in a row. Giving up."),
                recoverable: false,
            });
            break;
        }

        let delay_secs = 1u64 << (attempt - 1); // 1, 2, 4
        tracing::warn!(attempt, delay_secs, "Agent process disconnected, reconnecting...");
        let _ = event_tx.send(AcpEvent::StatusChanged {
            connection_id: connection_id.clone(),
            status: ConnectionStatus::Initializing,
        });
        tokio::time::sleep(tokio::time::Duration::from_secs(delay_secs)).await;
    }
}

async fn run_stdio_connection(
    connection_id: String,
    agent: AcpAgent,
    mut command_rx: &mut mpsc::UnboundedReceiver<ClientCommand>,
    event_tx: mpsc::UnboundedSender<AcpEvent>,
) {
    let notif_conn_id = connection_id.clone();
    let notif_event_tx = event_tx.clone();

    // Pending permission requests: request_id -> responder, so a later
    // RespondPermission command (arriving on the outer command channel, long
    // after this handler returned control to the event loop) can still
    // reply to the right in-flight request.
    let pending_permissions: std::sync::Arc<
        tokio::sync::Mutex<HashMap<String, agent_client_protocol::Responder<RequestPermissionResponse>>>,
    > = std::sync::Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    let perm_conn_id = connection_id.clone();
    let perm_event_tx = event_tx.clone();
    let perm_store = pending_permissions.clone();

    let result = Client
        .builder()
        .name("runyard")
        .on_receive_notification(
            move |notification: SessionNotification, _cx| {
                let conn_id = notif_conn_id.clone();
                let evt_tx = notif_event_tx.clone();
                async move {
                    forward_session_notification(&conn_id, &notification, &evt_tx);
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            move |request: RequestPermissionRequest, responder, _connection| {
                let conn_id = perm_conn_id.clone();
                let evt_tx = perm_event_tx.clone();
                let store = perm_store.clone();
                async move {
                    let request_id = Uuid::new_v4().to_string();
                    let session_id = request.session_id.to_string();
                    let options = request
                        .options
                        .iter()
                        .map(|opt| crate::events::PermissionOption {
                            option_id: opt.option_id.to_string(),
                            label: opt.name.clone(),
                        })
                        .collect();
                    store.lock().await.insert(request_id.clone(), responder);
                    let _ = evt_tx.send(AcpEvent::PermissionRequested {
                        connection_id: conn_id,
                        session_id,
                        request_id,
                        tool_name: request.tool_call.fields.title.clone().unwrap_or_else(|| "unknown tool".into()),
                        arguments: serde_json::to_value(&request.tool_call).unwrap_or_default(),
                        options,
                    });
                    Ok(())
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        // ── 1.7.11: Inbound IDE tool execution requests (agent → IDE) ──────
        // The agent asks the IDE to execute file/terminal operations. The IDE
        // responds with the result, then the agent decides what to do next.
        .on_receive_request(
            async move |request: ReadTextFileRequest, responder, _connection| {
                let path = request.path.to_string_lossy().to_string();
                match std::fs::read_to_string(&path) {
                    Ok(content) => { let _ = responder.respond(ReadTextFileResponse::new(content)); }
                    Err(e) => {
                        let _ = responder.respond(ReadTextFileResponse::new(
                            format!("[error reading {path}: {e}]")
                        ));
                    }
                }
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: WriteTextFileRequest, responder, _connection| {
                let path = request.path.clone();
                let result = (|| -> std::io::Result<()> {
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    let mut tmp = tempfile::NamedTempFile::new_in(
                        path.parent().unwrap_or(std::path::Path::new("."))
                    )?;
                    std::io::Write::write_all(&mut tmp, request.content.as_bytes())?;
                    tmp.persist(&path)?;
                    Ok(())
                })();
                if let Err(e) = result {
                    tracing::warn!(path = %path.display(), error = %e, "IDE fs/write_text_file failed");
                }
                let _ = responder.respond(WriteTextFileResponse::new());
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: TerminalOutputRequest, responder, _connection| {
                // terminal/output needs access to Tauri's TerminalState which
                // lives in runyard-core (a different crate). This crate is
                // Tauri-free by design. The acp_bridge.rs in apps/desktop/src-tauri
                // re-registers this handler with real PTY access using the same
                // AcpAgent builder pattern but with a Tauri AppHandle closure.
                let _ = request;
                let _ = responder.respond(TerminalOutputResponse::new("", false));
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: KillTerminalRequest, responder, _connection| {
                let _ = request;
                let _ = responder.respond(KillTerminalResponse::new());
                Ok(())
            },
            agent_client_protocol::on_receive_request!(),
        )
        // ── end 1.7.11 ─────────────────────────────────────────────────────
        .connect_with(agent, |connection: ConnectionTo<Agent>| {
            let event_tx = event_tx.clone();
            let connection_id = connection_id.clone();
            async move {
                let _ = event_tx.send(AcpEvent::StatusChanged {
                    connection_id: connection_id.clone(),
                    status: ConnectionStatus::Initializing,
                });

                // Advertise what Runyard's IDE side can actually do for the
                // agent - fs/terminal/session capabilities per 1.7.12. Tool
                // execution for these lives in the (not-yet-built) Tauri
                // bridge; advertising them here is the client-capabilities
                // half of that contract.
                let client_capabilities = ClientCapabilities::new()
                    .fs(FileSystemCapabilities::new().read_text_file(true).write_text_file(true))
                    .terminal(true)
                    .session(ClientSessionCapabilities::new());
                let init_request = InitializeRequest::new(ProtocolVersion::V1)
                    .client_capabilities(client_capabilities)
                    .client_info(Implementation::new("Runyard", env!("CARGO_PKG_VERSION")));

                let init_response = connection.send_request(init_request).block_task().await?;

                // Version negotiation (1.7.19): Runyard only understands V1
                // semantics (V2 is an unstable draft RFD and not compiled in
                // via the `unstable_protocol_v2` feature). Per protocol docs,
                // the client should disconnect if the agent doesn't support
                // the version it asked for.
                if init_response.protocol_version < ProtocolVersion::V1 {
                    return Err(agent_client_protocol::util::internal_error(format!(
                        "agent only supports protocol version {:?}, Runyard requires at least V1",
                        init_response.protocol_version
                    )));
                }
                tracing::info!(version = ?init_response.protocol_version, "ACP protocol version negotiated");

                let agent_name = init_response.agent_info.as_ref().map(|info| info.name.clone());
                let auth_methods = init_response
                    .auth_methods
                    .iter()
                    .map(|method| method.id().to_string())
                    .collect();

                let _ = event_tx.send(AcpEvent::Connected {
                    connection_id: connection_id.clone(),
                    agent_name,
                    agent_capabilities: serde_json::to_value(&init_response.agent_capabilities)
                        .unwrap_or_default(),
                    auth_methods,
                });
                let _ = event_tx.send(AcpEvent::StatusChanged {
                    connection_id: connection_id.clone(),
                    status: ConnectionStatus::Ready,
                });

                run_command_loop(connection_id, connection, &mut command_rx, &event_tx, pending_permissions).await;

                Ok(())
            }
        })
        .await;

    if let Err(e) = result {
        let _ = event_tx.send(AcpEvent::Error {
            connection_id: connection_id.clone(),
            session_id: None,
            code: "connection_failed".into(),
            message: e.to_string(),
            recoverable: false,
        });
    }
    let _ = event_tx.send(AcpEvent::Disconnected {
        connection_id,
        reason: "connection loop ended".into(),
    });
}

async fn run_command_loop(
    connection_id: String,
    connection: ConnectionTo<Agent>,
    command_rx: &mut mpsc::UnboundedReceiver<ClientCommand>,
    event_tx: &mpsc::UnboundedSender<AcpEvent>,
    pending_permissions: std::sync::Arc<
        tokio::sync::Mutex<HashMap<String, agent_client_protocol::Responder<RequestPermissionResponse>>>,
    >,
) {
    let mut sessions: HashMap<String, mpsc::UnboundedSender<SessionCommand>> = HashMap::new();

    while let Some(cmd) = command_rx.recv().await {
        match cmd {
            ClientCommand::NewSession { cwd, mcp_servers, reply } => {
                // Runyard's IDE is an ACP *client* only - it never speaks MCP
                // itself. `mcp_servers` config just gets forwarded verbatim in
                // the wire request; the agent is the one that connects to
                // those servers. See protocol-research-reference.md.
                let request = agent_client_protocol::schema::v1::NewSessionRequest::new(&cwd)
                    .mcp_servers(parse_mcp_servers(mcp_servers));
                let builder = connection.build_session_from(request);
                match builder.block_task().start_session().await {
                    Ok(active_session) => {
                        let session_id = active_session.session_id().to_string();
                        let (session_tx, session_rx) = mpsc::unbounded_channel();
                        sessions.insert(session_id.clone(), session_tx);

                        let evt_tx = event_tx.clone();
                        let conn_id = connection_id.clone();
                        let sid = session_id.clone();
                        let _ = connection.spawn(async move {
                            run_session_worker(conn_id, sid, active_session, session_rx, evt_tx).await
                        });

                        let _ = event_tx.send(AcpEvent::SessionStarted {
                            connection_id: connection_id.clone(),
                            session_id: session_id.clone(),
                        });
                        let _ = reply.send(Ok(session_id));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(AcpClientError::Protocol(e.to_string())));
                    }
                }
            }
            ClientCommand::SendPrompt { session_id, text, reply } => {
                if let Some(tx) = sessions.get(&session_id) {
                    let _ = tx.send(SessionCommand::SendPrompt { text, reply });
                } else {
                    let _ = reply.send(Err(AcpClientError::SessionNotFound(session_id)));
                }
            }
            ClientCommand::Cancel { session_id, reply } => {
                if let Some(tx) = sessions.get(&session_id) {
                    let _ = tx.send(SessionCommand::Cancel { reply });
                } else {
                    let _ = reply.send(Err(AcpClientError::SessionNotFound(session_id)));
                }
            }
            ClientCommand::CloseSession { session_id, reply } => {
                if let Some(tx) = sessions.remove(&session_id) {
                    let _ = tx.send(SessionCommand::Close { reply });
                    let _ = event_tx.send(AcpEvent::SessionClosed {
                        connection_id: connection_id.clone(),
                        session_id,
                    });
                } else {
                    let _ = reply.send(Err(AcpClientError::SessionNotFound(session_id)));
                }
            }
            ClientCommand::RespondPermission { request_id, option_id, reply } => {
                let responder = pending_permissions.lock().await.remove(&request_id);
                match responder {
                    Some(responder) => {
                        let outcome = match option_id {
                            Some(id) => RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(id)),
                            None => RequestPermissionOutcome::Cancelled,
                        };
                        let _ = responder.respond(RequestPermissionResponse::new(outcome));
                        let _ = reply.send(Ok(()));
                    }
                    None => {
                        let _ = reply.send(Err(AcpClientError::Protocol(format!(
                            "no pending permission request with id {request_id}"
                        ))));
                    }
                }
            }
            ClientCommand::LoadSession { session_id, cwd, reply } => {
                let request = LoadSessionRequest::new(session_id.clone(), cwd);
                match connection.send_request(request).block_task().await {
                    Ok(response) => {
                        reattach_session(
                            &connection,
                            &connection_id,
                            session_id,
                            response.modes,
                            &mut sessions,
                            event_tx,
                            reply,
                        );
                    }
                    Err(e) => {
                        let _ = reply.send(Err(AcpClientError::Protocol(e.to_string())));
                    }
                }
            }
            ClientCommand::ResumeSession { session_id, cwd, reply } => {
                let request = ResumeSessionRequest::new(session_id.clone(), cwd);
                match connection.send_request(request).block_task().await {
                    Ok(response) => {
                        reattach_session(
                            &connection,
                            &connection_id,
                            session_id,
                            response.modes,
                            &mut sessions,
                            event_tx,
                            reply,
                        );
                    }
                    Err(e) => {
                        let _ = reply.send(Err(AcpClientError::Protocol(e.to_string())));
                    }
                }
            }
            ClientCommand::SetConfigOption { session_id, key, value, reply } => {
                match json_to_config_value(&value) {
                    Ok(config_value) => {
                        let request = SetSessionConfigOptionRequest::new(
                            session_id,
                            SessionConfigId::new(key),
                            config_value,
                        );
                        match connection.send_request(request).block_task().await {
                            Ok(_response) => {
                                let _ = reply.send(Ok(()));
                            }
                            Err(e) => {
                                let _ = reply.send(Err(AcpClientError::Protocol(e.to_string())));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            ClientCommand::SetMode { session_id, mode, reply } => {
                let request = SetSessionModeRequest::new(session_id, SessionModeId::new(mode));
                match connection.send_request(request).block_task().await {
                    Ok(_response) => {
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(AcpClientError::Protocol(e.to_string())));
                    }
                }
            }
            ClientCommand::Logout { reply } => {
                match connection.send_request(LogoutRequest::new()).block_task().await {
                    Ok(_response) => {
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(AcpClientError::Protocol(e.to_string())));
                    }
                }
            }
            ClientCommand::ListSessions { reply } => {
                let _ = reply.send(Ok(sessions.keys().cloned().collect()));
            }
            ClientCommand::Authenticate { method_id, reply } => {
                let request = AuthenticateRequest::new(AuthMethodId::new(method_id));
                match connection.send_request(request).block_task().await {
                    Ok(_response) => {
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(AcpClientError::AuthFailed(e.to_string())));
                    }
                }
            }
            ClientCommand::Shutdown { reply } => {
                let _ = reply.send(());
                break;
            }
        }
    }
}

async fn run_session_worker(
    connection_id: String,
    session_id: String,
    mut session: agent_client_protocol::ActiveSession<'static, Agent>,
    mut session_rx: mpsc::UnboundedReceiver<SessionCommand>,
    event_tx: mpsc::UnboundedSender<AcpEvent>,
) -> Result<(), agent_client_protocol::Error> {
    loop {
        tokio::select! {
            // Commands coming in from `RunyardAcpClient`'s public API.
            cmd = session_rx.recv() => {
                match cmd {
                    Some(SessionCommand::SendPrompt { text, reply }) => {
                        match session.send_prompt(text) {
                            Ok(()) => {
                                let _ = reply.send(Ok(()));
                            }
                            Err(e) => {
                                let _ = reply.send(Err(AcpClientError::Protocol(e.to_string())));
                            }
                        }
                    }
                    Some(SessionCommand::Cancel { reply }) => {
                        // Real `session/cancel` notification, per the protocol's
                        // turn-cancellation flow (client -> agent, best-effort,
                        // no response expected).
                        let result = session
                            .connection()
                            .send_notification(CancelNotification::new(session.session_id().clone()))
                            .map_err(|e| AcpClientError::Protocol(e.to_string()));
                        let _ = reply.send(result);
                    }
                    Some(SessionCommand::Close { reply }) => {
                        let _ = reply.send(Ok(()));
                        break;
                    }
                    None => break,
                }
            }
            // The `PromptResponse.stop_reason` for a prompt this session sent
            // arrives on the session's own update channel (it's the reply to
            // the `session/prompt` request, not a `session/update`
            // notification), so it has to be drained here to ever surface a
            // `PromptCompleted` event. Content chunks/tool calls/etc. are
            // already forwarded for real via the connection-level
            // `on_receive_notification` handler in `run_stdio_connection`, so
            // the `SessionMessage(Dispatch)` arm below is intentionally a
            // no-op - it would otherwise be a duplicate delivery path.
            update = session.read_update() => {
                match update {
                    Ok(agent_client_protocol::SessionMessage::StopReason(reason)) => {
                        let _ = event_tx.send(AcpEvent::PromptCompleted {
                            connection_id: connection_id.clone(),
                            session_id: session_id.clone(),
                            stop_reason: format!("{reason:?}"),
                        });
                    }
                    Ok(agent_client_protocol::SessionMessage::SessionMessage(_)) => {}
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
    }
    Ok(())
}

fn forward_session_notification(
    connection_id: &str,
    notification: &SessionNotification,
    event_tx: &mpsc::UnboundedSender<AcpEvent>,
) {
    let session_id = notification.session_id.to_string();
    let event = crate::updates::map_session_update(connection_id, &session_id, &notification.update);
    let _ = event_tx.send(event);
}

/// Registers a freshly loaded/resumed session exactly like `NewSession` does:
/// build a synthetic `NewSessionResponse` from what the agent gave back (load
/// and resume responses don't carry `session_id` on the wire since the client
/// already knows it - it's a request field, not a response field), attach a
/// session handler for it, and spawn its worker task.
fn reattach_session(
    connection: &ConnectionTo<Agent>,
    connection_id: &str,
    session_id: String,
    modes: Option<agent_client_protocol::schema::v1::SessionModeState>,
    sessions: &mut HashMap<String, mpsc::UnboundedSender<SessionCommand>>,
    event_tx: &mpsc::UnboundedSender<AcpEvent>,
    reply: oneshot::Sender<Result<(), AcpClientError>>,
) {
    let synthetic = NewSessionResponse::new(session_id.clone()).modes(modes);
    match connection.attach_session(synthetic, Vec::new()) {
        Ok(active_session) => {
            let (session_tx, session_rx) = mpsc::unbounded_channel();
            sessions.insert(session_id.clone(), session_tx);

            let evt_tx = event_tx.clone();
            let conn_id = connection_id.to_string();
            let sid = session_id.clone();
            let _ = connection.spawn(async move {
                run_session_worker(conn_id, sid, active_session, session_rx, evt_tx).await
            });

            let _ = event_tx.send(AcpEvent::SessionStarted {
                connection_id: connection_id.to_string(),
                session_id,
            });
            let _ = reply.send(Ok(()));
        }
        Err(e) => {
            let _ = reply.send(Err(AcpClientError::Protocol(e.to_string())));
        }
    }
}

/// Best-effort parse of Runyard's own JSON `mcp_servers` config array into
/// the wire `McpServer` schema type. Entries that don't parse are dropped
/// (logged) rather than failing the whole session - a typo in one server's
/// config shouldn't block a session the user didn't ask to configure MCP for.
fn parse_mcp_servers(values: Vec<serde_json::Value>) -> Vec<McpServer> {
    values
        .into_iter()
        .filter_map(|value| match serde_json::from_value::<McpServer>(value.clone()) {
            Ok(server) => Some(server),
            Err(e) => {
                tracing::warn!(error = %e, value = %value, "skipping malformed mcp_servers entry");
                None
            }
        })
        .collect()
}

/// Runyard's config-option command carries a plain `serde_json::Value`
/// (whatever the frontend sent); the wire type only accepts boolean or
/// string-id values, so map the common JSON shapes onto it and reject
/// anything else with a clear error instead of silently coercing.
fn json_to_config_value(value: &serde_json::Value) -> AcpResult<SessionConfigOptionValue> {
    match value {
        serde_json::Value::Bool(b) => Ok((*b).into()),
        serde_json::Value::String(s) => Ok(s.as_str().into()),
        other => Err(AcpClientError::Protocol(format!(
            "unsupported session config option value (expected boolean or string): {other}"
        ))),
    }
}
