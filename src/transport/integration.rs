// Integration of QUIC media transport with iroh
//
// This module implements the integration between the SFU and iroh for media transport.

use std::{
    collections::HashMap,
    sync::Arc,
};

use anyhow::Result;
use iroh::{
    endpoint::{Endpoint, Connection},
    NodeId,
};
use tokio::sync::RwLock;

use crate::{
    connection::RtcConnection,
    media::{
        codec::{CodecType, CodecFactory},
        frame::MediaFrame,
    },
    transport::{
        quic::{QuicMediaTrack, QuicMediaSender, QuicMediaReceiver},
    },
    SfuError,
};

/// Media stream direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamDirection {
    /// Sending only
    SendOnly,
    /// Receiving only
    RecvOnly,
    /// Sending and receiving
    SendRecv,
    /// Inactive
    Inactive,
}

/// Media stream configuration
pub struct MediaStreamConfig {
    /// Stream direction
    pub direction: StreamDirection,
    /// Codec type
    pub codec_type: CodecType,
    /// Payload type
    pub payload_type: u8,
    /// SSRC identifier
    pub ssrc: u32,
    /// Maximum bitrate
    pub max_bitrate: u32,
}

/// Media stream over QUIC
pub struct QuicMediaStream {
    /// Stream identifier
    stream_id: String,
    /// Media track
    track: Arc<QuicMediaTrack>,
    /// Media sender (if sending)
    sender: Option<QuicMediaSender>,
    /// Media receiver (if receiving)
    receiver: Option<QuicMediaReceiver>,
    /// Stream configuration
    config: MediaStreamConfig,
}

impl QuicMediaStream {
    /// Create a new media stream
    pub fn new(
        stream_id: String,
        track_id: u64,
        config: MediaStreamConfig,
    ) -> Self {
        // Create media track
        let track = Arc::new(QuicMediaTrack::new(
            track_id,
            config.codec_type,
            config.payload_type,
            config.ssrc,
        ));
        
        // Create sender and receiver based on direction
        let sender = match config.direction {
            StreamDirection::SendOnly | StreamDirection::SendRecv => {
                Some(QuicMediaSender::new(track.clone(), 30))
            }
            _ => None,
        };
        
        let receiver = match config.direction {
            StreamDirection::RecvOnly | StreamDirection::SendRecv => {
                Some(QuicMediaReceiver::new(track.clone(), 30))
            }
            _ => None,
        };
        
        Self {
            stream_id,
            track,
            sender,
            receiver,
            config,
        }
    }
    
    /// Send a media frame
    pub async fn send_frame(&self, frame: MediaFrame) -> Result<()> {
        if let Some(sender) = &self.sender {
            sender.send_frame(frame).await
        } else {
            Err(SfuError::Transport("Stream is not configured for sending".to_string()).into())
        }
    }
    
    /// Receive a media frame
    pub async fn receive_frame(&mut self) -> Result<Option<MediaFrame>> {
        if let Some(receiver) = &mut self.receiver {
            Ok(receiver.receive_frame().await)
        } else {
            Err(SfuError::Transport("Stream is not configured for receiving".to_string()).into())
        }
    }
    
    /// Get the stream identifier
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }
    
    /// Get the media track
    pub fn track(&self) -> &Arc<QuicMediaTrack> {
        &self.track
    }
    
    /// Get the stream configuration
    pub fn config(&self) -> &MediaStreamConfig {
        &self.config
    }
    
    /// Stop the stream
    pub async fn stop(&self) {
        if let Some(sender) = &self.sender {
            sender.stop().await;
        }
        
        if let Some(receiver) = &self.receiver {
            receiver.stop().await;
        }
    }
}

/// QUIC media session
pub struct QuicMediaSession {
    /// Session identifier
    session_id: String,
    /// RTC connection
    connection: RtcConnection,
    /// Media streams
    streams: HashMap<String, QuicMediaStream>,
}

impl QuicMediaSession {
    /// Create a new media session
    pub fn new(session_id: String, connection: RtcConnection) -> Self {
        Self {
            session_id,
            connection,
            streams: HashMap::new(),
        }
    }
    
    /// Create a new media stream
    pub fn create_stream(
        &mut self,
        stream_id: String,
        track_id: u64,
        config: MediaStreamConfig,
    ) -> Result<&QuicMediaStream> {
        if self.streams.contains_key(&stream_id) {
            return Err(SfuError::Transport(format!("Stream already exists: {}", stream_id)).into());
        }
        
        let stream = QuicMediaStream::new(stream_id.clone(), track_id, config);
        self.streams.insert(stream_id.clone(), stream);
        
        Ok(self.streams.get(&stream_id).unwrap())
    }
    
    /// Get a media stream
    pub fn get_stream(&self, stream_id: &str) -> Option<&QuicMediaStream> {
        self.streams.get(stream_id)
    }
    
    /// Get a mutable media stream
    pub fn get_stream_mut(&mut self, stream_id: &str) -> Option<&mut QuicMediaStream> {
        self.streams.get_mut(stream_id)
    }
    
    /// Remove a media stream
    pub fn remove_stream(&mut self, stream_id: &str) -> Result<()> {
        if let Some(stream) = self.streams.remove(stream_id) {
            // Stop the stream
            tokio::spawn(async move {
                stream.stop().await;
            });
            Ok(())
        } else {
            Err(SfuError::Transport(format!("Stream not found: {}", stream_id)).into())
        }
    }
    
    /// Get the session identifier
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    
    /// Get the RTC connection
    pub fn connection(&self) -> &RtcConnection {
        &self.connection
    }
    
    /// Get all stream identifiers
    pub fn stream_ids(&self) -> Vec<String> {
        self.streams.keys().cloned().collect()
    }
    
    /// Stop all streams
    pub async fn stop_all_streams(&self) {
        for stream in self.streams.values() {
            stream.stop().await;
        }
    }
}

/// QUIC media manager
pub struct QuicMediaManager {
    /// Iroh endpoint
    endpoint: Endpoint,
    /// Active sessions
    sessions: Arc<RwLock<HashMap<String, QuicMediaSession>>>,
}

impl QuicMediaManager {
    /// Create a new QUIC media manager
    pub fn new(endpoint: Endpoint) -> Self {
        Self {
            endpoint,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create a new media session
    pub async fn create_session(&self, node_id: NodeId, connection: Connection) -> Result<String> {
        let session_id = node_id.to_string();
        let rtc_connection = RtcConnection::new(connection, node_id);
        
        let session = QuicMediaSession::new(session_id.clone(), rtc_connection);
        
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);
        
        Ok(session_id)
    }
    
    /// Get a media session
    pub async fn get_session(&self, session_id: &str) -> Result<QuicMediaSession> {
        let sessions = self.sessions.read().await;
        
        if let Some(session) = sessions.get(session_id) {
            Ok(session.clone())
        } else {
            Err(SfuError::Session(format!("Session not found: {}", session_id)).into())
        }
    }
    
    /// Remove a media session
    pub async fn remove_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.remove(session_id) {
            // Stop all streams
            session.stop_all_streams().await;
            Ok(())
        } else {
            Err(SfuError::Session(format!("Session not found: {}", session_id)).into())
        }
    }
    
    /// Create a media stream in a session
    pub async fn create_stream(
        &self,
        session_id: &str,
        stream_id: String,
        track_id: u64,
        config: MediaStreamConfig,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(session_id) {
            session.create_stream(stream_id, track_id, config)?;
            Ok(())
        } else {
            Err(SfuError::Session(format!("Session not found: {}", session_id)).into())
        }
    }
    
    /// Send a media frame to a stream
    pub async fn send_frame(
        &self,
        session_id: &str,
        stream_id: &str,
        frame: MediaFrame,
    ) -> Result<()> {
        let sessions = self.sessions.read().await;
        
        if let Some(session) = sessions.get(session_id) {
            if let Some(stream) = session.get_stream(stream_id) {
                stream.send_frame(frame).await
            } else {
                Err(SfuError::Transport(format!("Stream not found: {}", stream_id)).into())
            }
        } else {
            Err(SfuError::Session(format!("Session not found: {}", session_id)).into())
        }
    }
    
    /// Get the iroh endpoint
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
}
