use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};

use crate::client::RunyardAcpClient;
use crate::error::{AcpClientError, AcpResult};
use crate::events::AcpEvent;
use crate::transport::AgentTransportConfig;

/// Manages multiple concurrent agent connections with a global concurrency
/// limit. Each connection gets its own `RunyardAcpClient`; callers look
/// connections up by `connection_id`.
pub struct AcpConnectionPool {
    max_concurrent: usize,
    connections: Arc<RwLock<HashMap<String, RunyardAcpClient>>>,
    event_tx: mpsc::UnboundedSender<AcpEvent>,
}

impl AcpConnectionPool {
    pub fn new(max_concurrent: usize, event_tx: mpsc::UnboundedSender<AcpEvent>) -> Self {
        Self {
            max_concurrent,
            connections: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
    }

    pub async fn connect(&self, transport: AgentTransportConfig) -> AcpResult<String> {
        {
            let conns = self.connections.read().await;
            if conns.len() >= self.max_concurrent {
                return Err(AcpClientError::PoolLimitReached(self.max_concurrent));
            }
        }
        let client = RunyardAcpClient::connect(transport, self.event_tx.clone()).await?;
        let id = client.connection_id.clone();
        self.connections.write().await.insert(id.clone(), client);
        Ok(id)
    }

    pub async fn disconnect(&self, connection_id: &str) -> AcpResult<()> {
        let client = self.connections.write().await.remove(connection_id);
        match client {
            Some(client) => {
                client.shutdown().await;
                Ok(())
            }
            None => Err(AcpClientError::ConnectionClosed),
        }
    }

    pub async fn active_connection_ids(&self) -> Vec<String> {
        self.connections.read().await.keys().cloned().collect()
    }

    pub async fn with_client<F, R>(&self, connection_id: &str, f: F) -> AcpResult<R>
    where
        F: for<'a> FnOnce(&'a RunyardAcpClient) -> std::pin::Pin<Box<dyn std::future::Future<Output = AcpResult<R>> + 'a>>,
    {
        let conns = self.connections.read().await;
        let client = conns
            .get(connection_id)
            .ok_or(AcpClientError::ConnectionClosed)?;
        f(client).await
    }

    pub async fn count(&self) -> usize {
        self.connections.read().await.len()
    }
}
