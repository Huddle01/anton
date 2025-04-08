// Connection management module for the SFU
//
// This module handles QUIC connections between participants and the SFU.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use iroh::{endpoint::Connection, NodeId};

use crate::SfuError;

/// RTC connection for media transport
pub struct RtcConnection {
    /// Iroh connection
    connection: Connection,
    /// Remote node ID
    remote_node_id: NodeId,
}

impl RtcConnection {
    /// Create a new RTC connection
    pub fn new(connection: Connection, remote_node_id: NodeId) -> Self {
        Self {
            connection,
            remote_node_id,
        }
    }

    /// Get the underlying Iroh connection
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Get the remote node ID
    pub fn remote_node_id(&self) -> &NodeId {
        &self.remote_node_id
    }
}

/// Connection manager trait
#[async_trait]
pub trait ConnectionManager: Send + Sync {
    /// Connect to a remote node
    async fn connect(&self, node_addr: iroh::NodeAddr) -> Result<RtcConnection>;

    /// Accept an incoming connection
    async fn accept(&self) -> Result<Option<RtcConnection>>;

    /// Close a connection
    async fn close(&self, connection: &RtcConnection, reason: &str) -> Result<()>;
}

/// Default implementation of the connection manager
pub struct DefaultConnectionManager {
    /// Iroh endpoint
    endpoint: iroh::endpoint::Endpoint,
}

impl DefaultConnectionManager {
    /// Create a new connection manager
    pub fn new(endpoint: iroh::endpoint::Endpoint) -> Self {
        Self { endpoint }
    }

    /// Get the underlying Iroh endpoint
    pub fn endpoint(&self) -> &iroh::endpoint::Endpoint {
        &self.endpoint
    }
}

#[async_trait]
impl ConnectionManager for DefaultConnectionManager {
    async fn connect(&self, node_addr: iroh::NodeAddr) -> Result<RtcConnection> {
        // Connect to the remote node
        let connection = self.endpoint.connect(node_addr).await?;
        
        // Get the remote node ID
        let remote_node_id = connection.remote_node_id().await?;
        
        // Create RTC connection
        let rtc_connection = RtcConnection::new(connection, remote_node_id);
        
        Ok(rtc_connection)
    }

    async fn accept(&self) -> Result<Option<RtcConnection>> {
        // Accept an incoming connection
        if let Some(connection) = self.endpoint.accept().await? {
            // Get the remote node ID
            let remote_node_id = connection.remote_node_id().await?;
            
            // Create RTC connection
            let rtc_connection = RtcConnection::new(connection, remote_node_id);
            
            Ok(Some(rtc_connection))
        } else {
            Ok(None)
        }
    }

    async fn close(&self, connection: &RtcConnection, reason: &str) -> Result<()> {
        // Close the connection
        connection.connection().close(0u32.into(), reason.as_bytes());
        
        Ok(())
    }
}
