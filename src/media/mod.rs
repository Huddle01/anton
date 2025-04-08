// Media routing module for the SFU
//
// This module is responsible for selective forwarding of media packets.

pub mod codec;
pub mod rtp;
pub mod frame;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    session::{SessionId, SessionManager},
    SfuError,
};

/// Unique identifier for a media track
pub type TrackId = u64;

/// Media track kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrackKind {
    /// Audio track
    Audio,
    /// Video track
    Video,
}

/// Media track
pub struct MediaTrack {
    // Implementation details will be added later
}

impl MediaTrack {
    /// Get the track kind
    pub fn kind(&self) -> TrackKind {
        // Placeholder implementation
        TrackKind::Audio
    }

    /// Get the codec name
    pub fn codec_name(&self) -> &str {
        // Placeholder implementation
        "opus"
    }

    /// Get the codec parameters
    pub fn codec_parameters(&self) -> HashMap<String, String> {
        // Placeholder implementation
        HashMap::new()
    }
}

/// Media packet with routing information
pub struct RoutableMediaPacket {
    /// Source track identifier
    pub track_id: TrackId,
    /// Publisher session identifier
    pub publisher_id: SessionId,
    /// RTP packet
    pub packet: Vec<u8>,
    /// Packet priority
    pub priority: PacketPriority,
    /// Packet timestamp
    pub timestamp: Instant,
}

/// Priority levels for media packets
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PacketPriority {
    /// Critical packets (e.g., keyframes, audio)
    Critical,
    /// High priority packets
    High,
    /// Medium priority packets
    Medium,
    /// Low priority packets
    Low,
}

/// Forwarding decision for a media packet
pub struct ForwardingDecision {
    /// Target subscriber session identifiers
    pub target_subscribers: Vec<SessionId>,
    /// Whether to adapt the packet before forwarding
    pub adapt: bool,
    /// Adaptation parameters if needed
    pub adaptation_params: Option<AdaptationParams>,
}

/// Parameters for packet adaptation
pub struct AdaptationParams {
    /// Target bitrate
    pub target_bitrate: u32,
    /// Whether to drop non-essential parts
    pub drop_non_essential: bool,
}

/// Media router trait
#[async_trait]
pub trait MediaRouter: Send + Sync {
    /// Process a media packet
    async fn process_packet(&self, packet: RoutableMediaPacket) -> Result<()>;

    /// Get forwarding decision for a packet
    async fn get_forwarding_decision(&self, packet: &RoutableMediaPacket) -> Result<ForwardingDecision>;

    /// Forward a packet to subscribers
    async fn forward_packet(&self, packet: RoutableMediaPacket, decision: ForwardingDecision) -> Result<()>;

    /// Register a new track
    async fn register_track(&self, publisher_id: SessionId, track_id: TrackId, kind: TrackKind) -> Result<()>;

    /// Unregister a track
    async fn unregister_track(&self, publisher_id: SessionId, track_id: TrackId) -> Result<()>;
}

/// Default implementation of the media router
pub struct DefaultMediaRouter {
    /// Session manager
    session_manager: Arc<dyn SessionManager>,
    /// Track registry
    tracks: Arc<RwLock<HashMap<TrackId, TrackInfo>>>,
}

/// Track information
struct TrackInfo {
    /// Track identifier
    track_id: TrackId,
    /// Publisher session identifier
    publisher_id: SessionId,
    /// Track kind
    kind: TrackKind,
    /// Subscribers
    subscribers: HashSet<SessionId>,
}

impl DefaultMediaRouter {
    /// Create a new media router
    pub fn new(session_manager: Arc<dyn SessionManager>) -> Self {
        Self {
            session_manager,
            tracks: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl MediaRouter for DefaultMediaRouter {
    async fn process_packet(&self, packet: RoutableMediaPacket) -> Result<()> {
        // Get forwarding decision
        let decision = self.get_forwarding_decision(&packet).await?;
        
        // Forward the packet
        self.forward_packet(packet, decision).await
    }

    async fn get_forwarding_decision(&self, packet: &RoutableMediaPacket) -> Result<ForwardingDecision> {
        let tracks = self.tracks.read().await;
        
        // Get track info
        let track_info = tracks
            .get(&packet.track_id)
            .ok_or_else(|| SfuError::Media(format!("Track not found: {}", packet.track_id)))?;
        
        // Get subscribers
        let target_subscribers = track_info.subscribers.iter().cloned().collect();
        
        // Create forwarding decision
        let decision = ForwardingDecision {
            target_subscribers,
            adapt: false,
            adaptation_params: None,
        };
        
        Ok(decision)
    }

    async fn forward_packet(&self, packet: RoutableMediaPacket, decision: ForwardingDecision) -> Result<()> {
        // For each target subscriber
        for subscriber_id in decision.target_subscribers {
            // Get the participant
            if let Ok(participant) = self.session_manager.get_participant(subscriber_id).await {
                let participant = participant.read().await;
                
                // Check if the participant is subscribed to this track
                if participant.subscribed_tracks.contains_key(&packet.track_id) {
                    // Forward the packet to the participant
                    // This is a placeholder - actual implementation will depend on the transport layer
                    tracing::debug!(
                        "Forwarding packet from track {} to subscriber {}",
                        packet.track_id,
                        subscriber_id
                    );
                }
            }
        }
        
        Ok(())
    }

    async fn register_track(&self, publisher_id: SessionId, track_id: TrackId, kind: TrackKind) -> Result<()> {
        let mut tracks = self.tracks.write().await;
        
        // Create track info
        let track_info = TrackInfo {
            track_id,
            publisher_id,
            kind,
            subscribers: HashSet::new(),
        };
        
        // Register the track
        tracks.insert(track_id, track_info);
        
        Ok(())
    }

    async fn unregister_track(&self, publisher_id: SessionId, track_id: TrackId) -> Result<()> {
        let mut tracks = self.tracks.write().await;
        
        // Check if the track exists and belongs to the publisher
        if let Some(track) = tracks.get(&track_id) {
            if track.publisher_id != publisher_id {
                return Err(SfuError::Media(format!(
                    "Track {} does not belong to publisher {}",
                    track_id, publisher_id
                ))
                .into());
            }
        } else {
            return Err(SfuError::Media(format!("Track not found: {}", track_id)).into());
        }
        
        // Remove the track
        tracks.remove(&track_id);
        
        Ok(())
    }
}
