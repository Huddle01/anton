# Testing and Deployment Guidelines for Rust SFU with iroh

This document provides guidelines for testing and deploying the Selective Forwarding Unit (SFU) implemented in Rust using iroh for media over QUIC.

## Testing Guidelines

### Unit Testing

1. **Component Tests**
   - Test each component in isolation using mock dependencies
   - Focus on session management, media routing, and bandwidth adaptation
   - Use `tokio-test` for async testing

   ```rust
   #[tokio::test]
   async fn test_session_manager() {
       // Setup test environment
       let session_manager = DefaultSessionManager::new();
       
       // Create a test connection
       let node_id = iroh::NodeId::default();
       let connection = create_mock_connection();
       
       // Test session creation
       let session_id = session_manager.create_session(node_id, connection).await.unwrap();
       assert!(session_id > 0);
       
       // Test session retrieval
       let participant = session_manager.get_participant(session_id).await.unwrap();
       assert_eq!(participant.read().await.session_id, session_id);
       
       // Test session removal
       session_manager.remove_session(session_id).await.unwrap();
       assert!(session_manager.get_participant(session_id).await.is_err());
   }
   ```

2. **Codec Tests**
   - Test encoding and decoding of audio/video frames
   - Verify correct packetization and depacketization
   - Test with different codecs (Opus, VP9)

   ```rust
   #[test]
   fn test_opus_codec() {
       let codec = OpusCodec::new();
       assert_eq!(codec.codec_type(), CodecType::Opus);
       
       let params = codec.parameters();
       assert_eq!(params.get("rate"), Some(&"48000".to_string()));
   }
   
   #[test]
   fn test_rtp_packetization() {
       let mut packetizer = RtpPacketizer::new(CodecType::Opus, 111, 12345);
       let frame_data = vec![0; 100]; // Sample frame data
       let timestamp = 90000;
       
       let packets = packetizer.packetize(&frame_data, timestamp).unwrap();
       assert_eq!(packets.len(), 1); // Opus frames are sent as single packets
       
       let packet = &packets[0];
       assert_eq!(packet.header.payload_type, 111);
       assert_eq!(packet.header.ssrc, 12345);
       assert_eq!(packet.header.timestamp, timestamp);
   }
   ```

3. **Transport Tests**
   - Test QUIC connection establishment
   - Test media stream creation and management
   - Test bandwidth adaptation

   ```rust
   #[tokio::test]
   async fn test_quic_media_transport() {
       // Setup test environment
       let endpoint = create_test_endpoint().await;
       let transport = QuicMediaTransport::new();
       
       // Create a test connection
       let connection = create_test_connection(endpoint).await;
       
       // Test session creation
       let session = transport.create_session(&connection).await.unwrap();
       
       // Test media track sending
       let track = create_test_media_track();
       transport.send_track(&session, track).await.unwrap();
       
       // Cleanup
       drop(session);
       drop(transport);
   }
   ```

### Integration Testing

1. **End-to-End Tests**
   - Test complete media flow from publisher to subscribers
   - Verify correct forwarding of media packets
   - Test with multiple participants

   ```rust
   #[tokio::test]
   async fn test_end_to_end_media_flow() {
       // Setup SFU
       let endpoint = create_test_endpoint().await;
       let config = SfuConfig::default();
       let sfu = Sfu::new(endpoint, config).await.unwrap();
       sfu.start().await.unwrap();
       
       // Create publisher
       let publisher = create_test_client().await;
       let publisher_id = publisher.connect_to_sfu().await.unwrap();
       
       // Create subscribers
       let subscriber1 = create_test_client().await;
       let subscriber1_id = subscriber1.connect_to_sfu().await.unwrap();
       
       let subscriber2 = create_test_client().await;
       let subscriber2_id = subscriber2.connect_to_sfu().await.unwrap();
       
       // Publish a track
       let track_id = publisher.publish_track(CodecType::Opus).await.unwrap();
       
       // Subscribe to the track
       subscriber1.subscribe_to_track(publisher_id, track_id).await.unwrap();
       subscriber2.subscribe_to_track(publisher_id, track_id).await.unwrap();
       
       // Send test frames
       let test_frames = create_test_frames();
       for frame in test_frames {
           publisher.send_frame(track_id, frame).await.unwrap();
       }
       
       // Verify frames received by subscribers
       let received1 = subscriber1.receive_frames(track_id, 5, Duration::from_secs(5)).await;
       let received2 = subscriber2.receive_frames(track_id, 5, Duration::from_secs(5)).await;
       
       assert_eq!(received1.len(), 5);
       assert_eq!(received2.len(), 5);
       
       // Cleanup
       sfu.stop().await.unwrap();
   }
   ```

2. **Simulcast Tests**
   - Test simulcast layer switching
   - Verify correct adaptation to bandwidth changes
   - Test with different network conditions

   ```rust
   #[tokio::test]
   async fn test_simulcast_adaptation() {
       // Setup SFU with simulcast support
       let endpoint = create_test_endpoint().await;
       let config = SfuConfig {
           enable_simulcast: true,
           ..Default::default()
       };
       let sfu = Sfu::new(endpoint, config).await.unwrap();
       sfu.start().await.unwrap();
       
       // Create publisher with simulcast support
       let publisher = create_test_client().await;
       let publisher_id = publisher.connect_to_sfu().await.unwrap();
       
       // Create subscriber
       let subscriber = create_test_client().await;
       let subscriber_id = subscriber.connect_to_sfu().await.unwrap();
       
       // Publish a simulcast track
       let simulcast_config = SimulcastConfig {
           spatial_layers: 3,
           temporal_layers: 2,
           base_resolution: Resolution { width: 320, height: 240 },
           base_framerate: 15.0,
           spatial_scale_factor: 2.0,
           temporal_scale_factor: 1.5,
       };
       
       let track_id = publisher.publish_simulcast_track(
           CodecType::VP9, 
           simulcast_config
       ).await.unwrap();
       
       // Subscribe to the track
       subscriber.subscribe_to_track(publisher_id, track_id).await.unwrap();
       
       // Test bandwidth changes
       // 1. High bandwidth - should get highest quality
       subscriber.set_bandwidth(5_000_000).await; // 5 Mbps
       tokio::time::sleep(Duration::from_secs(2)).await;
       
       let stats1 = subscriber.get_track_stats(track_id).await.unwrap();
       assert_eq!(stats1.spatial_layer, Some(2)); // Highest spatial layer
       
       // 2. Low bandwidth - should get lowest quality
       subscriber.set_bandwidth(500_000).await; // 500 kbps
       tokio::time::sleep(Duration::from_secs(2)).await;
       
       let stats2 = subscriber.get_track_stats(track_id).await.unwrap();
       assert_eq!(stats2.spatial_layer, Some(0)); // Lowest spatial layer
       
       // Cleanup
       sfu.stop().await.unwrap();
   }
   ```

3. **Stress Tests**
   - Test with high number of participants
   - Test with high bitrate streams
   - Test long-running sessions

   ```rust
   #[tokio::test]
   async fn test_stress_many_participants() {
       // Setup SFU
       let endpoint = create_test_endpoint().await;
       let config = SfuConfig {
           max_participants: 100,
           ..Default::default()
       };
       let sfu = Sfu::new(endpoint, config).await.unwrap();
       sfu.start().await.unwrap();
       
       // Create one publisher
       let publisher = create_test_client().await;
       let publisher_id = publisher.connect_to_sfu().await.unwrap();
       
       // Publish a track
       let track_id = publisher.publish_track(CodecType::VP9).await.unwrap();
       
       // Create many subscribers
       const NUM_SUBSCRIBERS: usize = 50;
       let mut subscribers = Vec::with_capacity(NUM_SUBSCRIBERS);
       
       for _ in 0..NUM_SUBSCRIBERS {
           let subscriber = create_test_client().await;
           let subscriber_id = subscriber.connect_to_sfu().await.unwrap();
           subscriber.subscribe_to_track(publisher_id, track_id).await.unwrap();
           subscribers.push(subscriber);
       }
       
       // Send test frames
       for i in 0..100 {
           let frame = create_test_video_frame(i);
           publisher.send_frame(track_id, frame).await.unwrap();
           tokio::time::sleep(Duration::from_millis(33)).await; // ~30fps
       }
       
       // Verify all subscribers received frames
       for (i, subscriber) in subscribers.iter().enumerate() {
           let received = subscriber.receive_frames(track_id, 10, Duration::from_secs(5)).await;
           assert!(!received.is_empty(), "Subscriber {} received no frames", i);
       }
       
       // Cleanup
       sfu.stop().await.unwrap();
   }
   ```

### Performance Testing

1. **Latency Tests**
   - Measure end-to-end latency
   - Test with different network conditions
   - Compare with baseline measurements

   ```rust
   #[tokio::test]
   async fn test_latency() {
       // Setup SFU
       let endpoint = create_test_endpoint().await;
       let config = SfuConfig::default();
       let sfu = Sfu::new(endpoint, config).await.unwrap();
       sfu.start().await.unwrap();
       
       // Create publisher and subscriber
       let publisher = create_test_client().await;
       let publisher_id = publisher.connect_to_sfu().await.unwrap();
       
       let subscriber = create_test_client().await;
       let subscriber_id = subscriber.connect_to_sfu().await.unwrap();
       
       // Publish a track
       let track_id = publisher.publish_track(CodecType::Opus).await.unwrap();
       
       // Subscribe to the track
       subscriber.subscribe_to_track(publisher_id, track_id).await.unwrap();
       
       // Measure latency
       let mut latencies = Vec::new();
       
       for _ in 0..100 {
           let start = Instant::now();
           
           // Send frame with timestamp
           let frame = create_timestamped_frame();
           publisher.send_frame(track_id, frame.clone()).await.unwrap();
           
           // Receive frame
           let received = subscriber.receive_frame(track_id).await.unwrap();
           
           // Calculate latency
           let end = Instant::now();
           let latency = end.duration_since(start);
           latencies.push(latency);
           
           tokio::time::sleep(Duration::from_millis(20)).await;
       }
       
       // Calculate statistics
       let avg_latency = calculate_average_duration(&latencies);
       let max_latency = latencies.iter().max().unwrap();
       let min_latency = latencies.iter().min().unwrap();
       
       println!("Average latency: {:?}", avg_latency);
       println!("Maximum latency: {:?}", max_latency);
       println!("Minimum latency: {:?}", min_latency);
       
       // Assert reasonable latency
       assert!(avg_latency < Duration::from_millis(100));
       
       // Cleanup
       sfu.stop().await.unwrap();
   }
   ```

2. **Throughput Tests**
   - Measure maximum throughput
   - Test with different codecs and bitrates
   - Identify bottlenecks

   ```rust
   #[tokio::test]
   async fn test_throughput() {
       // Setup SFU
       let endpoint = create_test_endpoint().await;
       let config = SfuConfig::default();
       let sfu = Sfu::new(endpoint, config).await.unwrap();
       sfu.start().await.unwrap();
       
       // Create publisher and multiple subscribers
       let publisher = create_test_client().await;
       let publisher_id = publisher.connect_to_sfu().await.unwrap();
       
       const NUM_SUBSCRIBERS: usize = 10;
       let mut subscribers = Vec::with_capacity(NUM_SUBSCRIBERS);
       
       for _ in 0..NUM_SUBSCRIBERS {
           let subscriber = create_test_client().await;
           let subscriber_id = subscriber.connect_to_sfu().await.unwrap();
           subscribers.push(subscriber);
       }
       
       // Publish a high-bitrate track
       let track_id = publisher.publish_track(CodecType::VP9).await.unwrap();
       publisher.set_track_bitrate(track_id, 5_000_000).await.unwrap(); // 5 Mbps
       
       // All subscribers subscribe to the track
       for subscriber in &subscribers {
           subscriber.subscribe_to_track(publisher_id, track_id).await.unwrap();
       }
       
       // Send high-bitrate frames for 10 seconds
       let start = Instant::now();
       let mut bytes_sent = 0;
       
       while start.elapsed() < Duration::from_secs(10) {
           let frame = create_large_video_frame();
           bytes_sent += frame.size();
           
           publisher.send_frame(track_id, frame).await.unwrap();
           
           tokio::time::sleep(Duration::from_millis(33)).await; // ~30fps
       }
       
       let duration = start.elapsed();
       let throughput_bps = (bytes_sent as f64 * 8.0) / duration.as_secs_f64();
       
       println!("Total bytes sent: {}", bytes_sent);
       println!("Duration: {:?}", duration);
       println!("Throughput: {:.2} Mbps", throughput_bps / 1_000_000.0);
       
       // Verify all subscribers received frames
       for (i, subscriber) in subscribers.iter().enumerate() {
           let received = subscriber.receive_frames(track_id, 10, Duration::from_secs(5)).await;
           assert!(!received.is_empty(), "Subscriber {} received no frames", i);
       }
       
       // Cleanup
       sfu.stop().await.unwrap();
   }
   ```

3. **Resource Usage Tests**
   - Monitor CPU and memory usage
   - Test scaling with number of participants
   - Identify resource bottlenecks

   ```rust
   #[tokio::test]
   async fn test_resource_usage() {
       // Setup SFU
       let endpoint = create_test_endpoint().await;
       let config = SfuConfig::default();
       let sfu = Sfu::new(endpoint, config).await.unwrap();
       sfu.start().await.unwrap();
       
       // Create one publisher
       let publisher = create_test_client().await;
       let publisher_id = publisher.connect_to_sfu().await.unwrap();
       
       // Publish a track
       let track_id = publisher.publish_track(CodecType::VP9).await.unwrap();
       
       // Start resource monitoring
       let (tx, rx) = mpsc::channel(100);
       let monitor_handle = tokio::spawn(async move {
           let mut measurements = Vec::new();
           let mut interval = tokio::time::interval(Duration::from_secs(1));
           
           loop {
               tokio::select! {
                   _ = interval.tick() => {
                       let cpu = measure_cpu_usage();
                       let memory = measure_memory_usage();
                       measurements.push((cpu, memory));
                   }
                   _ = rx.recv() => {
                       break;
                   }
               }
           }
           
           measurements
       });
       
       // Add subscribers gradually and measure resource usage
       let mut subscribers = Vec::new();
       
       for i in 0..50 {
           // Add a new subscriber
           let subscriber = create_test_client().await;
           let subscriber_id = subscriber.connect_to_sfu().await.unwrap();
           subscriber.subscribe_to_track(publisher_id, track_id).await.unwrap();
           subscribers.push(subscriber);
           
           // Send some frames
           for _ in 0..10 {
               let frame = create_test_video_frame(0);
               publisher.send_frame(track_id, frame).await.unwrap();
               tokio::time::sleep(Duration::from_millis(33)).await;
           }
           
           println!("Added subscriber {}", i + 1);
           tokio::time::sleep(Duration::from_secs(5)).await;
       }
       
       // Stop resource monitoring
       tx.send(()).await.unwrap();
       let measurements = monitor_handle.await.unwrap();
       
       // Analyze results
       let max_cpu = measurements.iter().map(|(cpu, _)| *cpu).fold(0.0, f64::max);
       let max_memory = measurements.iter().map(|(_, mem)| *mem).fold(0, usize::max);
       
       println!("Maximum CPU usage: {:.2}%", max_cpu);
       println!("Maximum memory usage: {} MB", max_memory / (1024 * 1024));
       
       // Plot resource usage graph
       plot_resource_usage(&measurements, "resource_usage.png");
       
       // Cleanup
       sfu.stop().await.unwrap();
   }
   ```

## Deployment Guidelines

### Local Deployment

1. **Development Environment**
   - Clone the repository
   - Install Rust and dependencies
   - Build the project

   ```bash
   # Clone the repository
   git clone https://github.com/yourusername/rust-sfu-iroh.git
   cd rust-sfu-iroh

   # Build the project
   cargo build --release
   ```

2. **Running the SFU**
   - Run the simple SFU example
   - Configure parameters as needed

   ```bash
   # Run the simple SFU example
   cargo run --release --example simple-sfu -- --listen-addr 0.0.0.0:8080 --max-participants 100 --max-bitrate 5 --enable-simulcast --enable-feedback
   ```

3. **Testing Locally**
   - Use the provided test clients
   - Monitor logs and performance

   ```bash
   # Run a test publisher
   cargo run --release --example test-publisher -- --sfu-addr 127.0.0.1:8080 --codec opus

   # Run a test subscriber
   cargo run --release --example test-subscriber -- --sfu-addr 127.0.0.1:8080 --publisher-id <publisher_id> --track-id <track_id>
   ```

### Production Deployment

1. **Server Requirements**
   - Recommended hardware specifications:
     - CPU: 4+ cores
     - RAM: 8+ GB
     - Network: 1 Gbps+ connection
   - Operating system: Linux (Ubuntu 20.04 LTS or newer recommended)
   - Firewall configuration: Allow UDP traffic on the SFU port

2. **Docker Deployment**
   - Use the provided Dockerfile
   - Configure environment variables

   ```dockerfile
   # Dockerfile
   FROM rust:1.70 as builder
   WORKDIR /usr/src/app
   COPY . .
   RUN cargo build --release

   FROM debian:bullseye-slim
   RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
   COPY --from=builder /usr/src/app/target/release/simple-sfu /usr/local/bin/
   
   EXPOSE 8080/udp
   
   CMD ["simple-sfu", "--listen-addr", "0.0.0.0:8080"]
   ```

   ```bash
   # Build and run the Docker container
   docker build -t rust-sfu-iroh .
   docker run -p 8080:8080/udp -e MAX_PARTICIPANTS=100 -e MAX_BITRATE=5 -e ENABLE_SIMULCAST=true rust-sfu-iroh
   ```

3. **Kubernetes Deployment**
   - Use the provided Kubernetes manifests
   - Scale horizontally as needed

   ```yaml
   # kubernetes/deployment.yaml
   apiVersion: apps/v1
   kind: Deployment
   metadata:
     name: rust-sfu-iroh
   spec:
     replicas: 3
     selector:
       matchLabels:
         app: rust-sfu-iroh
     template:
       metadata:
         labels:
           app: rust-sfu-iroh
       spec:
         containers:
         - name: rust-sfu-iroh
           image: rust-sfu-iroh:latest
           ports:
           - containerPort: 8080
             protocol: UDP
           env:
           - name: MAX_PARTICIPANTS
             value: "100"
           - name: MAX_BITRATE
             value: "5"
           - name: ENABLE_SIMULCAST
             value: "true"
           resources:
             requests:
               cpu: "1"
               memory: "1Gi"
             limits:
               cpu: "2"
               memory: "2Gi"
   ```

   ```yaml
   # kubernetes/service.yaml
   apiVersion: v1
   kind: Service
   metadata:
     name: rust-sfu-iroh
   spec:
     selector:
       app: rust-sfu-iroh
     ports:
     - port: 8080
       protocol: UDP
       targetPort: 8080
     type: LoadBalancer
   ```

   ```bash
   # Apply Kubernetes manifests
   kubectl apply -f kubernetes/deployment.yaml
   kubectl apply -f kubernetes/service.yaml
   ```

4. **Monitoring and Logging**
   - Use Prometheus for metrics
   - Use Grafana for visualization
   - Configure log aggregation

   ```yaml
   # prometheus.yaml
   scrape_configs:
     - job_name: 'rust-sfu-iroh'
       static_configs:
         - targets: ['rust-sfu-iroh:9090']
   ```

   ```bash
   # Enable metrics in the SFU
   cargo run --release --example simple-sfu -- --listen-addr 0.0.0.0:8080 --metrics-addr 0.0.0.0:9090
   ```

5. **Load Balancing**
   - Use a UDP-capable load balancer
   - Configure session affinity
   - Consider geographic distribution

   ```
   # Example HAProxy configuration
   frontend sfu_frontend
     bind *:8080 udp
     default_backend sfu_backend

   backend sfu_backend
     mode udp
     balance source
     server sfu1 sfu1.example.com:8080 check
     server sfu2 sfu2.example.com:8080 check
     server sfu3 sfu3.example.com:8080 check
   ```

### Security Considerations

1. **Authentication and Authorization**
   - Implement token-based authentication
   - Use HTTPS for signaling
   - Validate participant permissions

2. **Media Encryption**
   - QUIC provides transport-level encryption
   - Consider additional application-level encryption for sensitive use cases

3. **Rate Limiting and DoS Protection**
   - Implement connection rate limiting
   - Monitor for abnormal traffic patterns
   - Use firewall rules to block malicious traffic

4. **Regular Updates**
   - Keep dependencies up to date
   - Apply security patches promptly
   - Monitor security advisories

## Performance Tuning

1. **Network Optimization**
   - Adjust QUIC parameters for your environment
   - Optimize buffer sizes
   - Consider using jumbo frames if supported

   ```rust
   // Example QUIC configuration
   let quic_config = quinn::ServerConfig::with_crypto(crypto);
   quic_config.transport = Arc::new({
       let mut transport = quinn::TransportConfig::default();
       transport.max_concurrent_uni_streams(1024_u32.into());
       transport.max_idle_timeout(Some(Duration::from_secs(30).try_into().unwrap()));
       transport.keep_alive_interval(Some(Duration::from_secs(10)));
       transport
   });
   ```

2. **CPU Optimization**
   - Use thread pinning for critical threads
   - Adjust tokio runtime configuration
   - Profile and optimize hot paths

   ```rust
   // Example tokio runtime configuration
   let runtime = tokio::runtime::Builder::new_multi_thread()
       .worker_threads(num_cpus::get())
       .enable_io()
       .enable_time()
       .build()
       .unwrap();
   ```

3. **Memory Optimization**
   - Adjust buffer pool sizes
   - Implement frame dropping policies
   - Monitor and tune GC parameters

   ```rust
   // Example buffer pool configuration
   let buffer_pool = BufferPool::new(1500, 1000); // 1500 bytes per buffer, 1000 buffers
   ```

4. **Codec Configuration**
   - Optimize codec parameters for your use case
   - Balance quality vs. bandwidth
   - Consider hardware acceleration if available

   ```rust
   // Example VP9 configuration
   let vp9_config = VP9Config {
       threads: num_cpus::get(),
       error_resilient: true,
       lag_in_frames: 0, // Low-latency mode
       quality: 85,
       ..Default::default()
   };
   ```

## Scaling Guidelines

1. **Vertical Scaling**
   - Increase CPU and memory resources
   - Monitor resource utilization
   - Identify bottlenecks

2. **Horizontal Scaling**
   - Deploy multiple SFU instances
   - Use load balancing
   - Implement session distribution

3. **Geographic Distribution**
   - Deploy SFUs in multiple regions
   - Route participants to the nearest SFU
   - Consider multi-region federation

4. **Capacity Planning**
   - Estimate resource requirements based on:
     - Number of participants
     - Media quality (resolution, framerate)
     - Network conditions
   - Plan for peak usage and redundancy

## Troubleshooting

1. **Common Issues**
   - Connection failures
   - Media quality problems
   - Performance degradation

2. **Diagnostic Tools**
   - Enable debug logging
   - Use network packet capture
   - Monitor system metrics

3. **Resolution Steps**
   - Check network connectivity
   - Verify codec compatibility
   - Adjust bandwidth parameters
   - Restart problematic components

## Conclusion

This SFU implementation using Rust and iroh provides a high-performance, scalable solution for real-time media streaming. By following these testing and deployment guidelines, you can ensure reliable operation and optimal performance in your environment.

For further assistance or to report issues, please open an issue on the GitHub repository.
