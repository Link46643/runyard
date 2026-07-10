//! Exercises `AcpConnectionPool` (1.7.14) against real spawned mock-agent
//! processes: concurrency limit enforcement and resource cleanup on
//! disconnect. Complements `integration_test.rs`, which covers a single
//! `RunyardAcpClient` end to end.

use runyard_acp::{AcpConnectionPool, AgentTransportConfig};
use tokio::sync::mpsc;

#[tokio::test]
async fn pool_enforces_max_concurrent_and_cleans_up_on_disconnect() {
    let bin_path = env!("CARGO_BIN_EXE_mock-agent-bin");
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let pool = AcpConnectionPool::new(2, event_tx);

    assert_eq!(pool.count().await, 0);

    let first = pool
        .connect(AgentTransportConfig::stdio(bin_path))
        .await
        .expect("first connect should succeed");
    let second = pool
        .connect(AgentTransportConfig::stdio(bin_path))
        .await
        .expect("second connect should succeed (at the limit)");
    assert_eq!(pool.count().await, 2);

    // Third connection should be rejected - pool is at its max_concurrent(2).
    let third = pool.connect(AgentTransportConfig::stdio(bin_path)).await;
    assert!(third.is_err(), "expected pool limit error, got {third:?}");
    assert_eq!(pool.count().await, 2, "rejected connect must not affect pool size");

    let mut ids = pool.active_connection_ids().await;
    ids.sort();
    let mut expected = vec![first.clone(), second.clone()];
    expected.sort();
    assert_eq!(ids, expected);

    // Disconnecting frees a slot for a new connection.
    pool.disconnect(&first).await.expect("disconnect should succeed");
    assert_eq!(pool.count().await, 1);

    let fourth = pool
        .connect(AgentTransportConfig::stdio(bin_path))
        .await
        .expect("connect after freeing a slot should succeed");
    assert_eq!(pool.count().await, 2);

    // Clean up.
    pool.disconnect(&second).await.expect("disconnect should succeed");
    pool.disconnect(&fourth).await.expect("disconnect should succeed");
    assert_eq!(pool.count().await, 0);
}

#[tokio::test]
async fn pool_disconnect_of_unknown_connection_errors_cleanly() {
    let (event_tx, _event_rx) = mpsc::unbounded_channel();
    let pool = AcpConnectionPool::new(4, event_tx);

    let result = pool.disconnect("does-not-exist").await;
    assert!(result.is_err(), "expected an error disconnecting an unknown connection id");
}
