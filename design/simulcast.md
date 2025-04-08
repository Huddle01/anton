# Simulcast Implementation for Media over QUIC

## Overview

This document details the simulcast implementation for our SFU using media over QUIC with iroh. Simulcast is a technique where a publisher sends multiple versions of the same media stream at different quality levels, allowing the SFU to selectively forward the appropriate version to each subscriber based on their bandwidth capabilities and requirements.

## Simulcast Architecture

### 1. Layer Structure

#### 1.1 Spatial Layers
- Different resolution versions of the same video
- Typically 3 layers: low, medium, high resolution
- Each spatial layer has a unique identifier
- Independent encoding of each resolution

#### 1.2 Temporal Layers
- Different frame rate versions within each spatial layer
- Typically 3 layers: low, medium, high frame rate
- Hierarchical structure where higher layers depend on lower layers
- Enables fine-grained adaptation to bandwidth fluctuations

### 2. Publisher Implementation

#### 2.1 Encoding Configuration
```rust
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

/// Resolution specification
pub struct Resolution {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}
```

#### 2.2 Layer Management
```rust
/// Manages simulcast layers for a publisher
pub struct SimulcastManager {
    /// Configuration
    pub config: SimulcastConfig,
    /// Active layers
    pub active_layers: Vec<SimulcastLayer>,
    /// Encoding targets for each layer
    pub encoding_targets: HashMap<LayerId, EncodingTarget>,
    /// Bandwidth allocator
    pub bandwidth_allocator: BandwidthAllocator,
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
```

### 3. SFU Implementation

#### 3.1 Layer Selection Logic
```rust
/// Selects appropriate simulcast layer for a subscriber
pub fn select_layer(
    available_layers: &[SimulcastLayer],
    subscriber_bandwidth: u32,
    subscriber_preferences: &SubscriberPreferences,
) -> LayerId {
    // Implementation logic to select the best layer based on:
    // - Available bandwidth
    // - Subscriber's display resolution
    // - Preference for quality vs. smoothness
    // - Current network conditions
}
```

#### 3.2 Dynamic Adaptation
```rust
/// Adapts layer selection based on changing conditions
pub async fn adapt_layer_selection(
    session_id: SessionId,
    track_id: TrackId,
    current_layer: LayerId,
    bandwidth_trend: BandwidthTrend,
    quality_metrics: QualityMetrics,
) -> LayerId {
    // Implementation logic for dynamic adaptation:
    // - Upgrade to higher layer when bandwidth increases
    // - Downgrade to lower layer when bandwidth decreases
    // - Consider stability to avoid frequent switching
    // - Apply hysteresis to prevent oscillation
}
```

## Feedback-Based Adaptation

### 1. Quality Metrics Collection

#### 1.1 Subscriber Metrics
```rust
/// Quality metrics reported by subscribers
pub struct SubscriberQualityMetrics {
    /// Session identifier
    pub session_id: SessionId,
    /// Track identifier
    pub track_id: TrackId,
    /// Current layer identifier
    pub current_layer: LayerId,
    /// Packet loss percentage
    pub packet_loss: f32,
    /// Frame drop rate
    pub frame_drops: f32,
    /// Estimated available bandwidth
    pub available_bandwidth: u32,
    /// Rendering resolution
    pub display_resolution: Resolution,
    /// Playback buffer level in milliseconds
    pub buffer_level_ms: u32,
}
```

#### 1.2 Aggregated Metrics
```rust
/// Aggregated quality metrics for a track
pub struct TrackQualityMetrics {
    /// Track identifier
    pub track_id: TrackId,
    /// Publisher session identifier
    pub publisher_id: SessionId,
    /// Per-layer metrics
    pub layer_metrics: HashMap<LayerId, LayerMetrics>,
    /// Overall track health score (0-100)
    pub health_score: u8,
}

/// Metrics for a specific layer
pub struct LayerMetrics {
    /// Layer identifier
    pub layer_id: LayerId,
    /// Number of subscribers
    pub subscriber_count: u32,
    /// Average packet loss across subscribers
    pub avg_packet_loss: f32,
    /// Average frame drop rate
    pub avg_frame_drops: f32,
    /// Subscriber satisfaction score (0-100)
    pub satisfaction_score: u8,
}
```

### 2. Adaptation Algorithms

#### 2.1 Bandwidth-Based Adaptation
- Monitor available bandwidth for each subscriber
- Track bandwidth trends over time
- Apply moving average to smooth fluctuations
- Predict future bandwidth based on trends
- Select layer that fits within available bandwidth with safety margin

#### 2.2 Quality-Based Adaptation
- Monitor quality metrics (packet loss, frame drops)
- Detect quality degradation patterns
- Switch to lower layer when quality issues persist
- Gradually probe higher layers when quality is good
- Balance between quality and stability

#### 2.3 Hybrid Adaptation
- Combine bandwidth and quality metrics
- Weight factors based on application requirements
- Consider user preferences (quality vs. smoothness)
- Apply machine learning for optimal decision making
- Adapt to different network environments

## Implementation with Iroh and QUIC

### 1. Layer Identification

#### 1.1 RTP Header Extensions
```rust
/// RTP header extension for simulcast layer identification
pub struct SimulcastLayerExtension {
    /// Spatial layer identifier
    pub spatial_id: u8,
    /// Temporal layer identifier
    pub temporal_id: u8,
    /// Layer switching point indicator
    pub switching_point: bool,
}
```

#### 1.2 Stream Multiplexing
- Use separate QUIC streams for different spatial layers
- Implement priority-based scheduling for streams
- Ensure critical layers receive bandwidth priority
- Allow independent flow control per layer

### 2. Feedback Integration

#### 2.1 Layer Switching Requests
```rust
/// Request to switch to a different simulcast layer
pub struct LayerSwitchRequest {
    /// Track identifier
    pub track_id: TrackId,
    /// Requested spatial layer
    pub spatial_id: u8,
    /// Requested temporal layer
    pub temporal_id: u8,
    /// Priority of the request
    pub priority: RequestPriority,
    /// Reason for the switch
    pub reason: SwitchReason,
}
```

#### 2.2 Layer Availability Updates
```rust
/// Update about available simulcast layers
pub struct LayerAvailabilityUpdate {
    /// Track identifier
    pub track_id: TrackId,
    /// Available layers with their properties
    pub available_layers: Vec<SimulcastLayerInfo>,
    /// Currently recommended layer
    pub recommended_layer: LayerId,
}

/// Information about a simulcast layer
pub struct SimulcastLayerInfo {
    /// Layer identifier
    pub layer_id: LayerId,
    /// Spatial layer index
    pub spatial_id: u8,
    /// Temporal layer index
    pub temporal_id: u8,
    /// Resolution
    pub resolution: Resolution,
    /// Frame rate
    pub framerate: f32,
    /// Current bitrate
    pub bitrate: u32,
    /// Active status
    pub active: bool,
}
```

## Optimization Techniques

### 1. Bandwidth Efficiency

#### 1.1 Selective Layer Activation
- Only encode layers that have active subscribers
- Dynamically activate/deactivate layers based on demand
- Prioritize bandwidth for most-used layers
- Implement layer popularity tracking

#### 1.2 Bandwidth Distribution
```rust
/// Distributes available bandwidth across simulcast layers
pub fn distribute_bandwidth(
    available_bandwidth: u32,
    layer_demand: HashMap<LayerId, u32>,
    layer_priorities: HashMap<LayerId, u8>,
) -> HashMap<LayerId, u32> {
    // Implementation logic to:
    // - Allocate minimum bandwidth to all active layers
    // - Distribute remaining bandwidth based on priorities
    // - Ensure higher priority layers get sufficient bandwidth
    // - Optimize for overall quality across all subscribers
}
```

### 2. Computational Efficiency

#### 2.1 Encoding Optimization
- Reuse encoding information across layers when possible
- Implement efficient downscaling from highest resolution
- Balance CPU usage across layers
- Consider hardware acceleration for encoding

#### 2.2 SFU Processing
- Minimize packet inspection and manipulation
- Implement efficient layer switching
- Optimize feedback processing
- Use lock-free data structures for layer management

## Testing and Validation

### 1. Simulcast Performance Metrics
- Layer switching time
- Adaptation responsiveness
- Bandwidth utilization efficiency
- CPU usage per layer
- Quality consistency across layers

### 2. Network Condition Simulation
- Test under various bandwidth constraints
- Simulate network jitter and packet loss
- Validate adaptation behavior under stress
- Measure recovery time after network degradation

## Next Steps

1. Implement basic simulcast layer management
2. Develop feedback-based adaptation algorithms
3. Integrate with the SFU media router
4. Implement efficient layer switching
5. Optimize for various network conditions
6. Validate with real-world testing
