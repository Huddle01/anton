// QUIC media transport implementation using iroh
//
// This module implements media transport over QUIC using iroh-roq.

use std::{
    collections::HashMap,
    sync::{Arc, atomic::{AtomicU64, Ordering}},
};

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use iroh_roq::{Session, SendFlow, ReceiveFlow};
use tokio::sync::{mpsc, RwLock};

use crate::{
    connection::RtcConnection,
    media::{
        frame::MediaFrame,
        rtp::{RtpPacket, RtpPacketizer, RtpDepacketizer},
        codec::{Codec, CodecType},
        TrackId,
    },
    SfuError,
};

/// Media track transport over QUIC
pub struct QuicMediaTrack {
    /// Track identifier
    track_id: TrackId,
    /// Codec type
    codec_type: CodecType,
    /// RTP packetizer
    packetizer: RtpPacketizer,
    /// RTP depacketizer
    depacketizer: RtpDepacketizer,
    /// Send flow for outgoing media
    send_flow: Option<SendFlow>,
    /// Receive flow for incoming media
    receive_flow: Option<ReceiveFlow>,
    /// Frame sequence number
    frame_seq: AtomicU64,
}

impl QuicMediaTrack {
    /// Create a new QUIC media track
    pub fn new(track_id: TrackId, codec_type: CodecType, payload_type: u8, ssrc: u32) -> Self {
        Self {
            track_id,
            codec_type,
            packetizer: RtpPacketizer::new(codec_type, payload_type, ssrc),
            depacketizer: RtpDepacketizer::new(codec_type),
            send_flow: None,
            receive_flow: None,
            frame_seq: AtomicU64::new(0),
        }
    }
    
    /// Set the send flow
    pub fn set_send_flow(&mut self, flow: SendFlow) {
        self.send_flow = Some(flow);
    }
    
    /// Set the receive flow
    pub fn set_receive_flow(&mut self, flow: ReceiveFlow) {
        self.receive_flow = Some(flow);
    }
    
    /// Send a media frame
    pub async fn send_frame(&self, frame: &MediaFrame) -> Result<()> {
        if let Some(send_flow) = &self.send_flow {
            // Get timestamp from frame
            let timestamp = frame.timestamp;
            
            // Packetize the frame
            let packets = self.packetizer.packetize(&frame.data, timestamp)?;
            
            // Send each packet
            for packet in packets {
                let packet_data = packet.serialize();
                send_flow.send(packet_data).await?;
            }
            
            // Increment frame sequence
            self.frame_seq.fetch_add(1, Ordering::SeqCst);
            
            Ok(())
        } else {
            Err(SfuError::Transport("No send flow available".to_string()).into())
        }
    }
    
    /// Receive a media frame
    pub async fn receive_frame(&self) -> Result<Option<MediaFrame>> {
        if let Some(receive_flow) = &self.receive_flow {
            // Try to receive a packet
            if let Some(packet_data) = receive_flow.receive().await? {
                // Parse RTP packet
                let packet = RtpPacket::parse(&packet_data)?;
                
                // Process packet with depacketizer
                if let Some(frame_data) = self.depacketizer.process_packet(packet)? {
                    // Create media frame
                    let frame = match self.codec_type {
                        CodecType::Opus => {
                            MediaFrame::new_audio(
                                self.codec_type,
                                Bytes::from(frame_data),
                                packet.header.timestamp,
                                std::time::Duration::from_millis(20), // Typical Opus frame duration
                            )?
                        }
                        CodecType::VP9 => {
                            // Determine if this is a key frame (simplified)
                            let is_key_frame = frame_data.len() > 0 && (frame_data[0] & 0x01) == 0;
                            
                            if is_key_frame {
                                MediaFrame::new_video_key(
                                    self.codec_type,
                                    Bytes::from(frame_data),
                                    packet.header.timestamp,
                                    std::time::Duration::from_millis(33), // ~30fps
                                    None, // Spatial layer would be extracted from VP9 payload
                                    None, // Temporal layer would be extracted from VP9 payload
                                )?
                            } else {
                                MediaFrame::new_video_delta(
                                    self.codec_type,
                                    Bytes::from(frame_data),
                                    packet.header.timestamp,
                                    std::time::Duration::from_millis(33), // ~30fps
                                    None, // Spatial layer would be extracted from VP9 payload
                                    None, // Temporal layer would be extracted from VP9 payload
                                )?
                            }
                        }
                        _ => {
                            return Err(SfuError::Media(format!("Unsupported codec: {:?}", self.codec_type)).into());
                        }
                    };
                    
                    return Ok(Some(frame));
                }
            }
            
            // No complete frame available yet
            Ok(None)
        } else {
            Err(SfuError::Transport("No receive flow available".to_string()).into())
        }
    }
    
    /// Get the track identifier
    pub fn track_id(&self) -> TrackId {
        self.track_id
    }
    
    /// Get the codec type
    pub fn codec_type(&self) -> CodecType {
        self.codec_type
    }
}

/// QUIC media transport implementation
pub struct QuicMediaTransport {
    /// Active transport sessions
    sessions: Arc<RwLock<HashMap<String, Arc<TransportSession>>>>,
}

impl QuicMediaTransport {
    /// Create a new QUIC media transport
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl crate::transport::MediaTransport for QuicMediaTransport {
    async fn create_session(&self, connection: &RtcConnection) -> Result<crate::transport::TransportSession> {
        // Create a new transport session
        let session = crate::transport::TransportSession::new(connection.clone());
        
        // Store the session
        let session_id = connection.remote_node_id().to_string();
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, Arc::new(session.clone()));
        
        Ok(session)
    }
    
    async fn send_track(&self, session: &crate::transport::TransportSession, track: crate::media::MediaTrack) -> Result<()> {
        // Create a new send flow
        let send_flow = session.new_send_flow().await?;
        
        // Implementation details for sending a track will be added later
        // This would involve setting up a QuicMediaTrack with the send flow
        
        Ok(())
    }
    
    async fn receive_track(&self, session: &crate::transport::TransportSession) -> Result<Option<crate::media::MediaTrack>> {
        // Create a new receive flow
        let receive_flow = session.new_receive_flow().await?;
        
        // Implementation details for receiving a track will be added later
        // This would involve setting up a QuicMediaTrack with the receive flow
        
        // Placeholder implementation
        Ok(None)
    }
}

// Default implementation
impl Default for QuicMediaTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Media track sender over QUIC
pub struct QuicMediaSender {
    /// Media track
    track: Arc<QuicMediaTrack>,
    /// Frame queue
    frame_queue: mpsc::Sender<MediaFrame>,
    /// Running state
    running: Arc<RwLock<bool>>,
}

impl QuicMediaSender {
    /// Create a new media sender
    pub fn new(track: Arc<QuicMediaTrack>, queue_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(queue_size);
        let running = Arc::new(RwLock::new(true));
        
        // Start sender task
        let track_clone = track.clone();
        let running_clone = running.clone();
        tokio::spawn(async move {
            Self::sender_task(track_clone, rx, running_clone).await;
        });
        
        Self {
            track,
            frame_queue: tx,
            running,
        }
    }
    
    /// Send a media frame
    pub async fn send_frame(&self, frame: MediaFrame) -> Result<()> {
        self.frame_queue.send(frame).await.map_err(|e| {
            SfuError::Transport(format!("Failed to queue frame: {}", e)).into()
        })
    }
    
    /// Stop the sender
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }
    
    /// Sender task
    async fn sender_task(
        track: Arc<QuicMediaTrack>,
        mut rx: mpsc::Receiver<MediaFrame>,
        running: Arc<RwLock<bool>>,
    ) {
        while *running.read().await {
            // Wait for a frame
            if let Some(frame) = rx.recv().await {
                // Send the frame
                if let Err(e) = track.send_frame(&frame).await {
                    tracing::error!("Failed to send frame: {}", e);
                }
            } else {
                // Channel closed
                break;
            }
        }
    }
}

/// Media track receiver over QUIC
pub struct QuicMediaReceiver {
    /// Media track
    track: Arc<QuicMediaTrack>,
    /// Frame queue
    frame_queue: mpsc::Receiver<MediaFrame>,
    /// Running state
    running: Arc<RwLock<bool>>,
}

impl QuicMediaReceiver {
    /// Create a new media receiver
    pub fn new(track: Arc<QuicMediaTrack>, queue_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(queue_size);
        let running = Arc::new(RwLock::new(true));
        
        // Start receiver task
        let track_clone = track.clone();
        let tx_clone = tx.clone();
        let running_clone = running.clone();
        tokio::spawn(async move {
            Self::receiver_task(track_clone, tx_clone, running_clone).await;
        });
        
        Self {
            track,
            frame_queue: rx,
            running,
        }
    }
    
    /// Receive a media frame
    pub async fn receive_frame(&mut self) -> Option<MediaFrame> {
        self.frame_queue.recv().await
    }
    
    /// Stop the receiver
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }
    
    /// Receiver task
    async fn receiver_task(
        track: Arc<QuicMediaTrack>,
        tx: mpsc::Sender<MediaFrame>,
        running: Arc<RwLock<bool>>,
    ) {
        while *running.read().await {
            // Try to receive a frame
            match track.receive_frame().await {
                Ok(Some(frame)) => {
                    // Send the frame to the queue
                    if let Err(e) = tx.send(frame).await {
                        tracing::error!("Failed to queue received frame: {}", e);
                    }
                }
                Ok(None) => {
                    // No frame available yet, wait a bit
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                }
                Err(e) => {
                    tracing::error!("Failed to receive frame: {}", e);
                    // Wait a bit before retrying
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }
}
