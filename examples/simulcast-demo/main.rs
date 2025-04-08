// Example implementation of a simulcast demo
//
// This example demonstrates how to use the SFU library with simulcast support.

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use iroh::endpoint::{Endpoint, EndpointConfig};
use rust_sfu_iroh::{
    sfu::{Sfu, SfuConfig},
    simulcast::{SimulcastConfig, Resolution},
    init_logging,
};

/// Simulcast demo example
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Listen address for the SFU server
    #[clap(short, long, default_value = "0.0.0.0:8081")]
    listen_addr: String,
    
    /// Number of spatial layers
    #[clap(short, long, default_value = "3")]
    spatial_layers: u8,
    
    /// Number of temporal layers
    #[clap(short, long, default_value = "3")]
    temporal_layers: u8,
    
    /// Base resolution width
    #[clap(long, default_value = "320")]
    base_width: u32,
    
    /// Base resolution height
    #[clap(long, default_value = "240")]
    base_height: u32,
    
    /// Base framerate
    #[clap(long, default_value = "15.0")]
    base_framerate: f32,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    init_logging();
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Create iroh endpoint
    let endpoint_config = EndpointConfig::default();
    let endpoint = Endpoint::new(endpoint_config).await?;
    
    // Create simulcast configuration
    let simulcast_config = SimulcastConfig {
        spatial_layers: args.spatial_layers,
        temporal_layers: args.temporal_layers,
        base_resolution: Resolution {
            width: args.base_width,
            height: args.base_height,
        },
        base_framerate: args.base_framerate,
        spatial_scale_factor: 2.0,  // Each spatial layer doubles resolution
        temporal_scale_factor: 1.5, // Each temporal layer increases framerate by 1.5x
    };
    
    // Create SFU configuration
    let config = SfuConfig {
        max_participants: 50,
        max_bitrate_per_participant: 10_000_000, // 10 Mbps
        enable_simulcast: true,
        enable_feedback: true,
    };
    
    // Create SFU
    let sfu = Sfu::new(endpoint, config).await?;
    
    // Start SFU
    sfu.start().await?;
    
    println!("Simulcast demo server started on {}", args.listen_addr);
    println!("Simulcast configuration:");
    println!("  Spatial layers: {}", args.spatial_layers);
    println!("  Temporal layers: {}", args.temporal_layers);
    println!("  Base resolution: {}x{}", args.base_width, args.base_height);
    println!("  Base framerate: {}", args.base_framerate);
    println!("Press Ctrl+C to stop");
    
    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    
    // Stop SFU
    sfu.stop().await?;
    
    println!("Simulcast demo server stopped");
    
    Ok(())
}
