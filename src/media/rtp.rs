// RTP packetization module for the SFU
//
// This module handles RTP packet encoding and decoding for media transport.

use std::sync::Arc;

use anyhow::Result;
use bytes::{Bytes, BytesMut, BufMut};

use crate::{
    media::codec::{Codec, CodecType},
    SfuError,
};

/// RTP packet
pub struct RtpPacket {
    /// RTP header
    pub header: RtpHeader,
    /// Payload data
    pub payload: Bytes,
}

/// RTP header
pub struct RtpHeader {
    /// RTP version (2)
    pub version: u8,
    /// Padding flag
    pub padding: bool,
    /// Extension flag
    pub extension: bool,
    /// CSRC count
    pub csrc_count: u8,
    /// Marker bit
    pub marker: bool,
    /// Payload type
    pub payload_type: u8,
    /// Sequence number
    pub sequence_number: u16,
    /// Timestamp
    pub timestamp: u32,
    /// SSRC identifier
    pub ssrc: u32,
    /// CSRC identifiers
    pub csrc: Vec<u32>,
    /// Header extension
    pub extension_data: Option<RtpExtension>,
}

/// RTP extension
pub struct RtpExtension {
    /// Extension profile
    pub profile: u16,
    /// Extension data
    pub data: Bytes,
}

impl RtpPacket {
    /// Create a new RTP packet
    pub fn new(header: RtpHeader, payload: Bytes) -> Self {
        Self { header, payload }
    }
    
    /// Parse an RTP packet from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < 12 {
            return Err(SfuError::Media("RTP packet too short".to_string()).into());
        }
        
        let version = (data[0] >> 6) & 0x03;
        let padding = (data[0] >> 5) & 0x01 != 0;
        let extension = (data[0] >> 4) & 0x01 != 0;
        let csrc_count = data[0] & 0x0F;
        
        let marker = (data[1] >> 7) & 0x01 != 0;
        let payload_type = data[1] & 0x7F;
        
        let sequence_number = ((data[2] as u16) << 8) | (data[3] as u16);
        let timestamp = ((data[4] as u32) << 24) | ((data[5] as u32) << 16) | ((data[6] as u32) << 8) | (data[7] as u32);
        let ssrc = ((data[8] as u32) << 24) | ((data[9] as u32) << 16) | ((data[10] as u32) << 8) | (data[11] as u32);
        
        let mut offset = 12;
        
        // Parse CSRC identifiers
        let mut csrc = Vec::with_capacity(csrc_count as usize);
        for _ in 0..csrc_count {
            if offset + 4 > data.len() {
                return Err(SfuError::Media("RTP packet too short for CSRC".to_string()).into());
            }
            
            let csrc_id = ((data[offset] as u32) << 24) | ((data[offset + 1] as u32) << 16) | ((data[offset + 2] as u32) << 8) | (data[offset + 3] as u32);
            csrc.push(csrc_id);
            offset += 4;
        }
        
        // Parse extension
        let mut extension_data = None;
        if extension {
            if offset + 4 > data.len() {
                return Err(SfuError::Media("RTP packet too short for extension".to_string()).into());
            }
            
            let profile = ((data[offset] as u16) << 8) | (data[offset + 1] as u16);
            let length = ((data[offset + 2] as u16) << 8) | (data[offset + 3] as u16);
            offset += 4;
            
            let ext_size = length as usize * 4;
            if offset + ext_size > data.len() {
                return Err(SfuError::Media("RTP packet too short for extension data".to_string()).into());
            }
            
            let ext_data = Bytes::copy_from_slice(&data[offset..offset + ext_size]);
            extension_data = Some(RtpExtension {
                profile,
                data: ext_data,
            });
            
            offset += ext_size;
        }
        
        // Parse payload
        let payload = Bytes::copy_from_slice(&data[offset..]);
        
        let header = RtpHeader {
            version,
            padding,
            extension,
            csrc_count,
            marker,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
            csrc,
            extension_data,
        };
        
        Ok(Self::new(header, payload))
    }
    
    /// Serialize the RTP packet to bytes
    pub fn serialize(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(12 + self.header.csrc.len() * 4 + self.payload.len());
        
        // First byte: version, padding, extension, CSRC count
        let b0 = (self.header.version << 6) | 
                 ((self.header.padding as u8) << 5) | 
                 ((self.header.extension as u8) << 4) | 
                 (self.header.csrc_count & 0x0F);
        buf.put_u8(b0);
        
        // Second byte: marker, payload type
        let b1 = ((self.header.marker as u8) << 7) | (self.header.payload_type & 0x7F);
        buf.put_u8(b1);
        
        // Sequence number
        buf.put_u16(self.header.sequence_number);
        
        // Timestamp
        buf.put_u32(self.header.timestamp);
        
        // SSRC
        buf.put_u32(self.header.ssrc);
        
        // CSRC identifiers
        for csrc_id in &self.header.csrc {
            buf.put_u32(*csrc_id);
        }
        
        // Extension
        if let Some(ext) = &self.header.extension_data {
            buf.put_u16(ext.profile);
            buf.put_u16((ext.data.len() / 4) as u16);
            buf.put_slice(&ext.data);
        }
        
        // Payload
        buf.put_slice(&self.payload);
        
        buf.freeze()
    }
}

/// RTP packetizer for media frames
pub struct RtpPacketizer {
    /// Codec type
    codec_type: CodecType,
    /// Payload type
    payload_type: u8,
    /// SSRC identifier
    ssrc: u32,
    /// Sequence number
    sequence_number: u16,
    /// Timestamp
    timestamp: u32,
    /// Maximum payload size
    max_payload_size: usize,
}

impl RtpPacketizer {
    /// Create a new RTP packetizer
    pub fn new(codec_type: CodecType, payload_type: u8, ssrc: u32) -> Self {
        Self {
            codec_type,
            payload_type,
            ssrc,
            sequence_number: 0,
            timestamp: 0,
            max_payload_size: 1200, // Default to 1200 bytes for QUIC
        }
    }
    
    /// Set the maximum payload size
    pub fn set_max_payload_size(&mut self, size: usize) {
        self.max_payload_size = size;
    }
    
    /// Packetize a media frame into RTP packets
    pub fn packetize(&mut self, frame: &[u8], timestamp: u32) -> Result<Vec<RtpPacket>> {
        // Update timestamp
        self.timestamp = timestamp;
        
        // Split frame into packets
        let mut packets = Vec::new();
        
        match self.codec_type {
            CodecType::Opus => {
                // Opus frames are sent as single RTP packets
                let header = RtpHeader {
                    version: 2,
                    padding: false,
                    extension: false,
                    csrc_count: 0,
                    marker: true,
                    payload_type: self.payload_type,
                    sequence_number: self.sequence_number,
                    timestamp: self.timestamp,
                    ssrc: self.ssrc,
                    csrc: Vec::new(),
                    extension_data: None,
                };
                
                let packet = RtpPacket::new(header, Bytes::copy_from_slice(frame));
                packets.push(packet);
                
                // Increment sequence number
                self.sequence_number = self.sequence_number.wrapping_add(1);
            }
            CodecType::VP9 => {
                // VP9 frames may need to be split into multiple packets
                let mut offset = 0;
                let mut is_first = true;
                let mut is_last = false;
                
                while offset < frame.len() {
                    let remaining = frame.len() - offset;
                    let payload_size = remaining.min(self.max_payload_size);
                    is_last = offset + payload_size >= frame.len();
                    
                    // Create VP9 payload descriptor
                    // This is a simplified version; a real implementation would include more fields
                    let mut descriptor = BytesMut::with_capacity(1);
                    let descriptor_byte = ((is_first as u8) << 7) | ((is_last as u8) << 6);
                    descriptor.put_u8(descriptor_byte);
                    
                    // Create payload with descriptor and frame data
                    let mut payload = BytesMut::with_capacity(descriptor.len() + payload_size);
                    payload.put_slice(&descriptor);
                    payload.put_slice(&frame[offset..offset + payload_size]);
                    
                    // Create RTP header
                    let header = RtpHeader {
                        version: 2,
                        padding: false,
                        extension: false,
                        csrc_count: 0,
                        marker: is_last,
                        payload_type: self.payload_type,
                        sequence_number: self.sequence_number,
                        timestamp: self.timestamp,
                        ssrc: self.ssrc,
                        csrc: Vec::new(),
                        extension_data: None,
                    };
                    
                    // Create RTP packet
                    let packet = RtpPacket::new(header, payload.freeze());
                    packets.push(packet);
                    
                    // Update for next packet
                    offset += payload_size;
                    is_first = false;
                    self.sequence_number = self.sequence_number.wrapping_add(1);
                }
            }
            _ => {
                return Err(SfuError::Media(format!("Unsupported codec for packetization: {:?}", self.codec_type)).into());
            }
        }
        
        Ok(packets)
    }
}

/// RTP depacketizer for media frames
pub struct RtpDepacketizer {
    /// Codec type
    codec_type: CodecType,
    /// Expected sequence number
    expected_seq: u16,
    /// Packet buffer for reassembly
    packet_buffer: Vec<RtpPacket>,
}

impl RtpDepacketizer {
    /// Create a new RTP depacketizer
    pub fn new(codec_type: CodecType) -> Self {
        Self {
            codec_type,
            expected_seq: 0,
            packet_buffer: Vec::new(),
        }
    }
    
    /// Process an RTP packet and try to reassemble a media frame
    pub fn process_packet(&mut self, packet: RtpPacket) -> Result<Option<Vec<u8>>> {
        match self.codec_type {
            CodecType::Opus => {
                // Opus frames are contained in single RTP packets
                // Just extract the payload
                Ok(Some(packet.payload.to_vec()))
            }
            CodecType::VP9 => {
                // Check if this is the start of a new frame
                if packet.payload.len() > 0 && (packet.payload[0] & 0x80) != 0 {
                    // Clear buffer if we're starting a new frame
                    self.packet_buffer.clear();
                }
                
                // Add packet to buffer
                self.packet_buffer.push(packet);
                
                // Check if we have a complete frame
                if self.packet_buffer.last().map_or(false, |p| p.header.marker) {
                    // Reassemble frame
                    let mut frame = Vec::new();
                    
                    for packet in &self.packet_buffer {
                        if packet.payload.len() > 1 {
                            // Skip VP9 payload descriptor (simplified)
                            frame.extend_from_slice(&packet.payload[1..]);
                        }
                    }
                    
                    // Clear buffer
                    self.packet_buffer.clear();
                    
                    Ok(Some(frame))
                } else {
                    // Frame not complete yet
                    Ok(None)
                }
            }
            _ => {
                Err(SfuError::Media(format!("Unsupported codec for depacketization: {:?}", self.codec_type)).into())
            }
        }
    }
}
