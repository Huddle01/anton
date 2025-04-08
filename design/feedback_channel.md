# Feedback Channel Design for Media over QUIC

## Overview

This document outlines the design of a feedback channel system for our SFU implementation using media over QUIC with iroh. Similar to RTCP in WebRTC, this feedback mechanism is crucial for maintaining media quality, adapting to network conditions, and supporting advanced features like simulcast.

## Feedback Channel Architecture

### 1. Feedback Message Types

#### 1.1 Receiver Reports
- Packet loss statistics
- Jitter measurements
- Round-trip time calculations
- Timestamp information for synchronization
- Received bitrate metrics

#### 1.2 Sender Reports
- Sent packet statistics
- Timestamp information
- Encoding parameters
- Current bitrate information

#### 1.3 Simulcast Control Messages
- Layer switching requests
- Active layer notifications
- Layer quality metrics
- Spatial/temporal layer selection feedback

#### 1.4 Picture Loss Indication (PLI)
- Request for full frame refresh
- Lost frame notifications
- Decoder reset requests

#### 1.5 Bandwidth Estimation Messages
- Available bandwidth estimates
- Congestion notifications
- Bandwidth change recommendations

### 2. Transport Mechanism

#### 2.1 Dedicated QUIC Streams
- Use separate QUIC streams for feedback messages
- Prioritize feedback messages over regular data streams
- Ensure reliable delivery for critical feedback
- Option for unreliable delivery for time-sensitive feedback

#### 2.2 Message Framing
- Compact binary format for efficiency
- Versioned message structure for future extensions
- Batching of multiple feedback messages when appropriate
- Timestamp information for timing correlation

## Simulcast Support

### 1. Simulcast Layer Management

#### 1.1 Layer Description
```rust
/// Represents a simulcast layer
pub struct SimulcastLayer {
    /// Layer identifier
    pub layer_id: u8,
    /// Spatial resolution index (0 = lowest)
    pub spatial_id: u8,
    /// Temporal resolution index (0 = lowest)
    pub temporal_id: u8,
    /// Target bitrate for this layer
    pub target_bitrate: u32,
    /// Current active state
    pub active: bool,
}
```

#### 1.2 Layer Control Messages
```rust
/// Message to control simulcast layers
pub enum LayerControlMessage {
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
```

### 2. Simulcast Implementation

#### 2.1 Publisher Side
- Encode multiple quality levels of the same media
- Assign unique identifiers to each layer
- Send layer description information
- Adapt active layers based on feedback
- Implement bandwidth distribution across layers

#### 2.2 SFU Side
- Track available layers for each media stream
- Forward layer selection requests to publishers
- Make intelligent forwarding decisions based on:
  - Subscriber bandwidth
  - Subscriber preferences
  - Overall network conditions
  - Display size and capabilities
- Provide layer switching notifications to subscribers

#### 2.3 Subscriber Side
- Request specific layers based on:
  - Available bandwidth
  - Display requirements
  - User preferences
- Handle layer switching gracefully
- Provide quality feedback to the SFU

## Feedback Processing Flow

### 1. Subscriber to SFU
1. Subscriber monitors media quality metrics
2. Subscriber generates feedback messages at regular intervals
3. Critical feedback (like PLI) sent immediately
4. Feedback sent over dedicated QUIC stream
5. SFU processes feedback and updates routing decisions

### 2. SFU to Publisher
1. SFU aggregates feedback from multiple subscribers
2. SFU generates consolidated feedback for publishers
3. Layer switching requests sent based on subscriber needs
4. Bandwidth recommendations provided to publishers
5. Publishers adapt encoding parameters based on feedback

### 3. Feedback Timing
- Regular feedback interval (default: 100ms)
- Immediate feedback for critical events
- Rate limiting to prevent feedback implosion
- Adaptive timing based on network conditions

## Implementation Considerations

### 1. Efficiency
- Compact binary message format
- Batching of feedback messages
- Prioritization of critical feedback
- Efficient processing of feedback messages

### 2. Reliability
- Ensure delivery of critical feedback
- Handle lost feedback messages
- Implement feedback redundancy when necessary
- Fallback mechanisms for extreme conditions

### 3. Extensibility
- Versioned message format
- Extension points for future enhancements
- Backward compatibility considerations
- Custom feedback types for application-specific needs

## Integration with Iroh

### 1. Using Iroh QUIC Streams
```rust
/// Create feedback channel for a connection
pub async fn create_feedback_channel(conn: &RtcConnection) -> Result<FeedbackChannel> {
    // Open bidirectional QUIC stream for feedback
    let (send_stream, recv_stream) = conn.transport().open_bi().await?;
    
    // Create feedback channel
    let channel = FeedbackChannel {
        send_stream,
        recv_stream,
        next_sequence: AtomicU32::new(0),
    };
    
    Ok(channel)
}

/// Send feedback message
pub async fn send_feedback(&self, message: FeedbackMessage) -> Result<()> {
    // Serialize message
    let bytes = message.serialize()?;
    
    // Send over QUIC stream
    self.send_stream.write_all(&bytes).await?;
    
    Ok(())
}
```

### 2. Processing Feedback
```rust
/// Process incoming feedback
pub async fn process_feedback(
    router: &MediaRouter,
    channel: &FeedbackChannel
) -> Result<()> {
    let mut buffer = [0u8; 2048];
    
    loop {
        // Read feedback message
        let n = channel.recv_stream.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        
        // Deserialize message
        let message = FeedbackMessage::deserialize(&buffer[..n])?;
        
        // Process based on message type
        match message {
            FeedbackMessage::ReceiverReport(report) => {
                process_receiver_report(router, report).await?;
            }
            FeedbackMessage::SimulcastControl(control) => {
                process_simulcast_control(router, control).await?;
            }
            FeedbackMessage::PictureLossIndication(pli) => {
                process_pli(router, pli).await?;
            }
            // Handle other message types
            _ => {}
        }
    }
    
    Ok(())
}
```

## Next Steps

1. Define detailed message formats
2. Implement serialization/deserialization
3. Create feedback processing logic
4. Integrate with media router
5. Implement simulcast layer management
6. Test feedback channel under various network conditions
