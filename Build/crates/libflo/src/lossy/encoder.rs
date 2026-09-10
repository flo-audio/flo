use super::mdct::{BlockSize, Mdct, WindowType};
use super::psychoacoustic::{PsychoacousticModel, NUM_BARK_BANDS};
use crate::core::{ChannelData, Frame, FrameType, ResidualEncoding, I16_MAX_F32, I16_MIN_F32};

/// Transform lossy encoder
pub struct TransformEncoder {
    /// Sample rate
    sample_rate: u32,
    /// Number of channels
    channels: u8,
    /// MDCT processor
    mdct: Mdct,
    /// Psychoacoustic model (one per channel)
    psy_models: Vec<PsychoacousticModel>,
    /// Quality setting (0.0 = lowest, 1.0 = transparent)
    quality: f32,
    /// Block size
    block_size: BlockSize,
}

/// Encoded frame data
#[derive(Debug, Clone)]
pub struct TransformFrame {
    /// Quantized MDCT coefficients per channel (as i16)
    pub coefficients: Vec<Vec<i16>>,
    /// Scale factors per Bark band per channel
    pub scale_factors: Vec<Vec<f32>>,
    /// Block size used
    pub block_size: BlockSize,
    /// Number of samples this frame represents (after overlap-add)
    pub num_samples: usize,
}

impl TransformEncoder {
    /// Create a new transform encoder
    pub fn new(sample_rate: u32, channels: u8, quality: f32) -> Self {
        let block_size = BlockSize::Long; // 2048 samples
        let fft_size = block_size.samples();

        let mdct = Mdct::new(channels as usize, WindowType::Vorbis);
        let psy_models: Vec<_> = (0..channels)
            .map(|_| PsychoacousticModel::new(sample_rate, fft_size))
            .collect();

        Self {
            sample_rate,
            channels,
            mdct,
            psy_models,
            quality: quality.clamp(0.0, 1.0),
            block_size,
        }
    }

    /// Set quality (0.0-1.0)
    pub fn set_quality(&mut self, quality: f32) {
        self.quality = quality.clamp(0.0, 1.0);
    }

    /// Encode a frame of audio
    /// Input: interleaved samples for one frame (block_size * channels)
    /// Returns encoded frame
    pub fn encode_frame(&mut self, samples: &[f32]) -> TransformFrame {
        let block_samples = self.block_size.samples();
        let num_coeffs = self.block_size.coefficients();
        let hop_size = num_coeffs; // 50% overlap

        // Deinterleave channels
        let mut channel_data: Vec<Vec<f32>> = (0..self.channels as usize)
            .map(|_| Vec::with_capacity(samples.len() / self.channels as usize))
            .collect();

        for (i, &s) in samples.iter().enumerate() {
            channel_data[i % self.channels as usize].push(s);
        }

        let mut all_coefficients = Vec::with_capacity(self.channels as usize);
        let mut all_scale_factors = Vec::with_capacity(self.channels as usize);

        for (ch, data) in channel_data.iter().enumerate() {
            // Pad to block size if needed
            let mut frame_data = data.clone();
            if frame_data.len() < block_samples {
                frame_data.resize(block_samples, 0.0);
            }

            // MDCT transform
            let coeffs = self.mdct.forward(&frame_data, self.block_size);

            // Psychoacoustic analysis
            let smr = self.psy_models[ch].calculate_smr(&coeffs);

            // Quantize based on perceptual importance
            let (quantized, scale_factors) = self.quantize_coefficients(&coeffs, &smr);

            all_coefficients.push(quantized);
            all_scale_factors.push(scale_factors);
        }

        TransformFrame {
            coefficients: all_coefficients,
            scale_factors: all_scale_factors,
            block_size: self.block_size,
            num_samples: hop_size,
        }
    }

    /// Quantize MDCT coefficients based on SMR
    pub fn quantize_coefficients(&self, coeffs: &[f32], smr: &[f32]) -> (Vec<i16>, Vec<f32>) {
        // Calculate scale factors per Bark band
        let mut band_max = [0.0f32; NUM_BARK_BANDS];
        let freq_resolution = self.sample_rate as f32 / self.block_size.samples() as f32;

        for (k, &c) in coeffs.iter().enumerate() {
            let freq = (k as f32 + 0.5) * freq_resolution;
            let band = PsychoacousticModel::freq_to_bark_band(freq);
            band_max[band] = band_max[band].max(c.abs());
        }

        // Calculate scale factors (to fit i16 range without clipping)
        let mut scale_factors = vec![1.0f32; NUM_BARK_BANDS];
        for (sf, &max_val) in scale_factors.iter_mut().zip(band_max.iter()) {
            if max_val > 1e-10 {
                // Use 30000 as max to leave some headroom
                *sf = 30000.0 / max_val;
            }
        }

        // Quality-dependent masking threshold
        let smr_threshold = if self.quality >= 0.99 {
            -100.0 // At max quality, keep essentially everything
        } else {
            // Exponential decay from 0 dB at quality=0 to -60 dB at quality=1
            let t = (1.0 - self.quality).max(0.001);
            -60.0 * (1.0 - t.powf(0.5))
        };

        // Quantize
        let mut quantized = vec![0i16; coeffs.len()];

        for (k, (q, &c)) in quantized.iter_mut().zip(coeffs.iter()).enumerate() {
            let freq = (k as f32 + 0.5) * freq_resolution;
            let band = PsychoacousticModel::freq_to_bark_band(freq);

            if smr[k] > smr_threshold {
                // Above masking threshold, quantize with appropriate precision
                let scaled = c * scale_factors[band];
                *q = scaled.round().clamp(I16_MIN_F32, I16_MAX_F32) as i16;
            }
            // else: below threshold, leave as 0
        }

        (quantized, scale_factors)
    }

    /// Reset encoder state
    pub fn reset(&mut self) {
        self.mdct.reset();
        for model in &mut self.psy_models {
            model.reset();
        }
    }

    /// Encode audio samples to flo™ file format
    ///
    /// This produces a complete flo™ file with transform-based frames
    pub fn encode_to_flo(&mut self, samples: &[f32], metadata: &[u8]) -> crate::FloResult<Vec<u8>> {
        let block_samples = self.block_size.samples();
        let hop_size = self.block_size.coefficients(); // 50% overlap (N = block_samples/2)

        // For proper MDCT overlap-add reconstruction, we need:
        // - A priming frame at the start (silence) to initialize overlap buffer
        // - Proper number of frames to cover all samples
        let num_samples_per_channel = samples.len() / self.channels as usize;

        // Add hop_size samples of pre-roll (zeros) at start for proper reconstruction
        let pre_roll = hop_size;
        let total_samples = num_samples_per_channel + pre_roll;
        let num_hops = total_samples.div_ceil(hop_size);
        let total_samples_needed = (num_hops + 1) * hop_size;

        // Create padded buffer with pre-roll zeros at start
        let mut padded = vec![0.0f32; total_samples_needed * self.channels as usize];

        // Copy original samples after pre-roll
        for ch in 0..self.channels as usize {
            for i in 0..num_samples_per_channel.min(total_samples_needed - pre_roll) {
                let src_idx = i * self.channels as usize + ch;
                let dst_idx = (i + pre_roll) * self.channels as usize + ch;
                if src_idx < samples.len() && dst_idx < padded.len() {
                    padded[dst_idx] = samples[src_idx];
                }
            }
        }

        // Encode frames
        let mut encoded_frames: Vec<Frame> = Vec::new();

        // Process overlapping blocks
        for hop_idx in 0..num_hops {
            let start = hop_idx * hop_size * self.channels as usize;
            let end = start + block_samples * self.channels as usize;

            if end > padded.len() {
                break;
            }

            let frame_samples = &padded[start..end];
            let transform_frame = self.encode_frame(frame_samples);

            // Serialize the transform frame
            let frame_data = serialize_frame(&transform_frame);

            // Create a flo Frame with transform type
            let mut flo_frame = Frame::new(FrameType::Transform as u8, hop_size as u32);
            flo_frame.channels.push(ChannelData {
                predictor_coeffs: vec![],
                shift_bits: 0,
                residual_encoding: ResidualEncoding::Raw,
                rice_parameter: 0,
                residuals: frame_data,
            });

            encoded_frames.push(flo_frame);
        }

        // Write using the standard Writer
        let writer = crate::Writer::new();
        writer.write_ex(
            self.sample_rate,
            self.channels,
            16,                                          // bit_depth for lossy
            5,    // compression level (not used for transform)
            true, // is_lossy
            ((self.quality * 4.0).round() as u8).min(4), // quality as 0-4
            &encoded_frames,
            metadata,
        )
    }
}

/// Serialize a transform frame to bytes (optimized)
pub fn serialize_frame(frame: &TransformFrame) -> Vec<u8> {
    let mut data = Vec::new();

    // Block size (1 byte)
    data.push(match frame.block_size {
        BlockSize::Long => 0,
        BlockSize::Short => 1,
        BlockSize::Start => 2,
        BlockSize::Stop => 3,
    });

    // Number of channels (1 byte)
    data.push(frame.coefficients.len() as u8);

    // Scale factors per channel (25 bands * 2 bytes * channels)
    // Encode as log scale u16 instead of f32 to save space
    for sf in &frame.scale_factors {
        for &s in sf {
            // Convert to log scale: log2(sf) * 256 + 32768
            let log_sf = if s > 1e-10 {
                ((s.log2() * 256.0) + 32768.0).clamp(0.0, 65535.0) as u16
            } else {
                0
            };
            data.extend_from_slice(&log_sf.to_le_bytes());
        }
    }

    // Coefficients per channel (sparse encoding for mostly-zeros)
    for quantized in &frame.coefficients {
        let encoded = serialize_sparse(quantized);
        let len = encoded.len() as u32;
        data.extend_from_slice(&len.to_le_bytes());
        data.extend_from_slice(&encoded);
    }

    data
}

/// Encode coefficients using sparse run-length encoding
/// Format: [zero_count_varint] [non_zero_count] [values...]
pub fn serialize_sparse(coeffs: &[i16]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut i = 0;

    while i < coeffs.len() {
        // Count leading zeros
        let zero_start = i;
        while i < coeffs.len() && coeffs[i] == 0 {
            i += 1;
        }
        let zero_count = i - zero_start;

        // Count non-zeros (up to 255)
        let non_zero_start = i;
        while i < coeffs.len() && coeffs[i] != 0 && (i - non_zero_start) < 255 {
            i += 1;
        }
        let non_zero_count = i - non_zero_start;

        // Encode run: [zero_count_varint] [non_zero_count] [values...]
        encode_varint(&mut output, zero_count as u32);
        output.push(non_zero_count as u8);

        // Write non-zero values as i16 LE
        for j in non_zero_start..non_zero_start + non_zero_count {
            output.extend_from_slice(&coeffs[j].to_le_bytes());
        }
    }

    output
}

/// Encode a u32 as varint (1-5 bytes)
fn encode_varint(output: &mut Vec<u8>, mut value: u32) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        output.push(byte);
        if value == 0 {
            break;
        }
    }
}
