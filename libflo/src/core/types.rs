//! common types for flo codec

// constants

/// Magic number "FLO!"
pub const MAGIC: [u8; 4] = [0x46, 0x4c, 0x4f, 0x21];

/// header size (excludes magic)
pub const HEADER_SIZE: u64 = 66;

/// format version
pub const VERSION_MAJOR: u8 = 1;
pub const VERSION_MINOR: u8 = 0;

// types

/// frame type
///
/// | Value | Type      | Description                    |
/// |-------|-----------|--------------------------------|
/// | 0     | Silence   | No audio data                  |
/// | 1-12  | ALPC      | LPC with order N               |
/// | 253   | Transform | MDCT-based lossy               |
/// | 254   | Raw       | Uncompressed PCM               |
/// | 255   | Reserved  | Future use                     |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FrameType {
    Silence = 0,
    Alpc1 = 1,
    Alpc2 = 2,
    Alpc3 = 3,
    Alpc4 = 4,
    Alpc5 = 5,
    Alpc6 = 6,
    Alpc7 = 7,
    Alpc8 = 8,
    Alpc9 = 9,
    Alpc10 = 10,
    Alpc11 = 11,
    Alpc12 = 12,
    Transform = 253,
    Raw = 254,
    Reserved = 255,
}

impl FrameType {
    /// lpc order (1-12) or None
    pub fn lpc_order(self) -> Option<usize> {
        let v = self as u8;
        if (1..=12).contains(&v) {
            Some(v as usize)
        } else {
            None
        }
    }

    /// is this alpc?
    pub fn is_alpc(self) -> bool {
        (1..=12).contains(&(self as u8))
    }

    /// is this transform/lossy?
    pub fn is_transform(self) -> bool {
        self == FrameType::Transform
    }

    /// make frametype from lpc order
    pub fn from_order(order: usize) -> Self {
        match order {
            1 => FrameType::Alpc1,
            2 => FrameType::Alpc2,
            3 => FrameType::Alpc3,
            4 => FrameType::Alpc4,
            5 => FrameType::Alpc5,
            6 => FrameType::Alpc6,
            7 => FrameType::Alpc7,
            8 => FrameType::Alpc8,
            9 => FrameType::Alpc9,
            10 => FrameType::Alpc10,
            11 => FrameType::Alpc11,
            12 => FrameType::Alpc12,
            _ => FrameType::Alpc8,
        }
    }
}

impl From<u8> for FrameType {
    fn from(v: u8) -> Self {
        match v {
            0 => FrameType::Silence,
            1 => FrameType::Alpc1,
            2 => FrameType::Alpc2,
            3 => FrameType::Alpc3,
            4 => FrameType::Alpc4,
            5 => FrameType::Alpc5,
            6 => FrameType::Alpc6,
            7 => FrameType::Alpc7,
            8 => FrameType::Alpc8,
            9 => FrameType::Alpc9,
            10 => FrameType::Alpc10,
            11 => FrameType::Alpc11,
            12 => FrameType::Alpc12,
            253 => FrameType::Transform,
            254 => FrameType::Raw,
            _ => FrameType::Reserved,
        }
    }
}

/// residual encoding method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ResidualEncoding {
    Rice = 0,
    Golomb = 1,
    Raw = 2,
}

impl From<u8> for ResidualEncoding {
    fn from(v: u8) -> Self {
        match v {
            0 => ResidualEncoding::Rice,
            1 => ResidualEncoding::Golomb,
            _ => ResidualEncoding::Raw,
        }
    }
}

// data structures

/// flo file header
#[derive(Debug, Clone)]
pub struct Header {
    pub version_major: u8,
    pub version_minor: u8,
    pub flags: u16,
    pub sample_rate: u32,
    pub channels: u8,
    pub bit_depth: u8,
    pub total_frames: u64,
    pub compression_level: u8,
    pub data_crc32: u32,
    pub header_size: u64,
    pub toc_size: u64,
    pub data_size: u64,
    pub extra_size: u64,
    pub meta_size: u64,
}

impl Default for Header {
    fn default() -> Self {
        Header {
            version_major: VERSION_MAJOR,
            version_minor: VERSION_MINOR,
            flags: 0,
            sample_rate: 44100,
            channels: 1,
            bit_depth: 16,
            total_frames: 0,
            compression_level: 5,
            data_crc32: 0,
            header_size: HEADER_SIZE,
            toc_size: 0,
            data_size: 0,
            extra_size: 0,
            meta_size: 0,
        }
    }
}

/// toc entry (20 bytes)
#[derive(Debug, Clone)]
pub struct TocEntry {
    pub frame_index: u32,
    pub byte_offset: u64,
    pub frame_size: u32,
    pub timestamp_ms: u32,
}

/// channel data within a frame
#[derive(Debug, Clone)]
pub struct ChannelData {
    pub predictor_coeffs: Vec<i32>,
    pub shift_bits: u8,
    pub residual_encoding: ResidualEncoding,
    pub rice_parameter: u8,
    pub residuals: Vec<u8>,
}

impl ChannelData {
    pub fn new_silence() -> Self {
        ChannelData {
            predictor_coeffs: vec![],
            shift_bits: 0,
            residual_encoding: ResidualEncoding::Rice,
            rice_parameter: 0,
            residuals: vec![],
        }
    }

    pub fn new_raw(data: Vec<u8>) -> Self {
        ChannelData {
            predictor_coeffs: vec![],
            shift_bits: 0,
            residual_encoding: ResidualEncoding::Raw,
            rice_parameter: 0,
            residuals: data,
        }
    }

    pub fn new_transform(data: Vec<u8>) -> Self {
        ChannelData {
            predictor_coeffs: vec![],
            shift_bits: 0,
            residual_encoding: ResidualEncoding::Rice,
            rice_parameter: 0,
            residuals: data,
        }
    }
}

/// audio frame (1 second)
#[derive(Debug, Clone)]
pub struct Frame {
    pub frame_type: u8,
    pub frame_samples: u32,
    pub flags: u8,
    pub channels: Vec<ChannelData>,
}

impl Frame {
    pub fn new(frame_type: u8, frame_samples: u32) -> Self {
        Frame {
            frame_type,
            frame_samples,
            flags: 0,
            channels: vec![],
        }
    }

    /// byte size of this frame
    pub fn byte_size(&self) -> usize {
        let mut size = 6; // header
        let frame_type = FrameType::from(self.frame_type);
        for ch in &self.channels {
            size += 4; // channel size prefix (u32)

            if frame_type.is_transform() {
                // just the serialized blob
                size += ch.residuals.len();
            } else if frame_type.is_alpc() {
                size += 1; // coefficient count (u8)
                size += ch.predictor_coeffs.len() * 4; // coeffs
                size += 1; // shift_bits
                size += 1; // residual_encoding
                if ch.residual_encoding == ResidualEncoding::Rice {
                    size += 1; // rice_parameter
                }
                size += ch.residuals.len();
            } else if frame_type == FrameType::Raw {
                size += ch.residuals.len();
            }
            // silence adds nothing
        }
        size
    }
}

/// complete decoded flo file
#[derive(Debug, Clone)]
pub struct FloFile {
    pub header: Header,
    pub toc: Vec<TocEntry>,
    pub frames: Vec<Frame>,
    pub extra: Vec<u8>,
    pub metadata: Vec<u8>,
}

/// result type for flo stuff
pub type FloResult<T> = Result<T, String>;
