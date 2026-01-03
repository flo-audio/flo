//! Streaming types and enums

/// Streaming decoder state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoderState {
    /// Waiting for header data
    WaitingForHeader,
    /// Header parsed, waiting for TOC
    WaitingForToc,
    /// Ready to decode frames
    Ready,
    /// End of stream reached
    Finished,
    /// Error state
    Error,
}

/// Audio information for streaming
#[derive(Debug, Clone)]
pub struct StreamingAudioInfo {
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u8,
    /// Bits per sample
    pub bit_depth: u8,
    /// Total frames (if known)
    pub total_frames: u64,
    /// Is lossy encoding
    pub is_lossy: bool,
}

impl StreamingAudioInfo {
    /// Calculate duration in seconds
    pub fn duration_secs(&self) -> f64 {
        self.total_frames as f64
    }

    /// Calculate samples per channel
    pub fn total_samples(&self) -> u64 {
        self.total_frames * self.sample_rate as u64
    }
}
