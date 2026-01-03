use crate::core::{crc32, FloResult, Frame, FrameType};
use crate::{ResidualEncoding, HEADER_SIZE, MAGIC, VERSION_MAJOR, VERSION_MINOR};

/// binary writer for flo format
pub struct Writer {
    buffer: Vec<u8>,
}

impl Writer {
    /// new writer
    pub fn new() -> Self {
        Writer { buffer: Vec::new() }
    }

    /// write a complete flo file
    pub fn write(
        self,
        sample_rate: u32,
        channels: u8,
        bit_depth: u8,
        compression_level: u8,
        frames: &[Frame],
        metadata: &[u8],
    ) -> FloResult<Vec<u8>> {
        self.write_ex(
            sample_rate,
            channels,
            bit_depth,
            compression_level,
            false,
            0,
            frames,
            metadata,
        )
    }

    /// write flo file with extended options
    #[allow(clippy::too_many_arguments)]    pub fn write_ex(
        mut self,
        sample_rate: u32,
        channels: u8,
        bit_depth: u8,
        compression_level: u8,
        lossy: bool,
        lossy_quality: u8,
        frames: &[Frame],
        metadata: &[u8],
    ) -> FloResult<Vec<u8>> {
        // sizes
        let toc_size = 4 + (frames.len() * 20) as u64;
        let data_chunk = self.build_data_chunk(frames);
        let data_size = data_chunk.len() as u64;
        let extra_size = 0u64;
        let meta_size = metadata.len() as u64;

        // crc32
        let data_crc32 = crc32::compute(&data_chunk);

        // toc
        let toc_chunk = self.build_toc_chunk(frames);

        // flags
        let mut flags: u16 = 0;
        if lossy {
            flags |= 0x01; // lossy mode
            flags |= (lossy_quality as u16) << 8; // quality level
        }

        // header
        self.write_header_ex(
            sample_rate,
            channels,
            bit_depth,
            compression_level,
            frames.len() as u64,
            data_crc32,
            flags,
            toc_size,
            data_size,
            extra_size,
            meta_size,
        );

        // toc
        self.buffer.extend_from_slice(&toc_chunk);

        // data
        self.buffer.extend_from_slice(&data_chunk);

        // extra (empty for now)

        // metadata
        self.buffer.extend_from_slice(metadata);

        Ok(self.buffer)
    }

    #[allow(dead_code, clippy::too_many_arguments)]
    fn write_header(
        &mut self,
        sample_rate: u32,
        channels: u8,
        bit_depth: u8,
        compression_level: u8,
        total_frames: u64,
        data_crc32: u32,
        toc_size: u64,
        data_size: u64,
        extra_size: u64,
        meta_size: u64,
    ) {
        self.write_header_ex(
            sample_rate,
            channels,
            bit_depth,
            compression_level,
            total_frames,
            data_crc32,
            0,
            toc_size,
            data_size,
            extra_size,
            meta_size,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn write_header_ex(
        &mut self,
        sample_rate: u32,
        channels: u8,
        bit_depth: u8,
        compression_level: u8,
        total_frames: u64,
        data_crc32: u32,
        flags: u16,
        toc_size: u64,
        data_size: u64,
        extra_size: u64,
        meta_size: u64,
    ) {
        // Magic "FLO!"
        self.buffer.extend_from_slice(&MAGIC);

        // Version (u8, u8)
        self.buffer.push(VERSION_MAJOR);
        self.buffer.push(VERSION_MINOR);

        // Flags (u16 LE)
        self.buffer.extend_from_slice(&flags.to_le_bytes());

        // Sample Rate (u32 LE)
        self.buffer.extend_from_slice(&sample_rate.to_le_bytes());

        // Channels (u8)
        self.buffer.push(channels);

        // Bit Depth (u8)
        self.buffer.push(bit_depth);

        // Total Frames (u64 LE)
        self.buffer.extend_from_slice(&total_frames.to_le_bytes());

        // Compression Level (u8)
        self.buffer.push(compression_level);

        // Reserved (3 bytes)
        self.buffer.extend_from_slice(&[0, 0, 0]);

        // Data CRC32 (u32 LE)
        self.buffer.extend_from_slice(&data_crc32.to_le_bytes());

        // Header Size (u64 LE) - size after magic
        self.buffer.extend_from_slice(&HEADER_SIZE.to_le_bytes());

        // TOC Size (u64 LE)
        self.buffer.extend_from_slice(&toc_size.to_le_bytes());

        // Data Size (u64 LE)
        self.buffer.extend_from_slice(&data_size.to_le_bytes());

        // Extra Size (u64 LE)
        self.buffer.extend_from_slice(&extra_size.to_le_bytes());

        // Meta Size (u64 LE)
        self.buffer.extend_from_slice(&meta_size.to_le_bytes());
    }

    fn build_toc_chunk(&self, frames: &[Frame]) -> Vec<u8> {
        let mut toc = Vec::new();

        // Number of entries (u32 LE)
        toc.extend_from_slice(&(frames.len() as u32).to_le_bytes());

        let mut byte_offset = 0u64;

        for (i, frame) in frames.iter().enumerate() {
            let frame_size = frame.byte_size() as u32;

            // Frame index (u32 LE)
            toc.extend_from_slice(&(i as u32).to_le_bytes());

            // Byte offset (u64 LE)
            toc.extend_from_slice(&byte_offset.to_le_bytes());

            // Frame size (u32 LE)
            toc.extend_from_slice(&frame_size.to_le_bytes());

            // Timestamp in milliseconds (u32 LE)
            let timestamp_ms = (i as u32) * 1000;
            toc.extend_from_slice(&timestamp_ms.to_le_bytes());

            byte_offset += frame_size as u64;
        }

        toc
    }

    fn build_data_chunk(&self, frames: &[Frame]) -> Vec<u8> {
        let mut data = Vec::new();

        for frame in frames {
            self.write_frame(&mut data, frame);
        }

        data
    }

    fn write_frame(&self, buffer: &mut Vec<u8>, frame: &Frame) {
        let frame_type = FrameType::from(frame.frame_type);

        // frame header
        buffer.push(frame.frame_type);
        buffer.extend_from_slice(&frame.frame_samples.to_le_bytes());
        buffer.push(frame.flags);

        // channel data with size prefix
        for ch_data in &frame.channels {
            // build channel first to get size
            let mut ch_buffer = Vec::new();
            self.write_channel_data(&mut ch_buffer, ch_data, frame_type);

            // size then data
            buffer.extend_from_slice(&(ch_buffer.len() as u32).to_le_bytes());
            buffer.extend_from_slice(&ch_buffer);
        }
    }

    fn write_channel_data(
        &self,
        buffer: &mut Vec<u8>,
        ch_data: &crate::ChannelData,
        frame_type: FrameType,
    ) {
        match frame_type {
            FrameType::Silence => {
                // nothing for silence
            }
            FrameType::Raw => {
                // raw residuals
                buffer.extend_from_slice(&ch_data.residuals);
            }
            FrameType::Transform => {
                // already serialized
                buffer.extend_from_slice(&ch_data.residuals);
            }
            _ if frame_type.is_alpc() => {
                // predictor count
                buffer.push(ch_data.predictor_coeffs.len() as u8);

                // predictor coeffs
                for &coeff in &ch_data.predictor_coeffs {
                    buffer.extend_from_slice(&coeff.to_le_bytes());
                }

                // shift bits
                buffer.push(ch_data.shift_bits);

                // residual encoding
                buffer.push(ch_data.residual_encoding as u8);

                // rice param
                if ch_data.residual_encoding == ResidualEncoding::Rice {
                    buffer.push(ch_data.rice_parameter);
                }

                // residuals
                buffer.extend_from_slice(&ch_data.residuals);
            }
            _ => {
                // reserved
            }
        }
    }
}

impl Default for Writer {
    fn default() -> Self {
        Self::new()
    }
}
