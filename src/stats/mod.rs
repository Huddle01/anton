// Statistics collection module for the SFU
//
// This module gathers metrics on media quality, latency, and packet loss.

use std::{
    collections::HashMap,
    sync::Arc,
    time::Instant,
};

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    media::TrackId,
    session::SessionId,
    SfuError,
};

/// Statistics for a participant session
pub struct SessionStats {
    /// Session identifier
    pub session_id: SessionId,
    /// Connection statistics
    pub connection_stats: ConnectionStats,
    /// Published track statistics
    pub published_tracks: HashMap<TrackId, TrackStats>,
    /// Subscribed track statistics
    pub subscribed_tracks: HashMap<TrackId, TrackStats>,
    /// Last update time
    pub last_update: Instant,
}

/// Connection statistics
pub struct ConnectionStats {
    /// Round-trip time in milliseconds
    pub rtt_ms: u32,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Packet loss percentage
    pub packet_loss_percent: f32,
    /// Connection duration
    pub duration_ms: u64,
}

/// Media track statistics
pub struct TrackStats {
    /// Track identifier
    pub track_id: TrackId,
    /// Packets processed
    pub packets: u64,
    /// Bytes processed
    pub bytes: u64,
    /// Current bitrate
    pub current_bitrate: u32,
    /// Packet loss percentage
    pub packet_loss_percent: f32,
    /// Jitter in milliseconds
    pub jitter_ms: f32,
    /// Frame rate (for video)
    pub frame_rate: Option<f32>,
    /// Codec name
    pub codec_name: String,
}

/// Statistics collector trait
#[async_trait]
pub trait StatsCollector: Send + Sync {
    /// Update session statistics
    async fn update_session_stats(&self, stats: SessionStats) -> Result<()>;
    
    /// Update connection statistics
    async fn update_connection_stats(
        &self,
        session_id: SessionId,
        stats: ConnectionStats,
    ) -> Result<()>;
    
    /// Update track statistics
    async fn update_track_stats(
        &self,
        session_id: SessionId,
        track_id: TrackId,
        stats: TrackStats,
        is_publisher: bool,
    ) -> Result<()>;
    
    /// Get session statistics
    async fn get_session_stats(&self, session_id: SessionId) -> Result<SessionStats>;
    
    /// Get all session statistics
    async fn get_all_session_stats(&self) -> Result<Vec<SessionStats>>;
    
    /// Get track statistics
    async fn get_track_stats(
        &self,
        session_id: SessionId,
        track_id: TrackId,
        is_publisher: bool,
    ) -> Result<TrackStats>;
}

/// Default implementation of the statistics collector
pub struct DefaultStatsCollector {
    /// Session statistics
    session_stats: Arc<RwLock<HashMap<SessionId, SessionStats>>>,
}

impl DefaultStatsCollector {
    /// Create a new statistics collector
    pub fn new() -> Self {
        Self {
            session_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl StatsCollector for DefaultStatsCollector {
    async fn update_session_stats(&self, stats: SessionStats) -> Result<()> {
        let mut session_stats = self.session_stats.write().await;
        session_stats.insert(stats.session_id, stats);
        Ok(())
    }
    
    async fn update_connection_stats(
        &self,
        session_id: SessionId,
        stats: ConnectionStats,
    ) -> Result<()> {
        let mut session_stats = self.session_stats.write().await;
        
        if let Some(session) = session_stats.get_mut(&session_id) {
            session.connection_stats = stats;
            session.last_update = Instant::now();
        } else {
            return Err(SfuError::Session(format!("Session not found: {}", session_id)).into());
        }
        
        Ok(())
    }
    
    async fn update_track_stats(
        &self,
        session_id: SessionId,
        track_id: TrackId,
        stats: TrackStats,
        is_publisher: bool,
    ) -> Result<()> {
        let mut session_stats = self.session_stats.write().await;
        
        if let Some(session) = session_stats.get_mut(&session_id) {
            if is_publisher {
                session.published_tracks.insert(track_id, stats);
            } else {
                session.subscribed_tracks.insert(track_id, stats);
            }
            session.last_update = Instant::now();
        } else {
            return Err(SfuError::Session(format!("Session not found: {}", session_id)).into());
        }
        
        Ok(())
    }
    
    async fn get_session_stats(&self, session_id: SessionId) -> Result<SessionStats> {
        let session_stats = self.session_stats.read().await;
        
        session_stats
            .get(&session_id)
            .cloned()
            .ok_or_else(|| SfuError::Session(format!("Session not found: {}", session_id)).into())
    }
    
    async fn get_all_session_stats(&self) -> Result<Vec<SessionStats>> {
        let session_stats = self.session_stats.read().await;
        Ok(session_stats.values().cloned().collect())
    }
    
    async fn get_track_stats(
        &self,
        session_id: SessionId,
        track_id: TrackId,
        is_publisher: bool,
    ) -> Result<TrackStats> {
        let session_stats = self.session_stats.read().await;
        
        let session = session_stats
            .get(&session_id)
            .ok_or_else(|| SfuError::Session(format!("Session not found: {}", session_id)))?;
        
        if is_publisher {
            session
                .published_tracks
                .get(&track_id)
                .cloned()
                .ok_or_else(|| SfuError::Media(format!("Published track not found: {}", track_id)).into())
        } else {
            session
                .subscribed_tracks
                .get(&track_id)
                .cloned()
                .ok_or_else(|| SfuError::Media(format!("Subscribed track not found: {}", track_id)).into())
        }
    }
}

// Default implementation
impl Default for DefaultStatsCollector {
    fn default() -> Self {
        Self::new()
    }
}
