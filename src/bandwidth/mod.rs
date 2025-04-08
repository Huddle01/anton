// Bandwidth management module for the SFU
//
// This module handles bandwidth estimation and adaptation.

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    media::TrackId,
    session::SessionId,
    feedback::{BandwidthEstimation, BandwidthTrend},
    SfuError,
};

/// Bandwidth information
pub struct BandwidthInfo {
    /// Estimated available upload bandwidth in bps
    pub upload_bandwidth: u32,
    /// Estimated available download bandwidth in bps
    pub download_bandwidth: u32,
    /// Last update time
    pub last_update: Instant,
    /// Bandwidth history for trend analysis
    pub history: Vec<(Instant, u32)>,
    /// Current bandwidth trend
    pub trend: BandwidthTrend,
}

/// Bandwidth manager trait
#[async_trait]
pub trait BandwidthManager: Send + Sync {
    /// Update bandwidth estimation for a session
    async fn update_bandwidth(
        &self,
        session_id: SessionId,
        bandwidth: u32,
        is_upload: bool,
    ) -> Result<()>;
    
    /// Get recommended bitrate for a track
    async fn get_recommended_bitrate(
        &self,
        session_id: SessionId,
        track_id: TrackId,
    ) -> Result<u32>;
    
    /// Get bandwidth trend for a session
    async fn get_bandwidth_trend(&self, session_id: SessionId) -> Result<BandwidthTrend>;
    
    /// Process bandwidth estimation message
    async fn process_bandwidth_estimation(&self, estimation: BandwidthEstimation) -> Result<()>;
    
    /// Distribute bandwidth among tracks
    async fn distribute_bandwidth(
        &self,
        session_id: SessionId,
        available_bandwidth: u32,
        track_priorities: HashMap<TrackId, u8>,
    ) -> Result<HashMap<TrackId, u32>>;
}

/// Default implementation of the bandwidth manager
pub struct DefaultBandwidthManager {
    /// Session bandwidth information
    session_bandwidth: Arc<RwLock<HashMap<SessionId, BandwidthInfo>>>,
    /// Track bitrate allocations
    track_bitrates: Arc<RwLock<HashMap<(SessionId, TrackId), u32>>>,
    /// History window size
    history_window: Duration,
    /// Minimum bandwidth for trend analysis
    min_samples_for_trend: usize,
}

impl DefaultBandwidthManager {
    /// Create a new bandwidth manager
    pub fn new() -> Self {
        Self {
            session_bandwidth: Arc::new(RwLock::new(HashMap::new())),
            track_bitrates: Arc::new(RwLock::new(HashMap::new())),
            history_window: Duration::from_secs(10),
            min_samples_for_trend: 5,
        }
    }
    
    /// Calculate bandwidth trend from history
    fn calculate_trend(history: &[(Instant, u32)], min_samples: usize) -> BandwidthTrend {
        if history.len() < min_samples {
            return BandwidthTrend::Stable;
        }
        
        // Simple linear regression to determine trend
        let n = history.len() as f64;
        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_xy = 0.0;
        let mut sum_xx = 0.0;
        
        let base_time = history[0].0;
        
        for (time, bandwidth) in history {
            let x = time.duration_since(base_time).as_secs_f64();
            let y = *bandwidth as f64;
            
            sum_x += x;
            sum_y += y;
            sum_xy += x * y;
            sum_xx += x * x;
        }
        
        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_xx - sum_x * sum_x);
        
        // Determine trend based on slope
        if slope > 1000.0 {
            BandwidthTrend::Increasing
        } else if slope < -1000.0 {
            BandwidthTrend::Decreasing
        } else {
            BandwidthTrend::Stable
        }
    }
    
    /// Prune old history entries
    fn prune_history(history: &mut Vec<(Instant, u32)>, window: Duration) {
        let now = Instant::now();
        history.retain(|(time, _)| now.duration_since(*time) <= window);
    }
}

#[async_trait]
impl BandwidthManager for DefaultBandwidthManager {
    async fn update_bandwidth(
        &self,
        session_id: SessionId,
        bandwidth: u32,
        is_upload: bool,
    ) -> Result<()> {
        let mut session_bandwidth = self.session_bandwidth.write().await;
        
        // Get or create bandwidth info
        let bandwidth_info = session_bandwidth.entry(session_id).or_insert_with(|| BandwidthInfo {
            upload_bandwidth: 0,
            download_bandwidth: 0,
            last_update: Instant::now(),
            history: Vec::new(),
            trend: BandwidthTrend::Stable,
        });
        
        // Update bandwidth
        if is_upload {
            bandwidth_info.upload_bandwidth = bandwidth;
        } else {
            bandwidth_info.download_bandwidth = bandwidth;
        }
        
        // Update history
        bandwidth_info.history.push((Instant::now(), bandwidth));
        
        // Prune old history entries
        Self::prune_history(&mut bandwidth_info.history, self.history_window);
        
        // Calculate trend
        bandwidth_info.trend = Self::calculate_trend(
            &bandwidth_info.history,
            self.min_samples_for_trend,
        );
        
        // Update last update time
        bandwidth_info.last_update = Instant::now();
        
        Ok(())
    }
    
    async fn get_recommended_bitrate(
        &self,
        session_id: SessionId,
        track_id: TrackId,
    ) -> Result<u32> {
        let track_bitrates = self.track_bitrates.read().await;
        
        // Get bitrate allocation for the track
        if let Some(bitrate) = track_bitrates.get(&(session_id, track_id)) {
            return Ok(*bitrate);
        }
        
        // If no specific allocation, use a default based on session bandwidth
        let session_bandwidth = self.session_bandwidth.read().await;
        
        if let Some(bandwidth_info) = session_bandwidth.get(&session_id) {
            // Use a conservative default (70% of available bandwidth)
            let default_bitrate = (bandwidth_info.upload_bandwidth as f32 * 0.7) as u32;
            Ok(default_bitrate)
        } else {
            // No bandwidth info available, use a very conservative default
            Ok(500_000) // 500 kbps
        }
    }
    
    async fn get_bandwidth_trend(&self, session_id: SessionId) -> Result<BandwidthTrend> {
        let session_bandwidth = self.session_bandwidth.read().await;
        
        if let Some(bandwidth_info) = session_bandwidth.get(&session_id) {
            Ok(bandwidth_info.trend)
        } else {
            // No bandwidth info available
            Err(SfuError::Other(format!("No bandwidth info for session {}", session_id)).into())
        }
    }
    
    async fn process_bandwidth_estimation(&self, estimation: BandwidthEstimation) -> Result<()> {
        // Update bandwidth based on estimation
        self.update_bandwidth(
            estimation.session_id,
            estimation.available_bandwidth,
            false, // Assuming this is download bandwidth
        ).await
    }
    
    async fn distribute_bandwidth(
        &self,
        session_id: SessionId,
        available_bandwidth: u32,
        track_priorities: HashMap<TrackId, u8>,
    ) -> Result<HashMap<TrackId, u32>> {
        let mut track_bitrates = self.track_bitrates.write().await;
        let mut allocations = HashMap::new();
        
        if track_priorities.is_empty() {
            return Ok(allocations);
        }
        
        // Calculate total priority weight
        let total_priority: u32 = track_priorities.values().map(|p| *p as u32).sum();
        
        // Distribute bandwidth proportionally to priorities
        for (track_id, priority) in track_priorities {
            let allocation = (available_bandwidth as f32 * priority as f32 / total_priority as f32) as u32;
            
            // Update allocation
            allocations.insert(track_id, allocation);
            track_bitrates.insert((session_id, track_id), allocation);
        }
        
        Ok(allocations)
    }
}

// Default implementation
impl Default for DefaultBandwidthManager {
    fn default() -> Self {
        Self::new()
    }
}
