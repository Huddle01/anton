# SFU Architecture with Iroh Integration

## Overview

This document outlines the architecture for a Selective Forwarding Unit (SFU) implemented in Rust using the Iroh library for media transport over QUIC. The SFU will enable efficient real-time media streaming between multiple participants by selectively forwarding media packets without transcoding.

## System Components

### 1. Core SFU Components

#### 1.1 Session Manager
- Manages participant sessions and their associated streams
- Handles participant join/leave events
- Maintains session state and participant information
- Coordinates media routing between participants

#### 1.2 Media Router
- Core component responsible for selective forwarding
- Receives media packets from publishers
- Determines which packets to forward to which subscribers
- Implements bandwidth adaptation and stream prioritization
- Handles packet forwarding based on participant subscriptions

#### 1.3 Bandwidth Manager
- Monitors available bandwidth for each participant
- Implements congestion control mechanisms
- Provides feedback to publishers for bitrate adaptation
- Prioritizes streams based on application requirements

#### 1.4 Statistics Collector
- Gathers metrics on media quality, latency, and packet loss
- Provides insights for debugging and optimization
- Exposes metrics through an API for monitoring

### 2. Iroh Integration Components

#### 2.1 Connection Manager
- Utilizes Iroh's Endpoint for connection establishment
- Manages QUIC connections between participants and the SFU
- Handles connection lifecycle (establishment, maintenance, termination)
- Leverages Iroh's hole-punching and relay capabilities

#### 2.2 Media Transport
- Uses iroh-roq for RTP over QUIC transport
- Manages media streams using QUIC's stream multiplexing
- Handles RTP packetization and depacketization
- Implements flow control for media streams

#### 2.3 Signaling Protocol
- Defines protocol for session establishment and negotiation
- Handles media capabilities exchange
- Manages subscription requests and updates
- Implements using Iroh's Router for protocol handling

## Data Flow

### 1. Publisher Flow
1. Publisher connects to SFU using Iroh Endpoint
2. Session is established with capability negotiation
3. Publisher creates MediaTrack(s) for audio/video
4. RTP packets are sent over QUIC streams to the SFU
5. SFU acknowledges receipt and provides feedback

### 2. Subscriber Flow
1. Subscriber connects to SFU using Iroh Endpoint
2. Session is established with capability negotiation
3. Subscriber requests available streams from the SFU
4. SFU forwards selected RTP packets from publishers to subscriber
5. Subscriber provides feedback for quality adaptation

### 3. Selective Forwarding Logic
1. SFU receives RTP packets from publishers
2. Media Router determines forwarding decisions based on:
   - Subscriber preferences and capabilities
   - Available bandwidth
   - Stream priorities
   - Quality requirements
3. Selected packets are forwarded to appropriate subscribers
4. Bandwidth Manager adjusts forwarding decisions based on network conditions

## Interfaces

### 1. External Interfaces

#### 1.1 Client API
```rust
// Connect to SFU
async fn connect(server_addr: NodeAddr) -> Result<SfuConnection>;

// Publish media track
async fn publish_track(track: MediaTrack) -> Result<TrackId>;

// Subscribe to media track
async fn subscribe_to_track(track_id: TrackId) -> Result<MediaTrack>;

// Unsubscribe from media track
async fn unsubscribe_from_track(track_id: TrackId) -> Result<()>;
```

#### 1.2 Admin API
```rust
// Get session statistics
async fn get_statistics() -> Result<SessionStats>;

// Configure bandwidth limits
async fn set_bandwidth_limits(limits: BandwidthLimits) -> Result<()>;

// Configure stream priorities
async fn set_stream_priorities(priorities: StreamPriorities) -> Result<()>;
```

### 2. Internal Interfaces

#### 2.1 Media Router Interface
```rust
// Register publisher track
async fn register_publisher_track(session_id: SessionId, track: MediaTrack) -> Result<TrackId>;

// Register subscriber
async fn register_subscriber(session_id: SessionId, track_id: TrackId) -> Result<()>;

// Process media packet
async fn process_media_packet(track_id: TrackId, packet: RtpPacket) -> Result<()>;
```

#### 2.2 Bandwidth Manager Interface
```rust
// Update available bandwidth for session
async fn update_bandwidth(session_id: SessionId, bandwidth: Bandwidth) -> Result<()>;

// Get recommended bitrate for publisher
async fn get_recommended_bitrate(session_id: SessionId, track_id: TrackId) -> Result<Bitrate>;
```

## Implementation Considerations

### 1. Performance Optimization
- Minimize packet copying during forwarding
- Efficient memory management for media packets
- Optimize RTP header processing
- Use lock-free data structures where possible

### 2. Scalability
- Design for horizontal scaling
- Implement efficient participant session management
- Optimize for large number of concurrent connections
- Consider cascading SFUs for very large deployments

### 3. Reliability
- Implement error handling and recovery mechanisms
- Handle network transitions and reconnections
- Provide fallback mechanisms for degraded network conditions
- Implement session persistence for recovery

### 4. Security
- Leverage Iroh's built-in encryption
- Implement authentication and authorization
- Protect against DoS attacks
- Secure signaling protocol

## Next Steps

1. Create detailed component specifications
2. Define data structures and interfaces
3. Implement proof-of-concept for core components
4. Develop testing strategy
5. Create project structure and dependencies
