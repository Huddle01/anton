// Example implementation of a simple SFU server
//
// This example demonstrates how to use the SFU library to create a simple server.

use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use iroh::endpoint::{Endpoint, EndpointConfig};
use rust_sfu_iroh::{
    sfu::{Sfu, SfuConfig},
    init_logging,
};

/// Simple SFU server example
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Listen address for the SFU server
    #[clap(short, long, default_value = "0.0.0.0:8080")]
    listen_addr: String,
    
    /// Maximum number of participants
    #[clap(short, long, default_value = "100")]
    max_participants: usize,
    
    /// Maximum bitrate per participant (in Mbps)
    #[clap(short, long, default_value = "5")]
    max_bitrate: u32,
    
    /// Enable simulcast
    #[clap(short, long)]
    enable_simulcast: bool,
    
    /// Enable feedback
    #[clap(short, long)]
    enable_feedback: bool,
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
    
    // Create SFU configuration
    let config = SfuConfig {
        max_participants: args.max_participants,
        max_bitrate_per_participant: args.max_bitrate * 1_000_000, // Convert Mbps to bps
        enable_simulcast: args.enable_simulcast,
        enable_feedback: args.enable_feedback,
    };
    
    // Create SFU
    let sfu = Sfu::new(endpoint, config).await?;
    
    // Start SFU
    sfu.start().await?;
    
    println!("SFU server started on {}", args.listen_addr);
    println!("Press Ctrl+C to stop");
    
    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;
    
    // Stop SFU
    sfu.stop().await?;
    
    println!("SFU server stopped");
    
    Ok(())
}
