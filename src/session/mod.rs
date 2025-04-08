// Session management module for the SFU
//
// This module handles participant sessions and their associated streams.

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Instant,
};

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    connection::RtcConnection,
    media::{MediaTrack, TrackId, TrackKind},
    SfuError,
};

/// Unique identifier for a participant session
pub type SessionId = u64;

/// Represents a participant in the SFU
pub struct Participant {
    /// Unique session identifier
    pub session_id: SessionId,
    /// Iroh node identifier
    pub node_id: iroh::NodeId,
    /// Connection to the participant
    pub connection: RtcConnection,
    /// Tracks published by this participant
    pub published_tracks: HashMap<TrackId, PublishedTrack>,
    /// Tracks subscribed to by this participant
    pub subscribed_tracks: HashMap<TrackId, SubscribedTrack>,
    /// Bandwidth information
    pub bandwidth: BandwidthInfo,
    /// Last activity timestamp
    pub last_activity: Instant,
}

/// Information about a track published by a participant
pub struct PublishedTrack {
    /// Unique track identifier
    pub track_id: TrackId,
    /// Owner of this track
    pub publisher_id: SessionId,
    /// Media kind (audio/video)
    pub kind: TrackKind,
    /// Codec information
    pub codec: CodecInfo,
    /// Current bitrate
    pub current_bitrate: u32,
    /// Subscribers to this track
    pub subscribers: HashSet<SessionId>,
}

/// Information about a track subscribed to by a participant
pub struct SubscribedTrack {
    /// Unique track identifier
    pub track_id: TrackId,
    /// Publisher of this track
    pub publisher_id: SessionId,
    /// Media receiver
    pub receiver: MediaTrackReceiver,
}

/// Bandwidth information for a participant
pub struct BandwidthInfo {
    /// Estimated available upload bandwidth
    pub upload_bandwidth: u32,
    /// Estimated available download bandwidth
    pub download_bandwidth: u32,
    /// Last bandwidth update time
    pub last_update: Instant,
}

/// Codec information
pub struct CodecInfo {
    /// Codec name
    pub name: String,
    /// Codec parameters
    pub parameters: HashMap<String, String>,
}

/// Media track receiver
pub struct MediaTrackReceiver {
    // Implementation details will be added later
}

/// Session manager trait
#[async_trait]
pub trait SessionManager: Send + Sync {
    /// Create a new session for a participant
    async fn create_session(&self, node_id: iroh::NodeId, connection: RtcConnection) -> Result<SessionId>;

    /// Get a participant by session ID
    async fn get_participant(&self, session_id: SessionId) -> Result<Arc<RwLock<Participant>>>;

    /// Remove a session
    async fn remove_session(&self, session_id: SessionId) -> Result<()>;

    /// Register a published track
    async fn register_published_track(
        &self,
        session_id: SessionId,
        track: MediaTrack,
    ) -> Result<TrackId>;

    /// Register a subscribed track
    async fn register_subscribed_track(
        &self,
        subscriber_id: SessionId,
        publisher_id: SessionId,
        track_id: TrackId,
    ) -> Result<()>;

    /// Unregister a subscribed track
    async fn unregister_subscribed_track(
        &self,
        subscriber_id: SessionId,
        track_id: TrackId,
    ) -> Result<()>;

    /// Get all active sessions
    async fn get_all_sessions(&self) -> Result<Vec<SessionId>>;

    /// Get all published tracks for a session
    async fn get_published_tracks(&self, session_id: SessionId) -> Result<Vec<TrackId>>;

    /// Get all subscribed tracks for a session
    async fn get_subscribed_tracks(&self, session_id: SessionId) -> Result<Vec<TrackId>>;
}

/// Default implementation of the session manager
pub struct DefaultSessionManager {
    participants: Arc<RwLock<HashMap<SessionId, Arc<RwLock<Participant>>>>>,
    next_session_id: Arc<Mutex<SessionId>>,
    next_track_id: Arc<Mutex<TrackId>>,
}

impl DefaultSessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            participants: Arc::new(RwLock::new(HashMap::new())),
            next_session_id: Arc::new(Mutex::new(1)),
            next_track_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Generate a new session ID
    fn generate_session_id(&self) -> SessionId {
        let mut id = self.next_session_id.lock().unwrap();
        let current = *id;
        *id += 1;
        current
    }

    /// Generate a new track ID
    fn generate_track_id(&self) -> TrackId {
        let mut id = self.next_track_id.lock().unwrap();
        let current = *id;
        *id += 1;
        current
    }
}

#[async_trait]
impl SessionManager for DefaultSessionManager {
    async fn create_session(&self, node_id: iroh::NodeId, connection: RtcConnection) -> Result<SessionId> {
        let session_id = self.generate_session_id();
        
        let participant = Participant {
            session_id,
            node_id,
            connection,
            published_tracks: HashMap::new(),
            subscribed_tracks: HashMap::new(),
            bandwidth: BandwidthInfo {
                upload_bandwidth: 0,
                download_bandwidth: 0,
                last_update: Instant::now(),
            },
            last_activity: Instant::now(),
        };
        
        let mut participants = self.participants.write().await;
        participants.insert(session_id, Arc::new(RwLock::new(participant)));
        
        Ok(session_id)
    }

    async fn get_participant(&self, session_id: SessionId) -> Result<Arc<RwLock<Participant>>> {
        let participants = self.participants.read().await;
        
        participants
            .get(&session_id)
            .cloned()
            .ok_or_else(|| SfuError::Session(format!("Session not found: {}", session_id)).into())
    }

    async fn remove_session(&self, session_id: SessionId) -> Result<()> {
        let mut participants = self.participants.write().await;
        
        if participants.remove(&session_id).is_none() {
            return Err(SfuError::Session(format!("Session not found: {}", session_id)).into());
        }
        
        Ok(())
    }

    async fn register_published_track(
        &self,
        session_id: SessionId,
        track: MediaTrack,
    ) -> Result<TrackId> {
        let participant = self.get_participant(session_id).await?;
        let track_id = self.generate_track_id();
        
        let mut participant = participant.write().await;
        
        let published_track = PublishedTrack {
            track_id,
            publisher_id: session_id,
            kind: track.kind(),
            codec: CodecInfo {
                name: track.codec_name().to_string(),
                parameters: track.codec_parameters(),
            },
            current_bitrate: 0,
            subscribers: HashSet::new(),
        };
        
        participant.published_tracks.insert(track_id, published_track);
        
        Ok(track_id)
    }

    async fn register_subscribed_track(
        &self,
        subscriber_id: SessionId,
        publisher_id: SessionId,
        track_id: TrackId,
    ) -> Result<()> {
        let publisher = self.get_participant(publisher_id).await?;
        let subscriber = self.get_participant(subscriber_id).await?;
        
        // Check if the track exists
        let publisher_read = publisher.read().await;
        if !publisher_read.published_tracks.contains_key(&track_id) {
            return Err(SfuError::Media(format!("Track not found: {}", track_id)).into());
        }
        
        // Add subscriber to the track
        let mut publisher = publisher.write().await;
        if let Some(track) = publisher.published_tracks.get_mut(&track_id) {
            track.subscribers.insert(subscriber_id);
        }
        
        // Create a receiver for the subscriber
        let mut subscriber = subscriber.write().await;
        subscriber.subscribed_tracks.insert(
            track_id,
            SubscribedTrack {
                track_id,
                publisher_id,
                receiver: MediaTrackReceiver {
                    // Implementation details will be added later
                },
            },
        );
        
        Ok(())
    }

    async fn unregister_subscribed_track(
        &self,
        subscriber_id: SessionId,
        track_id: TrackId,
    ) -> Result<()> {
        let subscriber = self.get_participant(subscriber_id).await?;
        
        // Get the publisher ID from the subscribed track
        let publisher_id = {
            let subscriber_read = subscriber.read().await;
            match subscriber_read.subscribed_tracks.get(&track_id) {
                Some(track) => track.publisher_id,
                None => return Err(SfuError::Media(format!("Track not subscribed: {}", track_id)).into()),
            }
        };
        
        // Remove the track from the subscriber
        {
            let mut subscriber = subscriber.write().await;
            subscriber.subscribed_tracks.remove(&track_id);
        }
        
        // Remove the subscriber from the publisher's track
        if let Ok(publisher) = self.get_participant(publisher_id).await {
            let mut publisher = publisher.write().await;
            if let Some(track) = publisher.published_tracks.get_mut(&track_id) {
                track.subscribers.remove(&subscriber_id);
            }
        }
        
        Ok(())
    }

    async fn get_all_sessions(&self) -> Result<Vec<SessionId>> {
        let participants = self.participants.read().await;
        Ok(participants.keys().cloned().collect())
    }

    async fn get_published_tracks(&self, session_id: SessionId) -> Result<Vec<TrackId>> {
        let participant = self.get_participant(session_id).await?;
        let participant = participant.read().await;
        Ok(participant.published_tracks.keys().cloned().collect())
    }

    async fn get_subscribed_tracks(&self, session_id: SessionId) -> Result<Vec<TrackId>> {
        let participant = self.get_participant(session_id).await?;
        let participant = participant.read().await;
        Ok(participant.subscribed_tracks.keys().cloned().collect())
    }
}

// Default implementation
impl Default for DefaultSessionManager {
    fn default() -> Self {
        Self::new()
    }
}
