use crate::core::{
    ChannelData, FloFile, FloResult, Frame, FrameType, Header, ResidualEncoding, TocEntry,
};
use crate::MAGIC;

/// binary reader for flo format
pub struct Reader;

impl Reader {
    /// new reader
    pub fn new() -> Self {
        Reader
    }

    /// read and parse a flo file
    pub fn read(&self, data: &[u8]) -> FloResult<FloFile> {
        let mut cursor = Cursor::new(data);

        // magic
        let magic = cursor.read_bytes(4)?;
        if magic != MAGIC {
            return Err("Invalid flo file: bad magic".to_string());
        }

        // header
        let header = self.read_header(&mut cursor)?;

        // toc
        let toc = self.read_toc(&mut cursor, header.toc_size as usize)?;

        // Read DATA chunk
        let frames = self.read_data_chunk(
            &mut cursor,
            header.data_size as usize,
            header.channels,
            &toc,
        )?;

        // Skip EXTRA chunk
        cursor.skip(header.extra_size as usize)?;

        // Read META chunk
        let metadata = cursor.read_bytes(header.meta_size as usize)?;

        Ok(FloFile {
            header,
            toc,
            frames,
            extra: vec![],
            metadata,
        })
    }

    fn read_header(&self, cursor: &mut Cursor) -> FloResult<Header> {
        Ok(Header {
            version_major: cursor.read_u8()?,
            version_minor: cursor.read_u8()?,
            flags: cursor.read_u16_le()?,
            sample_rate: cursor.read_u32_le()?,
            channels: cursor.read_u8()?,
            bit_depth: cursor.read_u8()?,
            total_frames: cursor.read_u64_le()?,
            compression_level: cursor.read_u8()?,
            data_crc32: {
                cursor.skip(3)?; // reserved
                cursor.read_u32_le()?
            },
            header_size: cursor.read_u64_le()?,
            toc_size: cursor.read_u64_le()?,
            data_size: cursor.read_u64_le()?,
            extra_size: cursor.read_u64_le()?,
            meta_size: cursor.read_u64_le()?,
        })
    }

    fn read_toc(&self, cursor: &mut Cursor, toc_size: usize) -> FloResult<Vec<TocEntry>> {
        if toc_size < 4 {
            return Ok(vec![]);
        }

        let num_entries = cursor.read_u32_le()? as usize;

        if num_entries > 100_000 {
            return Err("Invalid TOC: too many entries".to_string());
        }

        let mut entries = Vec::with_capacity(num_entries);

        for _ in 0..num_entries {
            entries.push(TocEntry {
                frame_index: cursor.read_u32_le()?,
                byte_offset: cursor.read_u64_le()?,
                frame_size: cursor.read_u32_le()?,
                timestamp_ms: cursor.read_u32_le()?,
            });
        }

        Ok(entries)
    }

    fn read_data_chunk(
        &self,
        cursor: &mut Cursor,
        data_size: usize,
        channels: u8,
        toc: &[TocEntry],
    ) -> FloResult<Vec<Frame>> {
        let data_start = cursor.pos;
        let data_end = cursor.pos + data_size;
        let mut frames = Vec::with_capacity(toc.len());

        for toc_entry in toc.iter() {
            let frame_start = data_start + toc_entry.byte_offset as usize;

            if frame_start >= data_end {
                break;
            }

            cursor.pos = frame_start;
            let frame_size = toc_entry.frame_size as usize;

            let frame = self.read_frame(cursor, channels, frame_size)?;
            frames.push(frame);
        }

        cursor.pos = data_end;
        Ok(frames)
    }

    fn read_frame(&self, cursor: &mut Cursor, channels: u8, frame_size: usize) -> FloResult<Frame> {
        let frame_start = cursor.pos;
        let frame_end = frame_start + frame_size;

        // frame header: type(1) + samples(4) + flags(1)
        let frame_type_byte = cursor.read_u8()?;
        let frame_samples = cursor.read_u32_le()?;
        let flags = cursor.read_u8()?;

        let frame_type = FrameType::from(frame_type_byte);
        let mut frame = Frame::new(frame_type_byte, frame_samples);
        frame.flags = flags;

        // transform frames are one blob, others are per-channel
        let num_channels_to_read = if frame_type == FrameType::Transform {
            1
        } else {
            channels as usize
        };

        // read each channels data
        for _ch_idx in 0..num_channels_to_read {
            // channel size
            let ch_size = cursor.read_u32_le()? as usize;
            let ch_end = cursor.pos + ch_size;

            let ch_data =
                self.read_channel_data(cursor, frame_type, frame_samples as usize, ch_end)?;
            frame.channels.push(ch_data);

            // move to end of channel
            cursor.pos = ch_end;
        }

        cursor.pos = frame_end;
        Ok(frame)
    }

    fn read_channel_data(
        &self,
        cursor: &mut Cursor,
        frame_type: FrameType,
        frame_samples: usize,
        channel_end: usize,
    ) -> FloResult<ChannelData> {
        if frame_samples > 2_000_000 {
            return Err("Invalid frame: too many samples".to_string());
        }

        match frame_type {
            FrameType::Silence => Ok(ChannelData::new_silence()),

            FrameType::Raw => {
                let bytes_needed = frame_samples.saturating_mul(2);
                let available = channel_end.saturating_sub(cursor.pos);
                let bytes_to_read = bytes_needed.min(available);
                let residuals = cursor.read_bytes(bytes_to_read)?;
                Ok(ChannelData::new_raw(residuals))
            }

            FrameType::Transform => {
                // serialized mdct data
                let remaining = channel_end.saturating_sub(cursor.pos);
                let residuals = if remaining > 0 {
                    cursor.read_bytes(remaining)?
                } else {
                    vec![]
                };

                Ok(ChannelData {
                    predictor_coeffs: vec![],
                    shift_bits: 0,
                    residual_encoding: ResidualEncoding::Raw,
                    rice_parameter: 0,
                    residuals,
                })
            }

            _ if frame_type.is_alpc() => {
                // predictor order
                let order = cursor.read_u8()? as usize;

                if order > 12 {
                    return Err("Invalid LPC order".to_string());
                }

                // predictor coeffs
                let mut predictor_coeffs = Vec::with_capacity(order);
                for _ in 0..order {
                    if cursor.pos + 4 > channel_end {
                        break;
                    }
                    predictor_coeffs.push(cursor.read_i32_le()?);
                }

                let shift_bits = cursor.read_u8()?;

                let residual_encoding_byte = cursor.read_u8()?;
                let residual_encoding = ResidualEncoding::from(residual_encoding_byte);

                // rice param only for rice encoding
                let rice_parameter = if residual_encoding == ResidualEncoding::Rice {
                    cursor.read_u8()?
                } else {
                    0
                };

                // rest is residuals
                let remaining = channel_end.saturating_sub(cursor.pos);
                let residuals = if remaining > 0 {
                    cursor.read_bytes(remaining)?
                } else {
                    vec![]
                };

                Ok(ChannelData {
                    predictor_coeffs,
                    shift_bits,
                    residual_encoding,
                    rice_parameter,
                    residuals,
                })
            }

            _ => Ok(ChannelData::new_silence()),
        }
    }
}

impl Default for Reader {
    fn default() -> Self {
        Self::new()
    }
}

// cursor helper

struct Cursor<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Cursor { data, pos: 0 }
    }

    fn read_bytes(&mut self, count: usize) -> FloResult<Vec<u8>> {
        if self.pos + count > self.data.len() {
            return Err("Unexpected end of file".to_string());
        }
        let bytes = self.data[self.pos..self.pos + count].to_vec();
        self.pos += count;
        Ok(bytes)
    }

    fn skip(&mut self, count: usize) -> FloResult<()> {
        self.pos = (self.pos + count).min(self.data.len());
        Ok(())
    }

    fn read_u8(&mut self) -> FloResult<u8> {
        if self.pos >= self.data.len() {
            return Err("Unexpected end of file".to_string());
        }
        let val = self.data[self.pos];
        self.pos += 1;
        Ok(val)
    }

    fn read_u16_le(&mut self) -> FloResult<u16> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32_le(&mut self) -> FloResult<u32> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_i32_le(&mut self) -> FloResult<i32> {
        let bytes = self.read_bytes(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u64_le(&mut self) -> FloResult<u64> {
        let bytes = self.read_bytes(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
}
