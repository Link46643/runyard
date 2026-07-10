//! A hand-written ACP agent that speaks the real protocol, used only for
//! testing `RunyardAcpClient` against real ACP wire messages. No actual
//! agent binary (Claude Code, Gemini CLI, etc.) is available in the build
//! environment this crate was developed in, so this stands in for one in
//! integration tests - it is not a mock of Runyard's own logic, it's a real,
//! minimal, protocol-correct ACP agent implementation.

use agent_client_protocol::schema::v1::{
    AgentCapabilities, CancelNotification, ContentBlock, ContentChunk, InitializeRequest,
    InitializeResponse, LoadSessionRequest, LoadSessionResponse, LogoutRequest, LogoutResponse,
    NewSessionRequest, NewSessionResponse, PromptRequest, PromptResponse, SessionId,
    SessionNotification, SessionUpdate, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse, SetSessionModeRequest, SetSessionModeResponse, StopReason,
    TextContent,
};
use agent_client_protocol::{Agent, Client, ConnectTo, ConnectionTo, Dispatch};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Runs a minimal ACP agent over the given transport until the connection
/// closes. Responds to initialize/new-session/prompt/load-session/set-mode/
/// set-config-option/logout, streams one content chunk notification before
/// completing each prompt, and echoes cancel notifications back as a content
/// chunk - all so integration tests can exercise the real wire protocol on
/// both the request/response and notification paths without a third-party
/// agent binary (none is available in this build environment).
pub async fn run_mock_agent(transport: impl ConnectTo<Agent> + 'static) -> Result<(), agent_client_protocol::Error> {
    let session_counter = Arc::new(AtomicUsize::new(0));

    Agent
        .builder()
        .name("runyard-mock-agent")
        .on_receive_request(
            async move |request: InitializeRequest, responder, _connection| {
                responder.respond(
                    InitializeResponse::new(request.protocol_version)
                        .agent_capabilities(AgentCapabilities::new()),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let session_counter = session_counter.clone();
                async move |_request: NewSessionRequest, responder, _connection| {
                    let n = session_counter.fetch_add(1, Ordering::SeqCst);
                    responder.respond(NewSessionResponse::new(SessionId::new(format!("mock-session-{n}"))))
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: PromptRequest, responder, connection: ConnectionTo<Client>| {
                let _ = connection.send_notification(SessionNotification::new(
                    request.session_id.clone(),
                    SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
                        "mock agent response",
                    )))),
                ));
                responder.respond(PromptResponse::new(StopReason::EndTurn))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: LoadSessionRequest, responder, _connection| {
                responder.respond(LoadSessionResponse::new())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: SetSessionModeRequest, responder, connection: ConnectionTo<Client>| {
                // Real agents notify the client of the mode change too;
                // exercised here so the client-side notification path for
                // this case is covered by a real wire round-trip, not just
                // the request/response half.
                let _ = connection.send_notification(SessionNotification::new(
                    request.session_id.clone(),
                    SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
                        format!("mode set to {}", request.mode_id),
                    )))),
                ));
                responder.respond(SetSessionModeResponse::new())
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: SetSessionConfigOptionRequest, responder, _connection| {
                responder.respond(SetSessionConfigOptionResponse::new(vec![]))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: LogoutRequest, responder, _connection| responder.respond(LogoutResponse::new()),
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_notification(
            async move |notification: CancelNotification, connection: ConnectionTo<Client>| {
                let _ = connection.send_notification(SessionNotification::new(
                    notification.session_id.clone(),
                    SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(TextContent::new(
                        "cancelled",
                    )))),
                ));
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_dispatch(
            async move |message: Dispatch, connection: ConnectionTo<Client>| {
                message.respond_with_error(agent_client_protocol::util::internal_error("unhandled message"), connection)
            },
            agent_client_protocol::on_receive_dispatch!(),
        )
        .connect_to(transport)
        .await
}
