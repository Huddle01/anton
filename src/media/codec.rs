// Media codec module for the SFU
//
// This module implements codec-specific functionality for audio and video.

use std::collections::HashMap;

use anyhow::Result;

use crate::SfuError;

/// Codec type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecType {
    /// Opus audio codec
    Opus,
    /// VP9 video codec
    VP9,
    /// H.264 video codec
    H264,
    /// AV1 video codec
    AV1,
}

impl CodecType {
    /// Get codec name
    pub fn name(&self) -> &'static str {
        match self {
            CodecType::Opus => "opus",
            CodecType::VP9 => "VP9",
            CodecType::H264 => "H264",
            CodecType::AV1 => "AV1",
        }
    }
    
    /// Check if codec is audio
    pub fn is_audio(&self) -> bool {
        matches!(self, CodecType::Opus)
    }
    
    /// Check if codec is video
    pub fn is_video(&self) -> bool {
        matches!(self, CodecType::VP9 | CodecType::H264 | CodecType::AV1)
    }
    
    /// Get codec from name
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "opus" => Some(CodecType::Opus),
            "vp9" => Some(CodecType::VP9),
            "h264" => Some(CodecType::H264),
            "av1" => Some(CodecType::AV1),
            _ => None,
        }
    }
}

/// Codec trait
pub trait Codec: Send + Sync {
    /// Get codec type
    fn codec_type(&self) -> CodecType;
    
    /// Get codec parameters
    fn parameters(&self) -> HashMap<String, String>;
    
    /// Check if codec is compatible with another codec
    fn is_compatible_with(&self, other: &dyn Codec) -> bool;
}

/// Opus audio codec
pub struct OpusCodec {
    /// Sample rate
    pub sample_rate: u32,
    /// Channels
    pub channels: u8,
    /// Use in-band FEC
    pub use_inband_fec: bool,
    /// Use DTX
    pub use_dtx: bool,
}

impl OpusCodec {
    /// Create a new Opus codec with default parameters
    pub fn new() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            use_inband_fec: true,
            use_dtx: true,
        }
    }
    
    /// Create a new Opus codec with custom parameters
    pub fn with_params(sample_rate: u32, channels: u8, use_inband_fec: bool, use_dtx: bool) -> Self {
        Self {
            sample_rate,
            channels,
            use_inband_fec,
            use_dtx,
        }
    }
}

impl Codec for OpusCodec {
    fn codec_type(&self) -> CodecType {
        CodecType::Opus
    }
    
    fn parameters(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("rate".to_string(), self.sample_rate.to_string());
        params.insert("channels".to_string(), self.channels.to_string());
        params.insert("useinbandfec".to_string(), if self.use_inband_fec { "1" } else { "0" }.to_string());
        params.insert("usedtx".to_string(), if self.use_dtx { "1" } else { "0" }.to_string());
        params
    }
    
    fn is_compatible_with(&self, other: &dyn Codec) -> bool {
        if other.codec_type() != CodecType::Opus {
            return false;
        }
        
        // Opus codecs are compatible if they have the same sample rate and channels
        let other_params = other.parameters();
        
        if let Some(rate) = other_params.get("rate") {
            if rate != &self.sample_rate.to_string() {
                return false;
            }
        }
        
        if let Some(channels) = other_params.get("channels") {
            if channels != &self.channels.to_string() {
                return false;
            }
        }
        
        true
    }
}

/// VP9 video codec
pub struct VP9Codec {
    /// Profile ID
    pub profile_id: u8,
    /// Support for simulcast
    pub simulcast: bool,
}

impl VP9Codec {
    /// Create a new VP9 codec with default parameters
    pub fn new() -> Self {
        Self {
            profile_id: 0,
            simulcast: true,
        }
    }
    
    /// Create a new VP9 codec with custom parameters
    pub fn with_params(profile_id: u8, simulcast: bool) -> Self {
        Self {
            profile_id,
            simulcast,
        }
    }
}

impl Codec for VP9Codec {
    fn codec_type(&self) -> CodecType {
        CodecType::VP9
    }
    
    fn parameters(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();
        params.insert("profile-id".to_string(), self.profile_id.to_string());
        params.insert("simulcast".to_string(), if self.simulcast { "1" } else { "0" }.to_string());
        params
    }
    
    fn is_compatible_with(&self, other: &dyn Codec) -> bool {
        if other.codec_type() != CodecType::VP9 {
            return false;
        }
        
        // VP9 codecs are compatible if they have the same profile ID
        let other_params = other.parameters();
        
        if let Some(profile_id) = other_params.get("profile-id") {
            if profile_id != &self.profile_id.to_string() {
                return false;
            }
        }
        
        true
    }
}

/// Codec factory
pub struct CodecFactory;

impl CodecFactory {
    /// Create a codec from type
    pub fn create_codec(codec_type: CodecType) -> Result<Box<dyn Codec>> {
        match codec_type {
            CodecType::Opus => Ok(Box::new(OpusCodec::new())),
            CodecType::VP9 => Ok(Box::new(VP9Codec::new())),
            _ => Err(SfuError::Media(format!("Unsupported codec: {:?}", codec_type)).into()),
        }
    }
    
    /// Create a codec from name and parameters
    pub fn create_codec_with_params(name: &str, params: HashMap<String, String>) -> Result<Box<dyn Codec>> {
        let codec_type = CodecType::from_name(name)
            .ok_or_else(|| SfuError::Media(format!("Unknown codec: {}", name)))?;
        
        match codec_type {
            CodecType::Opus => {
                let sample_rate = params
                    .get("rate")
                    .map(|s| s.parse::<u32>().unwrap_or(48000))
                    .unwrap_or(48000);
                
                let channels = params
                    .get("channels")
                    .map(|s| s.parse::<u8>().unwrap_or(2))
                    .unwrap_or(2);
                
                let use_inband_fec = params
                    .get("useinbandfec")
                    .map(|s| s == "1")
                    .unwrap_or(true);
                
                let use_dtx = params
                    .get("usedtx")
                    .map(|s| s == "1")
                    .unwrap_or(true);
                
                Ok(Box::new(OpusCodec::with_params(
                    sample_rate,
                    channels,
                    use_inband_fec,
                    use_dtx,
                )))
            }
            CodecType::VP9 => {
                let profile_id = params
                    .get("profile-id")
                    .map(|s| s.parse::<u8>().unwrap_or(0))
                    .unwrap_or(0);
                
                let simulcast = params
                    .get("simulcast")
                    .map(|s| s == "1")
                    .unwrap_or(true);
                
                Ok(Box::new(VP9Codec::with_params(profile_id, simulcast)))
            }
            _ => Err(SfuError::Media(format!("Unsupported codec: {:?}", codec_type)).into()),
        }
    }
}
