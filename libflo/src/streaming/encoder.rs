use crate::core::{ChannelData, FloResult, FrameType};
use crate::lossless::Encoder;
use crate::{compute_crc32, Reader, MAGIC};

pub struct StreamingEncoder {
    sample_rate: u32,
    channels: u8,
    bit_depth: u8,
    compression_level: u8,
    sample_buffer: Vec<f32>,
    samples_per_frame: usize,
    pending_frames: Vec<EncodedFrame>,
    encoder: Encoder,
    total_samples: u64,
    frame_index: u32,
}

/// An encoded frame ready for transmission
#[derive(Debug, Clone)]
pub struct EncodedFrame {
    /// Frame index
    pub index: u32,
    /// Timestamp in milliseconds
    pub timestamp_ms: u32,
    /// Encoded frame data
    pub data: Vec<u8>,
    /// Number of samples in this frame
    pub samples: u32,
}

impl StreamingEncoder {
    /// Create a new streaming encoder
    pub fn new(sample_rate: u32, channels: u8, bit_depth: u8) -> Self {
        let samples_per_frame = sample_rate as usize;

        Self {
            sample_rate,
            channels,
            bit_depth,
            compression_level: 5,
            sample_buffer: Vec::with_capacity(samples_per_frame * channels as usize * 2),
            samples_per_frame,
            pending_frames: Vec::new(),
            encoder: Encoder::new(sample_rate, channels, bit_depth),
            total_samples: 0,
            frame_index: 0,
        }
    }

    /// Set compression level (0-9)
    pub fn with_compression(mut self, level: u8) -> Self {
        self.compression_level = level.min(9);
        self.encoder =
            Encoder::new(self.sample_rate, self.channels, self.bit_depth).with_compression(level);
        self
    }

    /// Get number of pending samples in buffer
    pub fn pending_samples(&self) -> usize {
        self.sample_buffer.len() / self.channels as usize
    }

    /// Get number of encoded frames ready
    pub fn pending_frames(&self) -> usize {
        self.pending_frames.len()
    }

    /// Push samples to the encoder
    ///
    /// Samples should be interleaved if multi-channel
    pub fn push_samples(&mut self, samples: &[f32]) -> FloResult<()> {
        self.sample_buffer.extend_from_slice(samples);
        self.try_encode_frames()?;
        Ok(())
    }

    /// Get next encoded frame if available
    pub fn next_frame(&mut self) -> Option<EncodedFrame> {
        if self.pending_frames.is_empty() {
            None
        } else {
            Some(self.pending_frames.remove(0))
        }
    }

    /// Flush remaining samples (may produce a partial frame)
    pub fn flush(&mut self) -> FloResult<Option<EncodedFrame>> {
        if self.sample_buffer.is_empty() {
            return Ok(None);
        }

        let samples_per_channel = self.sample_buffer.len() / self.channels as usize;
        let timestamp_ms = (self.total_samples as f64 / self.sample_rate as f64 * 1000.0) as u32;

        let frame_data = self.encode_frame_data(&self.sample_buffer)?;

        let encoded = EncodedFrame {
            index: self.frame_index,
            timestamp_ms,
            data: frame_data,
            samples: samples_per_channel as u32,
        };

        self.total_samples += samples_per_channel as u64;
        self.frame_index += 1;
        self.sample_buffer.clear();

        Ok(Some(encoded))
    }

    /// Build a complete flo™ file from accumulated frames
    pub fn finalize(&mut self, metadata: &[u8]) -> FloResult<Vec<u8>> {
        if let Some(frame) = self.flush()? {
            self.pending_frames.push(frame);
        }

        // Build TOC
        let mut toc_data = Vec::new();
        let num_frames = self.pending_frames.len() as u32;
        toc_data.extend_from_slice(&num_frames.to_le_bytes());

        let mut byte_offset: u64 = 0;
        for frame in &self.pending_frames {
            toc_data.extend_from_slice(&frame.index.to_le_bytes());
            toc_data.extend_from_slice(&byte_offset.to_le_bytes());
            toc_data.extend_from_slice(&(frame.data.len() as u32).to_le_bytes());
            toc_data.extend_from_slice(&frame.timestamp_ms.to_le_bytes());
            byte_offset += frame.data.len() as u64;
        }

        // Build DATA
        let mut data_chunk = Vec::new();
        for frame in &self.pending_frames {
            data_chunk.extend_from_slice(&frame.data);
        }

        let data_crc32 = compute_crc32(&data_chunk);

        let header_size: u64 = 66;
        let toc_size = toc_data.len() as u64;
        let data_size = data_chunk.len() as u64;
        let extra_size: u64 = 0;
        let meta_size = metadata.len() as u64;

        let mut output = Vec::new();

        // Magic
        output.extend_from_slice(&MAGIC);

        // Header
        output.push(1); // version_major
        output.push(1); // version_minor
        output.extend_from_slice(&0u16.to_le_bytes()); // flags
        output.extend_from_slice(&self.sample_rate.to_le_bytes());
        output.push(self.channels);
        output.push(self.bit_depth);
        output.extend_from_slice(&(self.pending_frames.len() as u64).to_le_bytes());
        output.push(self.compression_level);
        output.extend_from_slice(&[0u8; 3]); // reserved
        output.extend_from_slice(&data_crc32.to_le_bytes());
        output.extend_from_slice(&header_size.to_le_bytes());
        output.extend_from_slice(&toc_size.to_le_bytes());
        output.extend_from_slice(&data_size.to_le_bytes());
        output.extend_from_slice(&extra_size.to_le_bytes());
        output.extend_from_slice(&meta_size.to_le_bytes());

        // TOC
        output.extend_from_slice(&toc_data);

        // DATA
        output.extend_from_slice(&data_chunk);

        // META
        output.extend_from_slice(metadata);

        self.pending_frames.clear();

        Ok(output)
    }

    // ========================================================================
    // Internal methods
    // ========================================================================

    fn try_encode_frames(&mut self) -> FloResult<()> {
        let frame_samples = self.samples_per_frame * self.channels as usize;

        while self.sample_buffer.len() >= frame_samples {
            let frame_data: Vec<f32> = self.sample_buffer.drain(..frame_samples).collect();
            let timestamp_ms =
                (self.total_samples as f64 / self.sample_rate as f64 * 1000.0) as u32;

            let encoded_data = self.encode_frame_data(&frame_data)?;

            self.pending_frames.push(EncodedFrame {
                index: self.frame_index,
                timestamp_ms,
                data: encoded_data,
                samples: self.samples_per_frame as u32,
            });

            self.total_samples += self.samples_per_frame as u64;
            self.frame_index += 1;
        }

        Ok(())
    }

    fn encode_frame_data(&self, samples: &[f32]) -> FloResult<Vec<u8>> {
        let temp_flo = self.encoder.encode(samples, &[])?;

        let reader = Reader::new();
        let file = reader.read(&temp_flo)?;

        if file.frames.is_empty() {
            return Err("No frames encoded".to_string());
        }

        let frame = &file.frames[0];
        let mut data = Vec::new();

        // Frame header
        data.push(frame.frame_type);
        data.extend_from_slice(&frame.frame_samples.to_le_bytes());
        data.push(frame.flags);

        // Channel data
        for ch in &frame.channels {
            let ch_data = self.serialize_channel(ch, FrameType::from(frame.frame_type));
            data.extend_from_slice(&(ch_data.len() as u32).to_le_bytes());
            data.extend_from_slice(&ch_data);
        }

        Ok(data)
    }

    fn serialize_channel(&self, ch: &ChannelData, frame_type: FrameType) -> Vec<u8> {
        match frame_type {
            FrameType::Silence => vec![],
            FrameType::Raw | FrameType::Transform => ch.residuals.clone(),
            _ => {
                let mut data = Vec::new();
                data.push(ch.rice_parameter);
                for &coeff in &ch.predictor_coeffs {
                    data.extend_from_slice(&coeff.to_le_bytes());
                }
                data.extend_from_slice(&ch.residuals);
                data
            }
        }
    }
}
