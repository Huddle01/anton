// Simulcast module for the SFU
//
// This module implements simulcast support for video streams.

use std::{
    collections::HashMap,
    sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    media::TrackId,
    session::SessionId,
    feedback::{SimulcastControlMessage, SwitchReason},
    SfuError,
};

/// Layer identifier
pub type LayerId = u8;

/// Simulcast layer
pub struct SimulcastLayer {
    /// Layer identifier
    pub layer_id: LayerId,
    /// Spatial resolution index (0 = lowest)
    pub spatial_id: u8,
    /// Temporal resolution index (0 = lowest)
    pub temporal_id: u8,
    /// Target bitrate for this layer
    pub target_bitrate: u32,
    /// Current active state
    pub active: bool,
}

/// Resolution specification
pub struct Resolution {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

/// Simulcast encoding configuration
pub struct SimulcastConfig {
    /// Number of spatial layers
    pub spatial_layers: u8,
    /// Number of temporal layers per spatial layer
    pub temporal_layers: u8,
    /// Base resolution for lowest spatial layer
    pub base_resolution: Resolution,
    /// Base frame rate for lowest temporal layer
    pub base_framerate: f32,
    /// Scaling factor between spatial layers
    pub spatial_scale_factor: f32,
    /// Scaling factor between temporal layers
    pub temporal_scale_factor: f32,
}

/// Target parameters for encoding a layer
pub struct EncodingTarget {
    /// Resolution
    pub resolution: Resolution,
    /// Frame rate
    pub framerate: f32,
    /// Target bitrate
    pub bitrate: u32,
    /// Quality parameter (0-100)
    pub quality: u8,
}

/// Simulcast manager trait
#[async_trait]
pub trait SimulcastManager: Send + Sync {
    /// Register a simulcast track
    async fn register_track(
        &self,
        track_id: TrackId,
        publisher_id: SessionId,
        config: SimulcastConfig,
    ) -> Result<()>;
    
    /// Unregister a simulcast track
    async fn unregister_track(&self, track_id: TrackId) -> Result<()>;
    
    /// Get available layers for a track
    async fn get_available_layers(&self, track_id: TrackId) -> Result<Vec<SimulcastLayer>>;
    
    /// Select layer for a subscriber
    async fn select_layer(
        &self,
        track_id: TrackId,
        subscriber_id: SessionId,
        available_bandwidth: u32,
    ) -> Result<LayerId>;
    
    /// Process simulcast control message
    async fn process_control_message(
        &self,
        message: SimulcastControlMessage,
        publisher_id: SessionId,
    ) -> Result<()>;
}

/// Default implementation of the simulcast manager
pub struct DefaultSimulcastManager {
    /// Track configurations
    track_configs: Arc<RwLock<HashMap<TrackId, TrackSimulcastInfo>>>,
}

/// Track simulcast information
struct TrackSimulcastInfo {
    /// Track identifier
    track_id: TrackId,
    /// Publisher session identifier
    publisher_id: SessionId,
    /// Simulcast configuration
    config: SimulcastConfig,
    /// Available layers
    layers: Vec<SimulcastLayer>,
    /// Subscriber layer selections
    subscriber_selections: HashMap<SessionId, LayerId>,
}

impl DefaultSimulcastManager {
    /// Create a new simulcast manager
    pub fn new() -> Self {
        Self {
            track_configs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create layers from configuration
    fn create_layers(config: &SimulcastConfig) -> Vec<SimulcastLayer> {
        let mut layers = Vec::new();
        let mut layer_id = 0;
        
        for spatial_id in 0..config.spatial_layers {
            for temporal_id in 0..config.temporal_layers {
                // Calculate target bitrate based on layer indices
                // This is a simplified model; real implementations would be more sophisticated
                let spatial_factor = config.spatial_scale_factor.powi(spatial_id as i32);
                let temporal_factor = config.temporal_scale_factor.powi(temporal_id as i32);
                let base_bitrate = 500_000; // 500 kbps base bitrate
                let target_bitrate = (base_bitrate as f32 * spatial_factor * temporal_factor) as u32;
                
                let layer = SimulcastLayer {
                    layer_id,
                    spatial_id,
                    temporal_id,
                    target_bitrate,
                    active: spatial_id == 0 && temporal_id == 0, // Only activate base layer by default
                };
                
                layers.push(layer);
                layer_id += 1;
            }
        }
        
        layers
    }
    
    /// Get layer by spatial and temporal IDs
    fn find_layer(
        layers: &[SimulcastLayer],
        spatial_id: u8,
        temporal_id: u8,
    ) -> Option<&SimulcastLayer> {
        layers.iter().find(|layer| layer.spatial_id == spatial_id && layer.temporal_id == temporal_id)
    }
}

#[async_trait]
impl SimulcastManager for DefaultSimulcastManager {
    async fn register_track(
        &self,
        track_id: TrackId,
        publisher_id: SessionId,
        config: SimulcastConfig,
    ) -> Result<()> {
        let mut track_configs = self.track_configs.write().await;
        
        // Create layers from configuration
        let layers = Self::create_layers(&config);
        
        // Create track simulcast info
        let track_info = TrackSimulcastInfo {
            track_id,
            publisher_id,
            config,
            layers,
            subscriber_selections: HashMap::new(),
        };
        
        // Register the track
        track_configs.insert(track_id, track_info);
        
        Ok(())
    }
    
    async fn unregister_track(&self, track_id: TrackId) -> Result<()> {
        let mut track_configs = self.track_configs.write().await;
        
        // Remove the track
        if track_configs.remove(&track_id).is_none() {
            return Err(SfuError::Media(format!("Simulcast track not found: {}", track_id)).into());
        }
        
        Ok(())
    }
    
    async fn get_available_layers(&self, track_id: TrackId) -> Result<Vec<SimulcastLayer>> {
        let track_configs = self.track_configs.read().await;
        
        // Get track info
        let track_info = track_configs
            .get(&track_id)
            .ok_or_else(|| SfuError::Media(format!("Simulcast track not found: {}", track_id)))?;
        
        // Return available layers
        Ok(track_info.layers.clone())
    }
    
    async fn select_layer(
        &self,
        track_id: TrackId,
        subscriber_id: SessionId,
        available_bandwidth: u32,
    ) -> Result<LayerId> {
        let mut track_configs = self.track_configs.write().await;
        
        // Get track info
        let track_info = track_configs
            .get_mut(&track_id)
            .ok_or_else(|| SfuError::Media(format!("Simulcast track not found: {}", track_id)))?;
        
        // Find the highest quality layer that fits within the available bandwidth
        let mut selected_layer_id = 0; // Default to lowest layer
        
        for layer in &track_info.layers {
            if layer.active && layer.target_bitrate <= available_bandwidth {
                selected_layer_id = layer.layer_id;
            }
        }
        
        // Update subscriber selection
        track_info.subscriber_selections.insert(subscriber_id, selected_layer_id);
        
        Ok(selected_layer_id)
    }
    
    async fn process_control_message(
        &self,
        message: SimulcastControlMessage,
        publisher_id: SessionId,
    ) -> Result<()> {
        match message {
            SimulcastControlMessage::ActivateLayers {
                track_id,
                spatial_id,
                temporal_id,
            } => {
                let mut track_configs = self.track_configs.write().await;
                
                // Get track info
                let track_info = track_configs
                    .get_mut(&track_id)
                    .ok_or_else(|| SfuError::Media(format!("Simulcast track not found: {}", track_id)))?;
                
                // Verify publisher
                if track_info.publisher_id != publisher_id {
                    return Err(SfuError::Media(format!(
                        "Track {} does not belong to publisher {}",
                        track_id, publisher_id
                    ))
                    .into());
                }
                
                // Find the layer
                let layer = Self::find_layer(&track_info.layers, spatial_id, temporal_id)
                    .ok_or_else(|| {
                        SfuError::Media(format!(
                            "Layer not found: spatial={}, temporal={}",
                            spatial_id, temporal_id
                        ))
                    })?;
                
                // Activate the layer
                let layer_id = layer.layer_id;
                if let Some(layer) = track_info
                    .layers
                    .iter_mut()
                    .find(|l| l.layer_id == layer_id)
                {
                    layer.active = true;
                }
                
                Ok(())
            }
            SimulcastControlMessage::LayerSwitched { .. } => {
                // This is a notification, no action needed
                Ok(())
            }
            SimulcastControlMessage::LayerBitrateUpdate {
                track_id,
                layer_id,
                target_bitrate,
            } => {
                let mut track_configs = self.track_configs.write().await;
                
                // Get track info
                let track_info = track_configs
                    .get_mut(&track_id)
                    .ok_or_else(|| SfuError::Media(format!("Simulcast track not found: {}", track_id)))?;
                
                // Verify publisher
                if track_info.publisher_id != publisher_id {
                    return Err(SfuError::Media(format!(
                        "Track {} does not belong to publisher {}",
                        track_id, publisher_id
                    ))
                    .into());
                }
                
                // Update layer bitrate
                if let Some(layer) = track_info
                    .layers
                    .iter_mut()
                    .find(|l| l.layer_id == layer_id)
                {
                    layer.target_bitrate = target_bitrate;
                } else {
                    return Err(SfuError::Media(format!("Layer not found: {}", layer_id)).into());
                }
                
                Ok(())
            }
        }
    }
}

// Default implementation
impl Default for DefaultSimulcastManager {
    fn default() -> Self {
        Self::new()
    }
}
