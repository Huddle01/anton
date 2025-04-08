// Rust SFU with iroh for media over QUIC
//
// This library implements a Selective Forwarding Unit (SFU) for real-time media
// streaming using iroh as a transport layer over QUIC.

pub mod session;
pub mod media;
pub mod bandwidth;
pub mod stats;
pub mod connection;
pub mod transport;
pub mod signaling;
pub mod feedback;
pub mod simulcast;
pub mod sfu;

// Re-export commonly used types
pub use iroh;
pub use iroh_roq;

/// Error types for the SFU
pub mod error {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum SfuError {
        #[error("Session error: {0}")]
        Session(String),

        #[error("Media error: {0}")]
        Media(String),

        #[error("Connection error: {0}")]
        Connection(String),

        #[error("Transport error: {0}")]
        Transport(String),

        #[error("Signaling error: {0}")]
        Signaling(String),

        #[error("Iroh error: {0}")]
        Iroh(#[from] iroh::Error),

        #[error("IO error: {0}")]
        Io(#[from] std::io::Error),

        #[error("Other error: {0}")]
        Other(String),
    }

    pub type Result<T> = std::result::Result<T, SfuError>;
}

pub use error::{Result, SfuError};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize logging for the SFU
pub fn init_logging() {
    use tracing_subscriber::{fmt, EnvFilter};

    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .init();
}
