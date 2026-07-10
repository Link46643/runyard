//! Tiny binary wrapper so integration tests can spawn the mock agent as a
//! real subprocess over real stdio, exercising the exact same code path
//! `RunyardAcpClient`'s stdio transport uses in production (`AcpAgent::from_str`
//! spawning a command). Only built when the `test-utils` feature is enabled.

#[tokio::main]
async fn main() -> Result<(), agent_client_protocol::Error> {
    runyard_acp::mock_agent::run_mock_agent(agent_client_protocol::Stdio::new()).await
}
