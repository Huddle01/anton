// Media frame module for the SFU
//
// This module handles media frame processing for audio and video.

use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;

use crate::{
    media::codec::{Codec, CodecType},
    SfuError,
};

/// Media frame types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    /// Audio frame
    Audio,
    /// Video key frame
    VideoKey,
    /// Video delta frame
    VideoDelta,
}

/// Media frame
pub struct MediaFrame {
    /// Frame type
    pub frame_type: FrameType,
    /// Codec type
    pub codec_type: CodecType,
    /// Frame data
    pub data: Bytes,
    /// Timestamp in media timebase
    pub timestamp: u32,
    /// Duration of the frame
    pub duration: Duration,
    /// Spatial layer index (for simulcast)
    pub spatial_layer: Option<u8>,
    /// Temporal layer index (for simulcast)
    pub temporal_layer: Option<u8>,
}

impl MediaFrame {
    /// Create a new audio frame
    pub fn new_audio(codec_type: CodecType, data: Bytes, timestamp: u32, duration: Duration) -> Result<Self> {
        if !codec_type.is_audio() {
            return Err(SfuError::Media(format!("Codec {:?} is not an audio codec", codec_type)).into());
        }
        
        Ok(Self {
            frame_type: FrameType::Audio,
            codec_type,
            data,
            timestamp,
            duration,
            spatial_layer: None,
            temporal_layer: None,
        })
    }
    
    /// Create a new video key frame
    pub fn new_video_key(
        codec_type: CodecType,
        data: Bytes,
        timestamp: u32,
        duration: Duration,
        spatial_layer: Option<u8>,
        temporal_layer: Option<u8>,
    ) -> Result<Self> {
        if !codec_type.is_video() {
            return Err(SfuError::Media(format!("Codec {:?} is not a video codec", codec_type)).into());
        }
        
        Ok(Self {
            frame_type: FrameType::VideoKey,
            codec_type,
            data,
            timestamp,
            duration,
            spatial_layer,
            temporal_layer,
        })
    }
    
    /// Create a new video delta frame
    pub fn new_video_delta(
        codec_type: CodecType,
        data: Bytes,
        timestamp: u32,
        duration: Duration,
        spatial_layer: Option<u8>,
        temporal_layer: Option<u8>,
    ) -> Result<Self> {
        if !codec_type.is_video() {
            return Err(SfuError::Media(format!("Codec {:?} is not a video codec", codec_type)).into());
        }
        
        Ok(Self {
            frame_type: FrameType::VideoDelta,
            codec_type,
            data,
            timestamp,
            duration,
            spatial_layer,
            temporal_layer,
        })
    }
    
    /// Check if this is an audio frame
    pub fn is_audio(&self) -> bool {
        self.frame_type == FrameType::Audio
    }
    
    /// Check if this is a video frame
    pub fn is_video(&self) -> bool {
        self.frame_type == FrameType::VideoKey || self.frame_type == FrameType::VideoDelta
    }
    
    /// Check if this is a key frame
    pub fn is_key_frame(&self) -> bool {
        self.frame_type == FrameType::VideoKey
    }
    
    /// Get the size of the frame in bytes
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Media frame processor
pub struct FrameProcessor {
    /// Codec for processing frames
    codec: Box<dyn Codec>,
}

impl FrameProcessor {
    /// Create a new frame processor
    pub fn new(codec: Box<dyn Codec>) -> Self {
        Self { codec }
    }
    
    /// Process a media frame
    pub fn process_frame(&self, frame: &mut MediaFrame) -> Result<()> {
        // This is a placeholder for frame processing logic
        // In a real implementation, this would apply transformations based on the codec
        
        match self.codec.codec_type() {
            CodecType::Opus => {
                // Process Opus audio frame
                // For example, adjust volume, apply filters, etc.
                Ok(())
            }
            CodecType::VP9 => {
                // Process VP9 video frame
                // For example, scale resolution, adjust quality, etc.
                Ok(())
            }
            _ => {
                Err(SfuError::Media(format!("Unsupported codec for frame processing: {:?}", self.codec.codec_type())).into())
            }
        }
    }
    
    /// Get the codec used by this processor
    pub fn codec(&self) -> &dyn Codec {
        self.codec.as_ref()
    }
}

/// Media frame queue
pub struct FrameQueue {
    /// Maximum queue size
    max_size: usize,
    /// Frames in the queue
    frames: Vec<MediaFrame>,
}

impl FrameQueue {
    /// Create a new frame queue
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            frames: Vec::with_capacity(max_size),
        }
    }
    
    /// Add a frame to the queue
    pub fn push(&mut self, frame: MediaFrame) -> Result<()> {
        if self.frames.len() >= self.max_size {
            // Remove oldest frame if queue is full
            self.frames.remove(0);
        }
        
        self.frames.push(frame);
        Ok(())
    }
    
    /// Get the next frame from the queue
    pub fn pop(&mut self) -> Option<MediaFrame> {
        if self.frames.is_empty() {
            None
        } else {
            Some(self.frames.remove(0))
        }
    }
    
    /// Peek at the next frame without removing it
    pub fn peek(&self) -> Option<&MediaFrame> {
        self.frames.first()
    }
    
    /// Get the number of frames in the queue
    pub fn len(&self) -> usize {
        self.frames.len()
    }
    
    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
    
    /// Clear the queue
    pub fn clear(&mut self) {
        self.frames.clear();
    }
}
