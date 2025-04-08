// Feedback channel module for the SFU
//
// This module implements feedback mechanisms for media quality and simulcast control.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::{
    media::TrackId,
    session::SessionId,
    transport::TransportSession,
    SfuError,
};

/// Feedback message types
#[derive(Debug, Clone)]
pub enum FeedbackMessage {
    /// Receiver report
    ReceiverReport(ReceiverReport),
    /// Sender report
    SenderReport(SenderReport),
    /// Simulcast control
    SimulcastControl(SimulcastControlMessage),
    /// Picture loss indication
    PictureLossIndication(PictureLossIndication),
    /// Bandwidth estimation
    BandwidthEstimation(BandwidthEstimation),
}

/// Receiver report
#[derive(Debug, Clone)]
pub struct ReceiverReport {
    /// Session identifier
    pub session_id: SessionId,
    /// Track identifier
    pub track_id: TrackId,
    /// Packet loss percentage
    pub packet_loss: f32,
    /// Jitter in milliseconds
    pub jitter_ms: f32,
    /// Round-trip time in milliseconds
    pub rtt_ms: u32,
    /// Received bitrate
    pub received_bitrate: u32,
}

/// Sender report
#[derive(Debug, Clone)]
pub struct SenderReport {
    /// Session identifier
    pub session_id: SessionId,
    /// Track identifier
    pub track_id: TrackId,
    /// Packets sent
    pub packets_sent: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Current bitrate
    pub current_bitrate: u32,
}

/// Simulcast control message
#[derive(Debug, Clone)]
pub enum SimulcastControlMessage {
    /// Request to activate specific layers
    ActivateLayers {
        /// Track identifier
        track_id: TrackId,
        /// Spatial layer index to activate
        spatial_id: u8,
        /// Temporal layer index to activate
        temporal_id: u8,
    },
    /// Notification of layer switching
    LayerSwitched {
        /// Track identifier
        track_id: TrackId,
        /// New active spatial layer
        spatial_id: u8,
        /// New active temporal layer
        temporal_id: u8,
        /// Reason for the switch
        reason: SwitchReason,
    },
    /// Layer bitrate update
    LayerBitrateUpdate {
        /// Track identifier
        track_id: TrackId,
        /// Layer identifier
        layer_id: u8,
        /// New target bitrate
        target_bitrate: u32,
    },
}

/// Reason for layer switching
#[derive(Debug, Clone, Copy)]
pub enum SwitchReason {
    /// Bandwidth limitation
    Bandwidth,
    /// Explicit user request
    UserRequest,
    /// Quality adaptation
    QualityAdaptation,
    /// Error recovery
    ErrorRecovery,
}

/// Picture loss indication
#[derive(Debug, Clone)]
pub struct PictureLossIndication {
    /// Session identifier
    pub session_id: SessionId,
    /// Track identifier
    pub track_id: TrackId,
}

/// Bandwidth estimation
#[derive(Debug, Clone)]
pub struct BandwidthEstimation {
    /// Session identifier
    pub session_id: SessionId,
    /// Estimated available bandwidth
    pub available_bandwidth: u32,
    /// Bandwidth trend
    pub trend: BandwidthTrend,
}

/// Bandwidth trend
#[derive(Debug, Clone, Copy)]
pub enum BandwidthTrend {
    /// Increasing
    Increasing,
    /// Stable
    Stable,
    /// Decreasing
    Decreasing,
}

/// Feedback channel for a connection
pub struct FeedbackChannel {
    /// Transport session
    session: Arc<TransportSession>,
    /// Sender for outgoing feedback messages
    feedback_tx: mpsc::Sender<FeedbackMessage>,
    /// Receiver for incoming feedback messages
    feedback_rx: mpsc::Receiver<FeedbackMessage>,
}

impl FeedbackChannel {
    /// Create a new feedback channel
    pub fn new(session: Arc<TransportSession>) -> Self {
        let (feedback_tx, feedback_rx) = mpsc::channel(100);
        
        Self {
            session,
            feedback_tx,
            feedback_rx,
        }
    }
    
    /// Send a feedback message
    pub async fn send_feedback(&self, message: FeedbackMessage) -> Result<()> {
        self.feedback_tx.send(message).await.map_err(|e| {
            SfuError::Other(format!("Failed to send feedback message: {}", e))
        })?;
        
        Ok(())
    }
    
    /// Receive a feedback message
    pub async fn receive_feedback(&mut self) -> Option<FeedbackMessage> {
        self.feedback_rx.recv().await
    }
    
    /// Get the transport session
    pub fn session(&self) -> &TransportSession {
        &self.session
    }
}

/// Feedback manager trait
#[async_trait]
pub trait FeedbackManager: Send + Sync {
    /// Create a feedback channel for a session
    async fn create_channel(&self, session: Arc<TransportSession>) -> Result<Arc<FeedbackChannel>>;
    
    /// Process feedback messages
    async fn process_feedback(&self, channel: &mut FeedbackChannel) -> Result<()>;
    
    /// Send receiver report
    async fn send_receiver_report(&self, channel: &FeedbackChannel, report: ReceiverReport) -> Result<()>;
    
    /// Send simulcast control message
    async fn send_simulcast_control(
        &self,
        channel: &FeedbackChannel,
        control: SimulcastControlMessage,
    ) -> Result<()>;
    
    /// Send picture loss indication
    async fn send_pli(&self, channel: &FeedbackChannel, pli: PictureLossIndication) -> Result<()>;
}

/// Default implementation of the feedback manager
pub struct DefaultFeedbackManager;

impl DefaultFeedbackManager {
    /// Create a new feedback manager
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl FeedbackManager for DefaultFeedbackManager {
    async fn create_channel(&self, session: Arc<TransportSession>) -> Result<Arc<FeedbackChannel>> {
        let channel = FeedbackChannel::new(session);
        Ok(Arc::new(channel))
    }
    
    async fn process_feedback(&self, channel: &mut FeedbackChannel) -> Result<()> {
        while let Some(message) = channel.receive_feedback().await {
            match message {
                FeedbackMessage::ReceiverReport(report) => {
                    tracing::debug!("Received receiver report: {:?}", report);
                    // Process receiver report
                }
                FeedbackMessage::SimulcastControl(control) => {
                    tracing::debug!("Received simulcast control: {:?}", control);
                    // Process simulcast control
                }
                FeedbackMessage::PictureLossIndication(pli) => {
                    tracing::debug!("Received PLI: {:?}", pli);
                    // Process PLI
                }
                _ => {
                    // Process other message types
                }
            }
        }
        
        Ok(())
    }
    
    async fn send_receiver_report(&self, channel: &FeedbackChannel, report: ReceiverReport) -> Result<()> {
        channel.send_feedback(FeedbackMessage::ReceiverReport(report)).await
    }
    
    async fn send_simulcast_control(
        &self,
        channel: &FeedbackChannel,
        control: SimulcastControlMessage,
    ) -> Result<()> {
        channel.send_feedback(FeedbackMessage::SimulcastControl(control)).await
    }
    
    async fn send_pli(&self, channel: &FeedbackChannel, pli: PictureLossIndication) -> Result<()> {
        channel.send_feedback(FeedbackMessage::PictureLossIndication(pli)).await
    }
}

// Default implementation
impl Default for DefaultFeedbackManager {
    fn default() -> Self {
        Self::new()
    }
}
