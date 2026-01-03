use crate::core::audio_constants::i32_to_f32;
use crate::core::{rice, ChannelData, FloResult, Frame, FrameType, Header, TocEntry};
use crate::lossless::Decoder as LosslessDecoder;
use crate::lossy::{deserialize_frame, TransformDecoder};
use crate::{Reader, ResidualEncoding, MAGIC};

use super::types::{DecoderState, StreamingAudioInfo};

pub struct StreamingDecoder {
    /// incoming data buffer
    buffer: Vec<u8>,
    /// current state
    state: DecoderState,
    /// parsed header
    header: Option<Header>,
    /// toc entries
    toc: Vec<TocEntry>,
    /// current frame being decoded
    current_frame: usize,
    /// where data chunk starts
    data_offset: usize,
    /// lossy decoder when needed
    lossy_decoder: Option<TransformDecoder>,
    /// is lossy?
    is_lossy: bool,
    /// skipped preroll frame?
    skipped_preroll: bool,
}

impl StreamingDecoder {
    /// new streaming decoder
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(64 * 1024),
            state: DecoderState::WaitingForHeader,
            header: None,
            toc: Vec::new(),
            current_frame: 0,
            data_offset: 0,
            lossy_decoder: None,
            is_lossy: false,
            skipped_preroll: false,
        }
    }

    /// current state
    pub fn state(&self) -> DecoderState {
        self.state
    }

    /// audio info if we have the header
    pub fn info(&self) -> Option<StreamingAudioInfo> {
        self.header.as_ref().map(|h| StreamingAudioInfo {
            sample_rate: h.sample_rate,
            channels: h.channels,
            bit_depth: h.bit_depth,
            total_frames: h.total_frames,
            is_lossy: self.is_lossy,
        })
    }

    /// how many frames ready to decode
    pub fn frames_available(&self) -> usize {
        if self.state != DecoderState::Ready {
            return 0;
        }
        self.count_complete_frames()
    }

    /// feed more data, returns true if new frames available
    pub fn feed(&mut self, data: &[u8]) -> FloResult<bool> {
        if self.state == DecoderState::Error || self.state == DecoderState::Finished {
            return Ok(false);
        }

        self.buffer.extend_from_slice(data);
        self.try_advance_state()
    }

    /// decode next frame, or None if nothing ready
    pub fn next_frame(&mut self) -> FloResult<Option<Vec<f32>>> {
        if self.state != DecoderState::Ready {
            return Ok(None);
        }

        let header = match self.header.as_ref() {
            Some(h) => h.clone(),
            None => return Err("No header".to_string()),
        };

        if self.current_frame >= self.toc.len() {
            self.state = DecoderState::Finished;
            return Ok(None);
        }

        let toc_entry = &self.toc[self.current_frame];
        let frame_start = self.data_offset + toc_entry.byte_offset as usize;
        let frame_end = frame_start + toc_entry.frame_size as usize;

        if frame_end > self.buffer.len() {
            return Ok(None);
        }

        let frame_data = &self.buffer[frame_start..frame_end];
        let frame = self.parse_frame(frame_data, header.channels)?;

        self.current_frame += 1;
        let samples = self.decode_frame(&frame, &header)?;

        Ok(Some(samples))
    }

    /// decode everything we have
    pub fn decode_available(&mut self) -> FloResult<Vec<f32>> {
        if self.state != DecoderState::Ready {
            return Ok(Vec::new());
        }

        let samples = self.decode_with_standard_decoder()?;
        self.state = DecoderState::Finished;
        Ok(samples)
    }

    /// reset for reuse
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.state = DecoderState::WaitingForHeader;
        self.header = None;
        self.toc.clear();
        self.current_frame = 0;
        self.data_offset = 0;
        self.lossy_decoder = None;
        self.is_lossy = false;
        self.skipped_preroll = false;
    }

    /// bytes buffered
    pub fn buffered_bytes(&self) -> usize {
        self.buffer.len()
    }

    /// frames ready to decode
    pub fn available_frames(&self) -> usize {
        if self.state != DecoderState::Ready {
            return 0;
        }
        self.count_complete_frames()
            .saturating_sub(self.current_frame)
    }

    /// current frame index
    pub fn current_frame_index(&self) -> usize {
        self.current_frame
    }

    // internal stuff

    fn try_advance_state(&mut self) -> FloResult<bool> {
        match self.state {
            DecoderState::WaitingForHeader => {
                if self.try_parse_header()? {
                    self.state = DecoderState::WaitingForToc;
                    return self.try_advance_state();
                }
            }
            DecoderState::WaitingForToc => {
                if self.try_parse_toc()? {
                    self.state = DecoderState::Ready;
                    return Ok(true);
                }
            }
            DecoderState::Ready => {
                return Ok(self.count_complete_frames() > self.current_frame);
            }
            _ => {}
        }
        Ok(false)
    }

    fn try_parse_header(&mut self) -> FloResult<bool> {
        // need at least 70 bytes
        if self.buffer.len() < 70 {
            return Ok(false);
        }

        if self.buffer[0..4] != MAGIC {
            self.state = DecoderState::Error;
            return Err("Invalid flo file: bad magic".to_string());
        }

        let header = Header {
            version_major: self.buffer[4],
            version_minor: self.buffer[5],
            flags: u16::from_le_bytes([self.buffer[6], self.buffer[7]]),
            sample_rate: u32::from_le_bytes([
                self.buffer[8],
                self.buffer[9],
                self.buffer[10],
                self.buffer[11],
            ]),
            channels: self.buffer[12],
            bit_depth: self.buffer[13],
            total_frames: u64::from_le_bytes([
                self.buffer[14],
                self.buffer[15],
                self.buffer[16],
                self.buffer[17],
                self.buffer[18],
                self.buffer[19],
                self.buffer[20],
                self.buffer[21],
            ]),
            compression_level: self.buffer[22],
            data_crc32: u32::from_le_bytes([
                self.buffer[26],
                self.buffer[27],
                self.buffer[28],
                self.buffer[29],
            ]),
            header_size: u64::from_le_bytes([
                self.buffer[30],
                self.buffer[31],
                self.buffer[32],
                self.buffer[33],
                self.buffer[34],
                self.buffer[35],
                self.buffer[36],
                self.buffer[37],
            ]),
            toc_size: u64::from_le_bytes([
                self.buffer[38],
                self.buffer[39],
                self.buffer[40],
                self.buffer[41],
                self.buffer[42],
                self.buffer[43],
                self.buffer[44],
                self.buffer[45],
            ]),
            data_size: u64::from_le_bytes([
                self.buffer[46],
                self.buffer[47],
                self.buffer[48],
                self.buffer[49],
                self.buffer[50],
                self.buffer[51],
                self.buffer[52],
                self.buffer[53],
            ]),
            extra_size: u64::from_le_bytes([
                self.buffer[54],
                self.buffer[55],
                self.buffer[56],
                self.buffer[57],
                self.buffer[58],
                self.buffer[59],
                self.buffer[60],
                self.buffer[61],
            ]),
            meta_size: u64::from_le_bytes([
                self.buffer[62],
                self.buffer[63],
                self.buffer[64],
                self.buffer[65],
                self.buffer[66],
                self.buffer[67],
                self.buffer[68],
                self.buffer[69],
            ]),
        };

        self.is_lossy = (header.flags & 0x01) != 0;
        if self.is_lossy {
            self.lossy_decoder = Some(TransformDecoder::new(header.sample_rate, header.channels));
        }

        self.header = Some(header);
        Ok(true)
    }

    fn try_parse_toc(&mut self) -> FloResult<bool> {
        let header = self.header.as_ref().ok_or("No header")?;
        let toc_start = 70;
        let toc_end = toc_start + header.toc_size as usize;

        if self.buffer.len() < toc_end {
            return Ok(false);
        }

        if header.toc_size >= 4 {
            let num_entries = u32::from_le_bytes([
                self.buffer[toc_start],
                self.buffer[toc_start + 1],
                self.buffer[toc_start + 2],
                self.buffer[toc_start + 3],
            ]) as usize;

            let entries_start = toc_start + 4;
            for i in 0..num_entries {
                let offset = entries_start + i * 20;
                if offset + 20 > self.buffer.len() {
                    return Ok(false);
                }

                self.toc.push(TocEntry {
                    frame_index: u32::from_le_bytes([
                        self.buffer[offset],
                        self.buffer[offset + 1],
                        self.buffer[offset + 2],
                        self.buffer[offset + 3],
                    ]),
                    byte_offset: u64::from_le_bytes([
                        self.buffer[offset + 4],
                        self.buffer[offset + 5],
                        self.buffer[offset + 6],
                        self.buffer[offset + 7],
                        self.buffer[offset + 8],
                        self.buffer[offset + 9],
                        self.buffer[offset + 10],
                        self.buffer[offset + 11],
                    ]),
                    frame_size: u32::from_le_bytes([
                        self.buffer[offset + 12],
                        self.buffer[offset + 13],
                        self.buffer[offset + 14],
                        self.buffer[offset + 15],
                    ]),
                    timestamp_ms: u32::from_le_bytes([
                        self.buffer[offset + 16],
                        self.buffer[offset + 17],
                        self.buffer[offset + 18],
                        self.buffer[offset + 19],
                    ]),
                });
            }
        }

        self.data_offset = toc_end;
        Ok(true)
    }

    fn count_complete_frames(&self) -> usize {
        let mut count = 0;
        for entry in &self.toc {
            let frame_end =
                self.data_offset + entry.byte_offset as usize + entry.frame_size as usize;
            if frame_end <= self.buffer.len() {
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    fn parse_frame(&self, data: &[u8], channels: u8) -> FloResult<Frame> {
        if data.len() < 6 {
            return Err("Frame too small".to_string());
        }

        let frame_type_byte = data[0];
        let frame_samples = u32::from_le_bytes([data[1], data[2], data[3], data[4]]);
        let flags = data[5];

        let frame_type = FrameType::from(frame_type_byte);
        let mut frame = Frame::new(frame_type_byte, frame_samples);
        frame.flags = flags;

        let num_channels = if frame_type == FrameType::Transform {
            1
        } else {
            channels as usize
        };

        let mut pos = 6;
        for _ in 0..num_channels {
            if pos + 4 > data.len() {
                return Err("Frame truncated".to_string());
            }

            let ch_size =
                u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                    as usize;
            pos += 4;

            if pos + ch_size > data.len() {
                return Err("Channel data truncated".to_string());
            }

            let ch_data = &data[pos..pos + ch_size];
            pos += ch_size;

            let channel = match frame_type {
                FrameType::Silence => ChannelData::new_silence(),
                FrameType::Raw | FrameType::Transform => ChannelData {
                    predictor_coeffs: vec![],
                    shift_bits: 0,
                    residual_encoding: ResidualEncoding::Raw,
                    rice_parameter: 0,
                    residuals: ch_data.to_vec(),
                },
                _ => self.parse_alpc_channel(ch_data, frame_type)?,
            };

            frame.channels.push(channel);
        }

        Ok(frame)
    }

    fn parse_alpc_channel(&self, data: &[u8], _frame_type: FrameType) -> FloResult<ChannelData> {
        if data.is_empty() {
            return Ok(ChannelData::new_silence());
        }

        let order = data[0] as usize;
        if order > 12 {
            return Err("Invalid LPC order".to_string());
        }

        let coeff_bytes = order * 4;
        let min_size = 1 + coeff_bytes + 2; // order + coeffs + shift + encoding
        if data.len() < min_size {
            return Err("ALPC channel too small".to_string());
        }

        // Read coefficients
        let mut coefficients = Vec::with_capacity(order);
        for i in 0..order {
            let offset = 1 + i * 4;
            let coeff = i32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            coefficients.push(coeff);
        }

        let mut pos = 1 + coeff_bytes;

        // Read shift_bits
        let shift_bits = data[pos];
        pos += 1;

        // Read residual encoding
        let residual_encoding_byte = data[pos];
        let residual_encoding = ResidualEncoding::from(residual_encoding_byte);
        pos += 1;

        // Read rice parameter (only for Rice encoding)
        let rice_parameter = if residual_encoding == ResidualEncoding::Rice {
            if pos >= data.len() {
                return Err("Missing rice parameter".to_string());
            }
            let rp = data[pos];
            pos += 1;
            rp
        } else {
            0
        };

        // Rest is residuals
        let residuals = data[pos..].to_vec();

        Ok(ChannelData {
            predictor_coeffs: coefficients,
            shift_bits,
            residual_encoding,
            rice_parameter,
            residuals,
        })
    }

    fn decode_frame(&mut self, frame: &Frame, header: &Header) -> FloResult<Vec<f32>> {
        let frame_type = FrameType::from(frame.frame_type);

        // Handle Transform (lossy) frames
        if frame_type == FrameType::Transform {
            if frame.channels.is_empty() {
                return Ok(Vec::new());
            }

            let frame_data = &frame.channels[0].residuals;
            if let Some(transform_frame) = deserialize_frame(frame_data) {
                let decoder = self.lossy_decoder.get_or_insert_with(|| {
                    TransformDecoder::new(header.sample_rate, header.channels)
                });
                let samples = decoder.decode_frame(&transform_frame);

                // Skip first frame (preroll) for lossy
                if !self.skipped_preroll {
                    self.skipped_preroll = true;
                    return Ok(Vec::new());
                }

                return Ok(samples);
            }
            return Ok(Vec::new());
        }

        // Handle lossless frames (Silence, Raw, ALPC variants)
        let channels = header.channels as usize;
        let frame_samples = frame.frame_samples as usize;
        let use_mid_side = channels == 2 && (frame.flags & 0x01) != 0;

        let mut frame_channels: Vec<Vec<i32>> = Vec::with_capacity(channels);

        for ch_data in &frame.channels {
            let samples = self.decode_channel_int(ch_data, frame_samples)?;
            frame_channels.push(samples);
        }

        // Convert mid-side back to left-right if needed
        let mut all_samples: Vec<Vec<i32>> = vec![vec![]; channels];
        if use_mid_side && frame_channels.len() == 2 {
            let (left, right) = self.decode_mid_side(&frame_channels[0], &frame_channels[1]);
            all_samples[0] = left;
            all_samples[1] = right;
        } else {
            for (ch_idx, samples) in frame_channels.into_iter().enumerate() {
                if ch_idx < channels {
                    all_samples[ch_idx] = samples;
                }
            }
        }

        // Interleave and convert to f32
        let max_len = all_samples.iter().map(|v| v.len()).max().unwrap_or(0);
        let mut interleaved = Vec::with_capacity(max_len * channels);

        for i in 0..max_len {
            for ch in 0..channels {
                let sample = all_samples[ch].get(i).copied().unwrap_or(0);
                interleaved.push(i32_to_f32(sample));
            }
        }

        Ok(interleaved)
    }

    /// Decode a single channel to integers
    fn decode_channel_int(
        &self,
        ch_data: &ChannelData,
        frame_samples: usize,
    ) -> FloResult<Vec<i32>> {
        let has_coeffs = !ch_data.predictor_coeffs.is_empty();
        let has_residuals = !ch_data.residuals.is_empty();
        let shift_bits = ch_data.shift_bits;

        // Check for fixed predictor marker: shift_bits >= 128 means fixed order (128 + order)
        let is_fixed_predictor = !has_coeffs && has_residuals && shift_bits >= 128;

        if is_fixed_predictor {
            let fixed_order = (shift_bits - 128) as usize;
            let residuals =
                rice::decode_i32(&ch_data.residuals, ch_data.rice_parameter, frame_samples);
            return Ok(self.reconstruct_fixed(fixed_order, &residuals, frame_samples));
        }

        if has_coeffs {
            // LPC decoding with stored coefficients
            // Decode residuals based on encoding type
            let residuals = match ch_data.residual_encoding {
                ResidualEncoding::Rice => {
                    rice::decode_i32(&ch_data.residuals, ch_data.rice_parameter, frame_samples)
                }
                ResidualEncoding::Raw | ResidualEncoding::Golomb => {
                    // Raw residuals as i16 (Golomb not implemented, fallback to raw)
                    let mut res = Vec::with_capacity(frame_samples);
                    for chunk in ch_data.residuals.chunks(2) {
                        if chunk.len() == 2 {
                            res.push(i16::from_le_bytes([chunk[0], chunk[1]]) as i32);
                        }
                    }
                    while res.len() < frame_samples {
                        res.push(0);
                    }
                    res
                }
            };

            let order = ch_data.predictor_coeffs.len();
            let samples = self.reconstruct_lpc_int(
                &ch_data.predictor_coeffs,
                &residuals,
                shift_bits,
                order,
                frame_samples,
            );
            return Ok(samples);
        }

        if has_residuals {
            // Raw PCM (no prediction)
            let mut samples = Vec::with_capacity(frame_samples);
            for chunk in ch_data.residuals.chunks(2) {
                if chunk.len() == 2 {
                    samples.push(i16::from_le_bytes([chunk[0], chunk[1]]) as i32);
                }
            }
            while samples.len() < frame_samples {
                samples.push(0);
            }
            return Ok(samples);
        }

        // Silence
        Ok(vec![0; frame_samples])
    }

    /// Convert mid-side back to left-right
    fn decode_mid_side(&self, mid: &[i32], side: &[i32]) -> (Vec<i32>, Vec<i32>) {
        let left: Vec<i32> = mid
            .iter()
            .zip(side.iter())
            .map(|(&m, &s)| (m + s) / 2)
            .collect();
        let right: Vec<i32> = mid
            .iter()
            .zip(side.iter())
            .map(|(&m, &s)| (m - s) / 2)
            .collect();
        (left, right)
    }

    /// Reconstruct from LPC prediction
    fn reconstruct_lpc_int(
        &self,
        coeffs: &[i32],
        residuals: &[i32],
        shift: u8,
        order: usize,
        target_len: usize,
    ) -> Vec<i32> {
        let mut samples = Vec::with_capacity(target_len);

        // Warmup samples from residuals
        for i in 0..order.min(residuals.len()) {
            samples.push(residuals[i]);
        }

        // Reconstruct remaining
        for i in order..target_len.min(residuals.len()) {
            let mut prediction: i64 = 0;
            for (j, &coeff) in coeffs.iter().enumerate() {
                if i > j {
                    prediction += (coeff as i64) * (samples[i - j - 1] as i64);
                }
            }
            prediction >>= shift;
            samples.push(prediction as i32 + residuals[i]);
        }

        while samples.len() < target_len {
            samples.push(0);
        }

        samples
    }

    /// Reconstruct from fixed predictor
    fn reconstruct_fixed(&self, order: usize, residuals: &[i32], target_len: usize) -> Vec<i32> {
        let mut samples = Vec::with_capacity(target_len);

        if residuals.is_empty() {
            return vec![0; target_len];
        }

        match order {
            0 => samples.extend_from_slice(residuals),
            1 => {
                samples.push(residuals[0]);
                for i in 1..residuals.len().min(target_len) {
                    samples.push(residuals[i].wrapping_add(samples[i - 1]));
                }
            }
            2 => {
                if !residuals.is_empty() {
                    samples.push(residuals[0]);
                }
                if residuals.len() > 1 {
                    samples.push(residuals[1].wrapping_add(samples[0]));
                }
                for i in 2..residuals.len().min(target_len) {
                    let pred = (2i64 * samples[i - 1] as i64 - samples[i - 2] as i64) as i32;
                    samples.push(residuals[i].wrapping_add(pred));
                }
            }
            3 => {
                if !residuals.is_empty() {
                    samples.push(residuals[0]);
                }
                if residuals.len() > 1 {
                    samples.push(residuals[1].wrapping_add(samples[0]));
                }
                if residuals.len() > 2 {
                    let pred = (2i64 * samples[1] as i64 - samples[0] as i64) as i32;
                    samples.push(residuals[2].wrapping_add(pred));
                }
                for i in 3..residuals.len().min(target_len) {
                    let pred = (3i64 * samples[i - 1] as i64 - 3i64 * samples[i - 2] as i64
                        + samples[i - 3] as i64) as i32;
                    samples.push(residuals[i].wrapping_add(pred));
                }
            }
            4 => {
                if !residuals.is_empty() {
                    samples.push(residuals[0]);
                }
                if residuals.len() > 1 {
                    samples.push(residuals[1].wrapping_add(samples[0]));
                }
                if residuals.len() > 2 {
                    let pred = (2i64 * samples[1] as i64 - samples[0] as i64) as i32;
                    samples.push(residuals[2].wrapping_add(pred));
                }
                if residuals.len() > 3 {
                    let pred = (3i64 * samples[2] as i64 - 3i64 * samples[1] as i64
                        + samples[0] as i64) as i32;
                    samples.push(residuals[3].wrapping_add(pred));
                }
                for i in 4..residuals.len().min(target_len) {
                    let pred = (4i64 * samples[i - 1] as i64 - 6i64 * samples[i - 2] as i64
                        + 4i64 * samples[i - 3] as i64
                        - samples[i - 4] as i64) as i32;
                    samples.push(residuals[i].wrapping_add(pred));
                }
            }
            _ => samples.extend_from_slice(residuals),
        }

        while samples.len() < target_len {
            samples.push(0);
        }

        samples
    }

    fn decode_with_standard_decoder(&self) -> FloResult<Vec<f32>> {
        let reader = Reader::new();
        let file = reader.read(&self.buffer)?;

        let is_transform = file
            .frames
            .iter()
            .any(|f| f.frame_type == (FrameType::Transform as u8));

        if is_transform {
            let mut decoder = TransformDecoder::new(file.header.sample_rate, file.header.channels);
            let mut all_samples = Vec::new();
            let mut frame_count = 0;

            for frame in &file.frames {
                if frame.channels.is_empty() {
                    continue;
                }
                let frame_data = &frame.channels[0].residuals;
                if let Some(transform_frame) = deserialize_frame(frame_data) {
                    let samples = decoder.decode_frame(&transform_frame);
                    if frame_count > 0 {
                        all_samples.extend(samples);
                    }
                    frame_count += 1;
                }
            }
            Ok(all_samples)
        } else {
            let decoder = LosslessDecoder::new();
            decoder.decode_file(&file)
        }
    }
}

impl Default for StreamingDecoder {
    fn default() -> Self {
        Self::new()
    }
}
