// Bandwidth adaptation module for QUIC media transport
//
// This module implements bandwidth adaptation for media streams over QUIC.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use tokio::sync::RwLock;

use crate::{
    bandwidth::{BandwidthManager, BandwidthTrend},
    feedback::{BandwidthEstimation, FeedbackMessage},
    media::{
        TrackId,
        codec::CodecType,
    },
    session::SessionId,
    simulcast::{SimulcastManager, LayerId},
    transport::integration::{QuicMediaStream, StreamDirection},
    SfuError,
};

/// Bandwidth adaptation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptationStrategy {
    /// Conservative strategy (prioritizes stability)
    Conservative,
    /// Moderate strategy (balanced approach)
    Moderate,
    /// Aggressive strategy (prioritizes quality)
    Aggressive,
}

/// Bandwidth adaptation parameters
pub struct AdaptationParams {
    /// Adaptation strategy
    pub strategy: AdaptationStrategy,
    /// Minimum bitrate (bps)
    pub min_bitrate: u32,
    /// Maximum bitrate (bps)
    pub max_bitrate: u32,
    /// Target buffer size (ms)
    pub target_buffer_ms: u32,
    /// Bandwidth headroom factor (0.0-1.0)
    pub headroom_factor: f32,
    /// Upscale threshold factor (> 1.0)
    pub upscale_factor: f32,
    /// Downscale threshold factor (< 1.0)
    pub downscale_factor: f32,
    /// Stability period after adaptation (ms)
    pub stability_period_ms: u32,
}

impl Default for AdaptationParams {
    fn default() -> Self {
        Self {
            strategy: AdaptationStrategy::Moderate,
            min_bitrate: 100_000,    // 100 kbps
            max_bitrate: 5_000_000,  // 5 Mbps
            target_buffer_ms: 500,   // 500 ms
            headroom_factor: 0.8,    // Use 80% of available bandwidth
            upscale_factor: 1.2,     // Upscale when 120% bandwidth available
            downscale_factor: 0.8,   // Downscale when below 80% bandwidth
            stability_period_ms: 2000, // 2 seconds stability period
        }
    }
}

/// Bandwidth adapter for media streams
pub struct BandwidthAdapter {
    /// Session identifier
    session_id: SessionId,
    /// Track identifier
    track_id: TrackId,
    /// Codec type
    codec_type: CodecType,
    /// Current bitrate
    current_bitrate: Arc<RwLock<u32>>,
    /// Available bandwidth
    available_bandwidth: Arc<RwLock<u32>>,
    /// Bandwidth trend
    bandwidth_trend: Arc<RwLock<BandwidthTrend>>,
    /// Adaptation parameters
    params: AdaptationParams,
    /// Last adaptation time
    last_adaptation: Arc<RwLock<Instant>>,
    /// Current simulcast layer (if applicable)
    current_layer: Arc<RwLock<Option<LayerId>>>,
    /// Simulcast manager (if applicable)
    simulcast_manager: Option<Arc<dyn SimulcastManager>>,
    /// Bandwidth manager
    bandwidth_manager: Arc<dyn BandwidthManager>,
}

impl BandwidthAdapter {
    /// Create a new bandwidth adapter
    pub fn new(
        session_id: SessionId,
        track_id: TrackId,
        codec_type: CodecType,
        initial_bitrate: u32,
        bandwidth_manager: Arc<dyn BandwidthManager>,
        simulcast_manager: Option<Arc<dyn SimulcastManager>>,
        params: AdaptationParams,
    ) -> Self {
        Self {
            session_id,
            track_id,
            codec_type,
            current_bitrate: Arc::new(RwLock::new(initial_bitrate)),
            available_bandwidth: Arc::new(RwLock::new(initial_bitrate)),
            bandwidth_trend: Arc::new(RwLock::new(BandwidthTrend::Stable)),
            params,
            last_adaptation: Arc::new(RwLock::new(Instant::now())),
            current_layer: Arc::new(RwLock::new(None)),
            simulcast_manager,
            bandwidth_manager,
        }
    }
    
    /// Process a feedback message
    pub async fn process_feedback(&self, message: &FeedbackMessage) -> Result<()> {
        match message {
            FeedbackMessage::BandwidthEstimation(estimation) => {
                if estimation.session_id == self.session_id {
                    // Update available bandwidth
                    let mut available = self.available_bandwidth.write().await;
                    *available = estimation.available_bandwidth;
                    
                    // Update bandwidth trend
                    let mut trend = self.bandwidth_trend.write().await;
                    *trend = estimation.trend;
                    
                    // Check if adaptation is needed
                    self.adapt_bitrate().await?;
                }
            }
            _ => {
                // Ignore other message types
            }
        }
        
        Ok(())
    }
    
    /// Adapt bitrate based on available bandwidth
    pub async fn adapt_bitrate(&self) -> Result<()> {
        // Check if we're in stability period
        let now = Instant::now();
        let last_adaptation = *self.last_adaptation.read().await;
        if now.duration_since(last_adaptation) < Duration::from_millis(self.params.stability_period_ms as u64) {
            return Ok(());
        }
        
        let available_bandwidth = *self.available_bandwidth.read().await;
        let current_bitrate = *self.current_bitrate.read().await;
        let trend = *self.bandwidth_trend.read().await;
        
        // Calculate target bitrate with headroom
        let target_bitrate = (available_bandwidth as f32 * self.params.headroom_factor) as u32;
        
        // Determine if adaptation is needed
        let mut new_bitrate = current_bitrate;
        let mut should_adapt = false;
        
        if target_bitrate > current_bitrate * self.params.upscale_factor as u32 && trend == BandwidthTrend::Increasing {
            // Increase bitrate
            new_bitrate = (current_bitrate as f32 * self.params.upscale_factor) as u32;
            should_adapt = true;
        } else if target_bitrate < current_bitrate * self.params.downscale_factor as u32 || trend == BandwidthTrend::Decreasing {
            // Decrease bitrate
            new_bitrate = (current_bitrate as f32 * self.params.downscale_factor) as u32;
            should_adapt = true;
        }
        
        // Apply min/max constraints
        new_bitrate = new_bitrate.max(self.params.min_bitrate).min(self.params.max_bitrate);
        
        if should_adapt && new_bitrate != current_bitrate {
            // Update current bitrate
            let mut current = self.current_bitrate.write().await;
            *current = new_bitrate;
            
            // Update last adaptation time
            let mut last = self.last_adaptation.write().await;
            *last = now;
            
            // If simulcast is enabled, select appropriate layer
            if let Some(simulcast_manager) = &self.simulcast_manager {
                let layer_id = simulcast_manager.select_layer(
                    self.track_id,
                    self.session_id,
                    new_bitrate,
                ).await?;
                
                // Update current layer
                let mut current_layer = self.current_layer.write().await;
                *current_layer = Some(layer_id);
            }
            
            // Update bandwidth manager
            self.bandwidth_manager.update_bandwidth(
                self.session_id,
                new_bitrate,
                true, // This is upload bandwidth
            ).await?;
        }
        
        Ok(())
    }
    
    /// Get the current bitrate
    pub async fn current_bitrate(&self) -> u32 {
        *self.current_bitrate.read().await
    }
    
    /// Get the current simulcast layer
    pub async fn current_layer(&self) -> Option<LayerId> {
        *self.current_layer.read().await
    }
    
    /// Apply adaptation to a media stream
    pub async fn apply_to_stream(&self, stream: &QuicMediaStream) -> Result<()> {
        // Only apply to send streams
        if stream.config().direction == StreamDirection::SendOnly || 
           stream.config().direction == StreamDirection::SendRecv {
            // Get current bitrate
            let bitrate = self.current_bitrate().await;
            
            // Apply bitrate to stream (implementation depends on codec)
            match self.codec_type {
                CodecType::Opus => {
                    // For Opus, we would adjust the encoding parameters
                    // This is a placeholder for actual implementation
                    tracing::debug!(
                        "Adapting Opus stream bitrate to {} bps for session {} track {}",
                        bitrate,
                        self.session_id,
                        self.track_id
                    );
                }
                CodecType::VP9 => {
                    // For VP9, we would adjust encoding parameters and possibly switch layers
                    // This is a placeholder for actual implementation
                    let layer = self.current_layer().await;
                    tracing::debug!(
                        "Adapting VP9 stream bitrate to {} bps (layer {:?}) for session {} track {}",
                        bitrate,
                        layer,
                        self.session_id,
                        self.track_id
                    );
                }
                _ => {
                    return Err(SfuError::Media(format!("Unsupported codec for adaptation: {:?}", self.codec_type)).into());
                }
            }
        }
        
        Ok(())
    }
}

/// Bandwidth adaptation manager
pub struct BandwidthAdaptationManager {
    /// Bandwidth manager
    bandwidth_manager: Arc<dyn BandwidthManager>,
    /// Simulcast manager
    simulcast_manager: Option<Arc<dyn SimulcastManager>>,
    /// Default adaptation parameters
    default_params: AdaptationParams,
}

impl BandwidthAdaptationManager {
    /// Create a new bandwidth adaptation manager
    pub fn new(
        bandwidth_manager: Arc<dyn BandwidthManager>,
        simulcast_manager: Option<Arc<dyn SimulcastManager>>,
    ) -> Self {
        Self {
            bandwidth_manager,
            simulcast_manager,
            default_params: AdaptationParams::default(),
        }
    }
    
    /// Create a bandwidth adapter for a track
    pub fn create_adapter(
        &self,
        session_id: SessionId,
        track_id: TrackId,
        codec_type: CodecType,
        initial_bitrate: u32,
    ) -> BandwidthAdapter {
        BandwidthAdapter::new(
            session_id,
            track_id,
            codec_type,
            initial_bitrate,
            self.bandwidth_manager.clone(),
            self.simulcast_manager.clone(),
            self.default_params.clone(),
        )
    }
    
    /// Create a bandwidth adapter with custom parameters
    pub fn create_adapter_with_params(
        &self,
        session_id: SessionId,
        track_id: TrackId,
        codec_type: CodecType,
        initial_bitrate: u32,
        params: AdaptationParams,
    ) -> BandwidthAdapter {
        BandwidthAdapter::new(
            session_id,
            track_id,
            codec_type,
            initial_bitrate,
            self.bandwidth_manager.clone(),
            self.simulcast_manager.clone(),
            params,
        )
    }
    
    /// Get the bandwidth manager
    pub fn bandwidth_manager(&self) -> &Arc<dyn BandwidthManager> {
        &self.bandwidth_manager
    }
    
    /// Get the simulcast manager
    pub fn simulcast_manager(&self) -> &Option<Arc<dyn SimulcastManager>> {
        &self.simulcast_manager
    }
    
    /// Get the default adaptation parameters
    pub fn default_params(&self) -> &AdaptationParams {
        &self.default_params
    }
    
    /// Set the default adaptation parameters
    pub fn set_default_params(&mut self, params: AdaptationParams) {
        self.default_params = params;
    }
}
