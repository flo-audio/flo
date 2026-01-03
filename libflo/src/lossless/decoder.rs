use crate::core::audio_constants::i32_to_f32;
use crate::core::types::{ChannelData, FloFile};
use crate::{core::rice, FloResult, Reader};

/// audio decoder for flo format
pub struct Decoder;

impl Decoder {
    pub fn new() -> Self {
        Decoder
    }

    /// decode flo file to samples
    pub fn decode(&self, data: &[u8]) -> FloResult<Vec<f32>> {
        let reader = Reader::new();
        let file = reader.read(data)?;
        self.decode_file(&file)
    }

    /// decode from parsed file
    pub fn decode_file(&self, file: &FloFile) -> FloResult<Vec<f32>> {
        let channels = file.header.channels as usize;
        let mut all_samples: Vec<Vec<i32>> = vec![vec![]; channels];

        for frame in &file.frames {
            let use_mid_side = channels == 2 && (frame.flags & 0x01) != 0;

            let mut frame_channels: Vec<Vec<i32>> = Vec::with_capacity(channels);

            for ch_data in &frame.channels {
                let samples = self.decode_channel_int(ch_data, frame.frame_samples as usize)?;
                frame_channels.push(samples);
            }

            // mid-side to left-right
            if use_mid_side && frame_channels.len() == 2 {
                let (left, right) = self.decode_mid_side(&frame_channels[0], &frame_channels[1]);
                all_samples[0].extend(left);
                all_samples[1].extend(right);
            } else {
                for (ch_idx, samples) in frame_channels.into_iter().enumerate() {
                    if ch_idx < channels {
                        all_samples[ch_idx].extend(samples);
                    }
                }
            }
        }

        // interleave and convert to f32
        let max_len = all_samples.iter().map(|v| v.len()).max().unwrap_or(0);
        let mut interleaved = Vec::with_capacity(max_len * channels);

        // Fast path for stereo (most common case)
        if channels == 2 && all_samples[0].len() == all_samples[1].len() {
            let left = &all_samples[0];
            let right = &all_samples[1];
            for i in 0..left.len() {
                interleaved.push(i32_to_f32(left[i]));
                interleaved.push(i32_to_f32(right[i]));
            }
        } else {
            // General case for mono or mismatched lengths
            for i in 0..max_len {
                for ch in 0..channels {
                    let sample = all_samples[ch].get(i).copied().unwrap_or(0);
                    interleaved.push(i32_to_f32(sample));
                }
            }
        }

        Ok(interleaved)
    }

    /// Convert mid-side back to left-right
    fn decode_mid_side(&self, mid: &[i32], side: &[i32]) -> (Vec<i32>, Vec<i32>) {
        // FLAC-style: mid = L + R, side = L - R
        // So: L = (mid + side) / 2, R = (mid - side) / 2
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
            // Fixed predictor: order stored as (128 + order)
            let fixed_order = (shift_bits - 128) as usize;

            let residuals =
                rice::decode_i32(&ch_data.residuals, ch_data.rice_parameter, frame_samples);

            return Ok(self.reconstruct_fixed(fixed_order, &residuals, frame_samples));
        }

        if has_coeffs {
            // LPC decoding with stored coefficients
            let residuals =
                rice::decode_i32(&ch_data.residuals, ch_data.rice_parameter, frame_samples);

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
            // Raw PCM
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

    /// Reconstruct from LPC prediction
    /// Optimized version with branch-free inner loop
    #[inline]
    fn reconstruct_lpc_int(
        &self,
        coeffs: &[i32],
        residuals: &[i32],
        shift: u8,
        order: usize,
        target_len: usize,
    ) -> Vec<i32> {
        let mut samples = Vec::with_capacity(target_len);
        let actual_len = target_len.min(residuals.len());

        // Warmup samples from residuals (no prediction needed)
        let warmup_len = order.min(actual_len);
        samples.extend_from_slice(&residuals[..warmup_len]);

        // Reconstruct remaining samples using LPC prediction
        // This is the hot loop - keep it simple and predictable
        for i in order..actual_len {
            let mut prediction: i64 = 0;

            // Unrolled inner loop for common orders
            // Access pattern: samples[i-1], samples[i-2], ..., samples[i-order]
            for j in 0..order {
                prediction += (coeffs[j] as i64) * (samples[i - j - 1] as i64);
            }

            samples.push((prediction >> shift) as i32 + residuals[i]);
        }

        // Pad if needed
        samples.resize(target_len, 0);
        samples
    }

    /// Reconstruct from fixed predictor
    fn reconstruct_fixed(&self, order: usize, residuals: &[i32], target_len: usize) -> Vec<i32> {
        let mut samples = Vec::with_capacity(target_len);

        if residuals.is_empty() {
            return vec![0; target_len];
        }

        match order {
            0 => {
                // No prediction - residuals are samples
                samples.extend_from_slice(residuals);
            }
            1 => {
                // s[i] = r[i] + s[i-1]
                samples.push(residuals[0]);
                for i in 1..residuals.len().min(target_len) {
                    samples.push(residuals[i].wrapping_add(samples[i - 1]));
                }
            }
            2 => {
                // s[i] = r[i] + 2*s[i-1] - s[i-2]
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
                // s[i] = r[i] + 3*s[i-1] - 3*s[i-2] + s[i-3]
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
                // s[i] = r[i] + 4*s[i-1] - 6*s[i-2] + 4*s[i-3] - s[i-4]
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
            _ => {
                // Unknown order, just use residuals
                samples.extend_from_slice(residuals);
            }
        }

        // Pad if needed
        while samples.len() < target_len {
            samples.push(0);
        }

        samples
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}
