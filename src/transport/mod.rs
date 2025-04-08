// Transport module for the SFU
//
// This module handles media transport over QUIC using iroh-roq.

pub mod quic;
pub mod integration;
pub mod adaptation;

use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

use anyhow::Result;
use async_trait::async_trait;
use iroh_roq::{Session, SendFlow, ReceiveFlow};

use crate::{
    connection::RtcConnection,
    media::{MediaTrack, TrackId},
    SfuError,
};

/// Media transport trait
#[async_trait]
pub trait MediaTransport: Send + Sync {
    /// Create a new transport session
    async fn create_session(&self, connection: &RtcConnection) -> Result<TransportSession>;
    
    /// Send a media track
    async fn send_track(&self, session: &TransportSession, track: MediaTrack) -> Result<()>;
    
    /// Receive a media track
    async fn receive_track(&self, session: &TransportSession) -> Result<Option<MediaTrack>>;
}

/// Transport session for media over QUIC
pub struct TransportSession {
    /// RTC connection
    connection: RtcConnection,
    /// RoQ session
    session: Session,
    /// Next flow identifier for receiving
    next_recv_flow_id: Arc<AtomicU32>,
    /// Next flow identifier for sending
    next_send_flow_id: Arc<AtomicU32>,
}

impl TransportSession {
    /// Create a new transport session
    pub fn new(connection: RtcConnection) -> Self {
        let session = Session::new(connection.connection().clone());
        
        Self {
            connection,
            session,
            next_recv_flow_id: Arc::new(AtomicU32::new(0)),
            next_send_flow_id: Arc::new(AtomicU32::new(0)),
        }
    }
    
    /// Get the underlying RTC connection
    pub fn connection(&self) -> &RtcConnection {
        &self.connection
    }
    
    /// Get the underlying RoQ session
    pub fn session(&self) -> &Session {
        &self.session
    }
    
    /// Create a new send flow
    pub async fn new_send_flow(&self) -> Result<SendFlow> {
        let flow_id = self.next_send_flow_id.fetch_add(1, Ordering::SeqCst);
        let send_flow = self.session.new_send_flow(flow_id.into()).await?;
        Ok(send_flow)
    }
    
    /// Create a new receive flow
    pub async fn new_receive_flow(&self) -> Result<ReceiveFlow> {
        let flow_id = self.next_recv_flow_id.fetch_add(1, Ordering::SeqCst);
        let recv_flow = self.session.new_receive_flow(flow_id.into()).await?;
        Ok(recv_flow)
    }
}

/// Default implementation of the media transport
pub struct DefaultMediaTransport;

impl DefaultMediaTransport {
    /// Create a new media transport
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MediaTransport for DefaultMediaTransport {
    async fn create_session(&self, connection: &RtcConnection) -> Result<TransportSession> {
        let session = TransportSession::new(connection.clone());
        Ok(session)
    }
    
    async fn send_track(&self, session: &TransportSession, track: MediaTrack) -> Result<()> {
        // Create a new send flow
        let _send_flow = session.new_send_flow().await?;
        
        // Implementation details for sending a track will be added later
        // This would involve setting up RTP packetization and sending media frames
        
        Ok(())
    }
    
    async fn receive_track(&self, session: &TransportSession) -> Result<Option<MediaTrack>> {
        // Create a new receive flow
        let _recv_flow = session.new_receive_flow().await?;
        
        // Implementation details for receiving a track will be added later
        // This would involve setting up RTP depacketization and receiving media frames
        
        // Placeholder implementation
        Ok(None)
    }
}

// Default implementation
impl Default for DefaultMediaTransport {
    fn default() -> Self {
        Self::new()
    }
}
