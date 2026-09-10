use super::encoder::TransformFrame;
use super::mdct::{BlockSize, Mdct, WindowType};
use super::psychoacoustic::{PsychoacousticModel, NUM_BARK_BANDS};

/// Transform lossy decoder
pub struct TransformDecoder {
    /// Sample rate
    sample_rate: u32,
    /// Number of channels
    channels: u8,
    /// MDCT processor
    mdct: Mdct,
}

impl TransformDecoder {
    /// Create a new transform decoder
    pub fn new(sample_rate: u32, channels: u8) -> Self {
        let mdct = Mdct::new(channels as usize, WindowType::Vorbis);

        Self {
            sample_rate,
            channels,
            mdct,
        }
    }

    /// Decode a frame
    /// Returns interleaved samples
    pub fn decode_frame(&mut self, frame: &TransformFrame) -> Vec<f32> {
        let freq_resolution = self.sample_rate as f32 / frame.block_size.samples() as f32;

        // Dequantize coefficients
        let mut dequantized: Vec<Vec<f32>> = Vec::with_capacity(self.channels as usize);

        for (ch, quantized) in frame.coefficients.iter().enumerate() {
            let mut coeffs = vec![0.0f32; quantized.len()];

            for (k, (&q, c)) in quantized.iter().zip(coeffs.iter_mut()).enumerate() {
                let freq = (k as f32 + 0.5) * freq_resolution;
                let band = PsychoacousticModel::freq_to_bark_band(freq);

                if frame.scale_factors[ch][band] > 0.0 {
                    *c = q as f32 / frame.scale_factors[ch][band];
                }
            }

            dequantized.push(coeffs);
        }

        // IMDCT + overlap-add
        self.mdct.synthesize(&dequantized, frame.block_size)
    }

    /// Reset decoder state
    pub fn reset(&mut self) {
        self.mdct.reset();
    }
}

/// Deserialize a transform frame from bytes
pub fn deserialize_frame(data: &[u8]) -> Option<TransformFrame> {
    if data.len() < 2 {
        return None;
    }

    let mut pos = 0;

    // Block size
    let block_size = match data[pos] {
        0 => BlockSize::Long,
        1 => BlockSize::Short,
        2 => BlockSize::Start,
        3 => BlockSize::Stop,
        _ => return None,
    };
    pos += 1;

    // Derive num_coeffs from block size
    let num_coeffs = block_size.coefficients();

    // Number of channels
    let num_channels = data[pos] as usize;
    pos += 1;

    // Scale factors (stored as log-scale u16)
    let mut scale_factors = Vec::with_capacity(num_channels);
    for _ in 0..num_channels {
        let mut sf = vec![0.0f32; NUM_BARK_BANDS];
        for s in &mut sf {
            if pos + 2 > data.len() {
                return None;
            }
            let log_sf = u16::from_le_bytes(data[pos..pos + 2].try_into().ok()?);
            pos += 2;

            // Decode from log scale: 2^((log_sf - 32768) / 256)
            if log_sf > 0 {
                *s = 2.0f32.powf((log_sf as f32 - 32768.0) / 256.0);
            }
        }
        scale_factors.push(sf);
    }

    // Coefficients (sparse encoded)
    let mut coefficients = Vec::with_capacity(num_channels);
    for _ in 0..num_channels {
        // Length (4 bytes)
        if pos + 4 > data.len() {
            return None;
        }
        let len = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
        pos += 4;

        if pos + len > data.len() {
            return None;
        }

        // Sparse decode
        let quantized = deserialize_sparse(&data[pos..pos + len], num_coeffs);
        coefficients.push(quantized);

        pos += len;
    }

    Some(TransformFrame {
        coefficients,
        scale_factors,
        block_size,
        num_samples: block_size.coefficients(),
    })
}

/// Decode sparse coefficients
pub fn deserialize_sparse(data: &[u8], num_coeffs: usize) -> Vec<i16> {
    let mut output = vec![0i16; num_coeffs];
    let mut pos = 0;
    let mut out_idx = 0;

    while pos < data.len() && out_idx < num_coeffs {
        // Read zero count
        let (zero_count, bytes_read) = decode_varint(&data[pos..]);
        pos += bytes_read;

        // Skip zeros
        out_idx += zero_count as usize;

        if pos >= data.len() {
            break;
        }

        // Read non-zero count
        let non_zero_count = data[pos] as usize;
        pos += 1;

        // Read non-zero values
        for _ in 0..non_zero_count {
            if pos + 2 > data.len() || out_idx >= num_coeffs {
                break;
            }
            output[out_idx] = i16::from_le_bytes([data[pos], data[pos + 1]]);
            pos += 2;
            out_idx += 1;
        }
    }

    output
}

/// Decode varint, returns (value, bytes_read)
fn decode_varint(data: &[u8]) -> (u32, usize) {
    let mut value = 0u32;
    let mut shift = 0;
    let mut bytes_read = 0;

    for &byte in data {
        value |= ((byte & 0x7F) as u32) << shift;
        bytes_read += 1;
        if byte & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 32 {
            break;
        }
    }

    (value, bytes_read)
}
