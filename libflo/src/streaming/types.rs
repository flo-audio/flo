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
    /// Total samples (actual sample count)
    pub total_samples: u64,
    /// Is lossy encoding
    pub is_lossy: bool,
}

impl StreamingAudioInfo {
    /// Calculate duration in seconds
    pub fn duration_secs(&self) -> f64 {
        self.total_samples as f64 / self.sample_rate as f64
    }

    /// Get total samples per channel
    pub fn total_samples_per_channel(&self) -> u64 {
        // `total_samples` is stored as samples per channel (number of sample-frames),
        // so return it directly.
        self.total_samples
    }
}
