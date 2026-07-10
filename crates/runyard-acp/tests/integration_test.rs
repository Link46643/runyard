//! End-to-end test of `RunyardAcpClient` against a real ACP agent process
//! (the mock agent binary, spawned exactly the way a real agent like Claude
//! Code or Gemini CLI would be - via stdio subprocess spawning). No actual
//! third-party agent binary is available in this build environment, so the
//! mock agent stands in for one; everything downstream of "spawn a process
//! that speaks ACP over stdio" is exercised for real.

use runyard_acp::{AcpEvent, AgentTransportConfig, RunyardAcpClient};
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

/// Waits for the next event, transparently skipping `LogLine` events - those
/// now fire for every raw line sent/received over stdio (real wire traffic,
/// see the dedicated `log_lines_are_captured_from_real_process_io` test
/// below), which would otherwise interleave unpredictably with the specific
/// protocol-event sequences the other tests assert on.
async fn next_event(rx: &mut mpsc::UnboundedReceiver<AcpEvent>) -> AcpEvent {
    loop {
        let event = timeout(Duration::from_secs(10), rx.recv())
            .await
            .expect("timed out waiting for event")
            .expect("event channel closed unexpectedly");
        if !matches!(event, AcpEvent::LogLine { .. }) {
            return event;
        }
    }
}

#[tokio::test]
async fn full_session_lifecycle_against_real_agent_process() {
    let bin_path = env!("CARGO_BIN_EXE_mock-agent-bin");
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    let client = RunyardAcpClient::connect(AgentTransportConfig::stdio(bin_path), event_tx)
        .await
        .expect("failed to connect to mock agent process");

    // 1. Connection + initialize handshake.
    let connected = next_event(&mut event_rx).await;
    assert!(matches!(connected, AcpEvent::StatusChanged { .. }), "expected StatusChanged(Initializing) first, got {connected:?}");
    let connected = next_event(&mut event_rx).await;
    assert!(matches!(connected, AcpEvent::Connected { .. }), "expected Connected, got {connected:?}");
    let ready = next_event(&mut event_rx).await;
    assert!(matches!(ready, AcpEvent::StatusChanged { .. }), "expected StatusChanged(Ready), got {ready:?}");

    // 2. Create a session.
    let session_id = client
        .new_session(".".to_string(), vec![])
        .await
        .expect("new_session failed");
    assert!(!session_id.is_empty());

    let started = next_event(&mut event_rx).await;
    match started {
        AcpEvent::SessionStarted { session_id: sid, .. } => assert_eq!(sid, session_id),
        other => panic!("expected SessionStarted, got {other:?}"),
    }

    // 3. Send a prompt and observe the real notification the mock agent streams back.
    client
        .send_prompt(session_id.clone(), "hello, agent!".to_string())
        .await
        .expect("send_prompt failed");

    let chunk = next_event(&mut event_rx).await;
    match chunk {
        AcpEvent::AgentMessageChunk { text, session_id: sid, .. } => {
            assert_eq!(sid, session_id);
            assert_eq!(text, "mock agent response");
        }
        other => panic!("expected AgentMessageChunk, got {other:?}"),
    }

    // The mock agent's `PromptResponse` (stop_reason) arrives on the
    // session's own update channel, separate from the notification stream
    // above - `run_session_worker` drains it and surfaces it as its own event.
    let completed = next_event(&mut event_rx).await;
    match completed {
        AcpEvent::PromptCompleted { session_id: sid, stop_reason, .. } => {
            assert_eq!(sid, session_id);
            assert_eq!(stop_reason, "EndTurn");
        }
        other => panic!("expected PromptCompleted, got {other:?}"),
    }

    // 4. Close the session and shut down cleanly.
    client.close_session(session_id.clone()).await.expect("close_session failed");
    let closed = next_event(&mut event_rx).await;
    assert!(matches!(closed, AcpEvent::SessionClosed { .. }), "expected SessionClosed, got {closed:?}");

    client.shutdown().await;
}

#[tokio::test]
async fn session_reattach_mode_cancel_and_logout_round_trip() {
    let bin_path = env!("CARGO_BIN_EXE_mock-agent-bin");
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    let client = RunyardAcpClient::connect(AgentTransportConfig::stdio(bin_path), event_tx)
        .await
        .expect("failed to connect to mock agent process");

    // Drain connect/initialize events.
    let _ = next_event(&mut event_rx).await;
    let _ = next_event(&mut event_rx).await;
    let _ = next_event(&mut event_rx).await;

    let session_id = client
        .new_session(".".to_string(), vec![])
        .await
        .expect("new_session failed");
    let _ = next_event(&mut event_rx).await; // SessionStarted

    // Re-attach via load_session (real `session/load` request/response round
    // trip against the mock agent, then a fresh local ActiveSession handler
    // wired up exactly like a brand new session).
    client
        .load_session(session_id.clone(), ".".to_string())
        .await
        .expect("load_session failed");
    let reattached = next_event(&mut event_rx).await;
    match reattached {
        AcpEvent::SessionStarted { session_id: sid, .. } => assert_eq!(sid, session_id),
        other => panic!("expected SessionStarted from load_session re-attach, got {other:?}"),
    }

    // Switch mode - the mock agent replies AND emits a notification so both
    // halves of the real wire flow get exercised.
    client
        .set_mode(session_id.clone(), "code".to_string())
        .await
        .expect("set_mode failed");
    let mode_chunk = next_event(&mut event_rx).await;
    match mode_chunk {
        AcpEvent::AgentMessageChunk { text, .. } => assert_eq!(text, "mode set to code"),
        other => panic!("expected AgentMessageChunk from set_mode, got {other:?}"),
    }

    // Config option round trip (boolean value).
    client
        .set_config_option(session_id.clone(), "auto_approve".to_string(), serde_json::Value::Bool(true))
        .await
        .expect("set_config_option failed");

    // Cancel - real `session/cancel` notification; mock agent echoes it back
    // as a content chunk so the send path is verifiably real, not just
    // "didn't error".
    client.cancel(session_id.clone()).await.expect("cancel failed");
    let cancel_chunk = next_event(&mut event_rx).await;
    match cancel_chunk {
        AcpEvent::AgentMessageChunk { text, .. } => assert_eq!(text, "cancelled"),
        other => panic!("expected AgentMessageChunk from cancel echo, got {other:?}"),
    }

    // list_sessions should reflect the one live session.
    let sessions = client.list_sessions().await.expect("list_sessions failed");
    assert_eq!(sessions, vec![session_id.clone()]);

    // Logout is connection-level, not session-level.
    client.logout().await.expect("logout failed");

    client.close_session(session_id.clone()).await.expect("close_session failed");
    let closed = next_event(&mut event_rx).await;
    assert!(matches!(closed, AcpEvent::SessionClosed { .. }), "expected SessionClosed, got {closed:?}");

    client.shutdown().await;
}

#[tokio::test]
async fn log_lines_are_captured_from_real_process_io() {
    let bin_path = env!("CARGO_BIN_EXE_mock-agent-bin");
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();

    let client = RunyardAcpClient::connect(AgentTransportConfig::stdio(bin_path), event_tx)
        .await
        .expect("failed to connect to mock agent process");

    // The initialize handshake alone sends one JSON-RPC line out (stdin, the
    // request) and one back (stdout, the response) - real wire bytes, not
    // synthesized. Drain events until at least one of each is observed,
    // failing fast if none show up before the timeout.
    let mut saw_stdin = false;
    let mut saw_stdout = false;
    for _ in 0..50 {
        let event = timeout(Duration::from_secs(10), event_rx.recv())
            .await
            .expect("timed out waiting for a log line")
            .expect("event channel closed unexpectedly");
        match event {
            AcpEvent::LogLine { direction: runyard_acp::LogDirection::Stdin, ref line, .. } => {
                assert!(line.contains("initialize"), "expected the initialize request on stdin, got: {line}");
                saw_stdin = true;
            }
            AcpEvent::LogLine { direction: runyard_acp::LogDirection::Stdout, .. } => {
                saw_stdout = true;
            }
            _ => {}
        }
        if saw_stdin && saw_stdout {
            break;
        }
    }
    assert!(saw_stdin, "never observed a stdin log line");
    assert!(saw_stdout, "never observed a stdout log line");

    client.shutdown().await;
}

#[tokio::test]
async fn unknown_session_operations_error_cleanly() {
    let bin_path = env!("CARGO_BIN_EXE_mock-agent-bin");
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let client = RunyardAcpClient::connect(AgentTransportConfig::stdio(bin_path), event_tx)
        .await
        .expect("failed to connect");

    // Drain the connect/initialize events before proceeding.
    let _ = next_event(&mut event_rx).await;
    let _ = next_event(&mut event_rx).await;
    let _ = next_event(&mut event_rx).await;

    let result = client.send_prompt("does-not-exist".to_string(), "hi".to_string()).await;
    assert!(result.is_err(), "expected an error for an unknown session id");

    client.shutdown().await;
}
