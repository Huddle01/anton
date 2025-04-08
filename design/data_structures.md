# SFU Data Structures

## Session Management

```rust
/// Unique identifier for a participant session
pub type SessionId = u64;

/// Unique identifier for a media track
pub type TrackId = u64;

/// Represents a participant in the SFU
pub struct Participant {
    /// Unique session identifier
    pub session_id: SessionId,
    /// Iroh node identifier
    pub node_id: NodeId,
    /// Connection to the participant
    pub connection: RtcConnection,
    /// Tracks published by this participant
    pub published_tracks: HashMap<TrackId, PublishedTrack>,
    /// Tracks subscribed to by this participant
    pub subscribed_tracks: HashMap<TrackId, SubscribedTrack>,
    /// Bandwidth information
    pub bandwidth: BandwidthInfo,
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
```

## Media Routing

```rust
/// Media packet with routing information
pub struct RoutableMediaPacket {
    /// Source track identifier
    pub track_id: TrackId,
    /// Publisher session identifier
    pub publisher_id: SessionId,
    /// RTP packet
    pub packet: RtpPacket,
    /// Packet priority
    pub priority: PacketPriority,
    /// Packet timestamp
    pub timestamp: Instant,
}

/// Priority levels for media packets
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
```

## Statistics and Monitoring

```rust
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
}
```

## Iroh Integration

```rust
/// SFU protocol handler for Iroh
pub struct SfuProtocol {
    /// Session manager
    pub session_manager: Arc<SessionManager>,
    /// Media router
    pub media_router: Arc<MediaRouter>,
    /// Bandwidth manager
    pub bandwidth_manager: Arc<BandwidthManager>,
    /// Statistics collector
    pub stats_collector: Arc<StatsCollector>,
}

/// RTP over QUIC session
pub struct RtpQuicSession {
    /// Iroh QUIC connection
    pub connection: Connection,
    /// RoQ session
    pub session: Session,
    /// Next flow identifier for receiving
    pub next_recv_flow_id: Arc<AtomicU32>,
    /// Next flow identifier for sending
    pub next_send_flow_id: Arc<AtomicU32>,
}

/// Media track sender over RTP/QUIC
pub struct RtpMediaTrackSender {
    /// Send flow
    pub send_flow: SendFlow,
    /// Media track
    pub track: MediaTrack,
    /// RTP packetizer
    pub packetizer: Packetizer,
}

/// Media track receiver over RTP/QUIC
pub struct RtpMediaTrackReceiver {
    /// Receive flow
    pub recv_flow: ReceiveFlow,
    /// Track information sender
    pub track_sender: broadcast::Sender<MediaFrame>,
    /// Sample builder for packet reordering
    pub sample_builder: SampleBuilder,
}
```
