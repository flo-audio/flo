use crate::core::audio_constants::f32_to_i32;
use crate::core::{ChannelData, Frame, FrameType, ResidualEncoding};
use crate::{core::rice, FloResult, Writer};

use super::lpc::{
    autocorr_int, calc_residuals_int, fixed_predictor_residuals, levinson_durbin_int,
};

pub struct Encoder {
    sample_rate: u32,
    channels: u8,
    bit_depth: u8,
    compression_level: u8,
}

impl Encoder {
    pub fn new(sample_rate: u32, channels: u8, bit_depth: u8) -> Self {
        Encoder {
            sample_rate,
            channels,
            bit_depth,
            compression_level: 5,
        }
    }

    pub fn with_compression(mut self, level: u8) -> Self {
        self.compression_level = level.min(9);
        self
    }

    /// encode samples to flo format
    pub fn encode(&self, samples: &[f32], metadata: &[u8]) -> FloResult<Vec<u8>> {
        let samples_per_frame = self.sample_rate as usize;
        let frames = self.encode_frames(samples, samples_per_frame);

        let writer = Writer::new();
        writer.write(
            self.sample_rate,
            self.channels,
            self.bit_depth,
            self.compression_level,
            &frames,
            metadata,
        )
    }

    fn encode_frames(&self, samples: &[f32], samples_per_frame: usize) -> Vec<Frame> {
        let total_samples = samples.len() / self.channels as usize;
        let num_frames = total_samples.div_ceil(samples_per_frame);

        let mut frames = Vec::with_capacity(num_frames);

        for frame_idx in 0..num_frames {
            let start = frame_idx * samples_per_frame * self.channels as usize;
            let end =
                ((frame_idx + 1) * samples_per_frame * self.channels as usize).min(samples.len());

            let frame_samples = &samples[start..end];
            let frame = self.encode_frame(frame_samples);
            frames.push(frame);
        }

        frames
    }

    fn encode_frame(&self, samples: &[f32]) -> Frame {
        let num_samples = samples.len() / self.channels as usize;

        // Check for silence
        if samples.iter().all(|&s| s.abs() < 1e-7) {
            let mut frame = Frame::new(FrameType::Silence as u8, num_samples as u32);
            for _ in 0..self.channels {
                frame.channels.push(ChannelData::new_silence());
            }
            return frame;
        }

        // Convert to integer domain
        let samples_i32: Vec<i32> = samples.iter().map(|&s| f32_to_i32(s)).collect();

        // Deinterleave channels
        let mut channel_data: Vec<Vec<i32>> = (0..self.channels as usize)
            .map(|ch| {
                samples_i32
                    .iter()
                    .skip(ch)
                    .step_by(self.channels as usize)
                    .copied()
                    .collect()
            })
            .collect();

        // Apply mid-side coding for stereo (if it helps)
        let use_mid_side = self.channels == 2 && self.should_use_mid_side(&channel_data);
        if use_mid_side {
            let (mid, side) = self.to_mid_side(&channel_data[0], &channel_data[1]);
            channel_data[0] = mid;
            channel_data[1] = side;
        }

        // Encode each channel
        let lpc_order = self.lpc_order_from_level();
        let mut encoded_channels = Vec::with_capacity(self.channels as usize);
        let mut all_raw = true;

        for ch_samples in &channel_data {
            let (ch_data, order_used) = self.encode_channel_int(ch_samples, lpc_order);
            if order_used > 0 {
                all_raw = false;
            }
            encoded_channels.push(ch_data);
        }

        // Determine frame type
        let frame_type = if all_raw {
            FrameType::Raw
        } else {
            FrameType::from_order(lpc_order)
        };

        let mut frame = Frame::new(frame_type as u8, num_samples as u32);
        // Set mid-side flag if used
        if use_mid_side {
            frame.flags |= 0x01; // Bit 0 = mid-side coding
        }
        frame.channels = encoded_channels;
        frame
    }

    /// Check if mid-side coding would help
    fn should_use_mid_side(&self, channels: &[Vec<i32>]) -> bool {
        if channels.len() != 2 {
            return false;
        }

        let left = &channels[0];
        let right = &channels[1];

        // Calculate variance of L-R vs L and R separately
        let mut var_l: i64 = 0;
        let mut var_r: i64 = 0;
        let mut var_side: i64 = 0;

        for (&l, &r) in left.iter().zip(right.iter()) {
            var_l += (l as i64) * (l as i64);
            var_r += (r as i64) * (r as i64);
            let side = l - r;
            var_side += (side as i64) * (side as i64);
        }

        // If side channel has less energy, mid-side helps
        var_side < (var_l + var_r) / 2
    }

    /// Convert stereo to mid-side
    fn to_mid_side(&self, left: &[i32], right: &[i32]) -> (Vec<i32>, Vec<i32>) {
        // FLAC-style: mid = L + R, side = L - R
        // This preserves all bits - no rounding
        let mid: Vec<i32> = left
            .iter()
            .zip(right.iter())
            .map(|(&l, &r)| l + r)
            .collect();
        let side: Vec<i32> = left
            .iter()
            .zip(right.iter())
            .map(|(&l, &r)| l - r)
            .collect();
        (mid, side)
    }

    /// Encode a single channel using integer LPC
    fn encode_channel_int(&self, samples: &[i32], max_order: usize) -> (ChannelData, usize) {
        if samples.is_empty() {
            return (ChannelData::new_silence(), 0);
        }

        // Try different encoding strategies and pick the smallest
        let mut best_data: Option<ChannelData> = None;
        let mut best_size = usize::MAX;
        let mut best_order = 0;

        // Strategy 1: Raw PCM (baseline)
        let raw = self.encode_raw(samples);
        let raw_size = raw.residuals.len();
        if raw_size < best_size {
            best_size = raw_size;
            best_data = Some(raw);
            best_order = 0;
        }

        // Strategy 2: Fixed predictors (order 0-4, very fast)
        for order in 0..=4.min(max_order) {
            if let Some((data, size)) = self.try_fixed_predictor(samples, order) {
                if size < best_size {
                    best_size = size;
                    best_data = Some(data);
                    best_order = order;
                }
            }
        }

        // Strategy 3: LPC predictors (if compression level allows)
        if self.compression_level >= 3 && max_order > 4 {
            for order in 5..=max_order {
                if let Some((data, size)) = self.try_lpc_predictor(samples, order) {
                    if size < best_size {
                        best_size = size;
                        best_data = Some(data);
                        best_order = order;
                    }
                }
            }
        }

        (best_data.unwrap(), best_order)
    }

    /// Encode as raw PCM
    fn encode_raw(&self, samples: &[i32]) -> ChannelData {
        let raw_bytes: Vec<u8> = samples
            .iter()
            .flat_map(|&s| (s as i16).to_le_bytes().to_vec())
            .collect();
        ChannelData::new_raw(raw_bytes)
    }

    /// Try fixed predictor
    fn try_fixed_predictor(&self, samples: &[i32], order: usize) -> Option<(ChannelData, usize)> {
        if order > 4 {
            return None;
        }

        let residuals = fixed_predictor_residuals(samples, order);

        // Find optimal Rice parameter
        let k = rice::estimate_rice_parameter_i32(&residuals);
        let encoded = rice::encode_i32(&residuals, k);

        // For fixed predictors: store negative order to distinguish from LPC
        // predictor_coeffs is empty, shift_bits stores (128 + order) as marker
        let ch_data = ChannelData {
            predictor_coeffs: vec![],        // Empty = fixed predictor
            shift_bits: (128 + order) as u8, // Marker: 128-132 = fixed order 0-4
            residual_encoding: ResidualEncoding::Rice,
            rice_parameter: k,
            residuals: encoded.clone(),
        };

        Some((ch_data, encoded.len()))
    }

    /// Try LPC predictor with given order
    fn try_lpc_predictor(&self, samples: &[i32], order: usize) -> Option<(ChannelData, usize)> {
        if samples.len() <= order {
            return None;
        }

        // Calculate autocorrelation in integer domain
        let autocorr = autocorr_int(samples, order);

        // Levinson-Durbin for LPC coefficients (in fixed-point)
        let (coeffs_fp, shift) = levinson_durbin_int(&autocorr, order)?;

        // Calculate residuals using integer arithmetic
        let residuals = calc_residuals_int(samples, &coeffs_fp, shift, order);

        // Check if residuals are reasonable (not exploding)
        let max_res = residuals.iter().map(|&r| r.abs()).max().unwrap_or(0);
        if max_res > 1_000_000 {
            return None; // Unstable, skip this order
        }

        // Encode residuals
        let k = rice::estimate_rice_parameter_i32(&residuals);
        let encoded = rice::encode_i32(&residuals, k);

        let ch_data = ChannelData {
            predictor_coeffs: coeffs_fp,
            shift_bits: shift,
            residual_encoding: ResidualEncoding::Rice,
            rice_parameter: k,
            residuals: encoded.clone(),
        };

        Some((ch_data, encoded.len()))
    }

    fn lpc_order_from_level(&self) -> usize {
        match self.compression_level {
            0 => 0, // Only fixed predictors
            1 => 2,
            2 => 4,
            3 => 4,
            4 => 6,
            5 => 8,
            6 => 8,
            7 => 10,
            8 => 12,
            _ => 12,
        }
    }
}

impl Default for Encoder {
    fn default() -> Self {
        Encoder::new(44100, 1, 16)
    }
}
