//! Transform-based lossy encoder for flo™
//!
//! Combines MDCT, psychoacoustic model, quantization, and entropy coding
//! for high-quality lossy compression comparable to MP3/AAC/Vorbis.

pub mod decoder;
pub mod encoder;
pub mod mdct;
pub mod psychoacoustic;

// Re-export main types
pub use decoder::{deserialize_frame, deserialize_sparse, TransformDecoder};
pub use encoder::{serialize_frame, serialize_sparse, TransformEncoder, TransformFrame};
pub use mdct::{BlockSize, Mdct, WindowType};
pub use psychoacoustic::{PsychoacousticModel, BARK_BAND_EDGES, NUM_BARK_BANDS};

/// Quality presets for lossy encoding
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QualityPreset {
    /// Lowest quality, highest compression (~30:1)
    /// Good for speech, podcasts, low bandwidth
    Low,
    /// Medium quality, good compression (~10:1)
    /// Good for general music
    Medium,
    /// High quality, moderate compression (~6:1)
    /// Good for quality-conscious listening
    High,
    /// Very high quality, light compression (~4:1)
    /// Near-transparent for most content
    VeryHigh,
    /// Transparent quality, minimal loss (~3:1)
    /// Perceptually lossless for almost all content
    Transparent,
}

impl QualityPreset {
    /// Get the numeric quality value (0.0-1.0)
    pub fn as_f32(self) -> f32 {
        match self {
            QualityPreset::Low => 0.0,
            QualityPreset::Medium => 0.35,
            QualityPreset::High => 0.55,
            QualityPreset::VeryHigh => 0.75,
            QualityPreset::Transparent => 1.0,
        }
    }

    /// Create from numeric value
    pub fn from_f32(quality: f32) -> Self {
        if quality < 0.2 {
            QualityPreset::Low
        } else if quality < 0.45 {
            QualityPreset::Medium
        } else if quality < 0.65 {
            QualityPreset::High
        } else if quality < 0.85 {
            QualityPreset::VeryHigh
        } else {
            QualityPreset::Transparent
        }
    }

    /// Estimate compression ratio for this quality level
    pub fn expected_ratio(self) -> f32 {
        match self {
            QualityPreset::Low => 30.0,
            QualityPreset::Medium => 10.0,
            QualityPreset::High => 6.0,
            QualityPreset::VeryHigh => 4.0,
            QualityPreset::Transparent => 3.0,
        }
    }

    /// Estimate equivalent bitrate (kbps) for stereo 44.1kHz
    pub fn equivalent_bitrate(self) -> u32 {
        match self {
            QualityPreset::Low => 48,
            QualityPreset::Medium => 128,
            QualityPreset::High => 192,
            QualityPreset::VeryHigh => 256,
            QualityPreset::Transparent => 320,
        }
    }

    /// Create quality preset from target bitrate
    pub fn from_bitrate(bitrate_kbps: u32, sample_rate: u32, channels: u8) -> Self {
        // Calculate raw PCM bitrate
        let raw_kbps = (sample_rate as u64 * channels as u64 * 16) / 1000;
        let target_ratio = raw_kbps as f32 / bitrate_kbps as f32;

        if target_ratio > 20.0 {
            QualityPreset::Low
        } else if target_ratio > 10.0 {
            QualityPreset::Medium
        } else if target_ratio > 6.0 {
            QualityPreset::High
        } else if target_ratio > 4.0 {
            QualityPreset::VeryHigh
        } else {
            QualityPreset::Transparent
        }
    }
}

impl From<u8> for QualityPreset {
    fn from(v: u8) -> Self {
        match v {
            0 => QualityPreset::Low,
            1 => QualityPreset::Medium,
            2 => QualityPreset::High,
            3 => QualityPreset::VeryHigh,
            _ => QualityPreset::Transparent,
        }
    }
}

impl From<QualityPreset> for u8 {
    fn from(q: QualityPreset) -> u8 {
        match q {
            QualityPreset::Low => 0,
            QualityPreset::Medium => 1,
            QualityPreset::High => 2,
            QualityPreset::VeryHigh => 3,
            QualityPreset::Transparent => 4,
        }
    }
}
