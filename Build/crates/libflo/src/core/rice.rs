// Rice coding implementation for residual compression

use super::audio_constants::{f32_to_i32, i32_to_f32, I16_MAX_F64};

/// Estimate optimal Rice parameter from float residuals
/// Residuals are expected to be in -1.0 to 1.0 range and will be scaled to 16-bit
pub fn estimate_rice_parameter(residuals: &[f32]) -> u8 {
    if residuals.is_empty() {
        return 10; // Good default for 16-bit audio
    }

    // Scale to 16-bit range and calculate mean absolute value
    let mean_abs: f64 = residuals
        .iter()
        .map(|&r| (r * I16_MAX_F64 as f32).abs() as f64)
        .sum::<f64>()
        / residuals.len() as f64;

    if mean_abs > 1.0 {
        // Rice parameter k where 2^k approximates mean_abs
        (mean_abs.log2().round() as u8).clamp(4, 14)
    } else {
        4
    }
}

/// Estimate Rice parameter from integer residuals
/// Ensures k is large enough that no quotient exceeds 255 during encoding
pub fn estimate_rice_parameter_i32(residuals: &[i32]) -> u8 {
    if residuals.is_empty() {
        return 4;
    }

    // Find maximum absolute value to ensure no overflow
    let max_abs = residuals
        .iter()
        .map(|&r| r.unsigned_abs())
        .max()
        .unwrap_or(0) as u64;

    if max_abs == 0 {
        return 0;
    }

    // Zigzag encoding doubles positive values: max_unsigned = 2 * max_abs
    let max_unsigned = 2 * max_abs;

    // quotient = unsigned >> k
    // We need quotient <= 255, so unsigned <= 255 << k
    // Therefore k >= log2(unsigned / 255)
    let min_k = if max_unsigned > 255 {
        let bits_needed = 64 - max_unsigned.leading_zeros();
        bits_needed.saturating_sub(8) as u8
    } else {
        0
    };

    // Also consider mean for efficiency
    let sum: u64 = residuals.iter().map(|&r| r.unsigned_abs() as u64).sum();
    let mean = (sum / residuals.len() as u64) as u32;
    let mean_k = if mean > 0 {
        (32 - mean.leading_zeros()) as u8
    } else {
        0
    };

    // Use the larger of min_k (for correctness) and mean_k (for efficiency)
    min_k.max(mean_k).clamp(0, 15)
}

/// Rice encode float residuals (quantizes to 16-bit)
pub fn encode(residuals: &[f32], k: u8) -> Vec<u8> {
    let mut bits = BitWriter::new();

    for &residual in residuals {
        let sample = f32_to_i32(residual);
        encode_sample(&mut bits, sample, k);
    }

    bits.into_bytes()
}

/// Rice encode integer residuals directly
pub fn encode_i32(residuals: &[i32], k: u8) -> Vec<u8> {
    let mut bits = BitWriter::new();

    for &sample in residuals {
        encode_sample(&mut bits, sample, k);
    }

    bits.into_bytes()
}

fn encode_sample(bits: &mut BitWriter, sample: i32, k: u8) {
    // Zigzag encode: map signed to unsigned
    // 0 → 0, -1 → 1, 1 → 2, -2 → 3, 2 → 4, ...
    let unsigned = ((sample << 1) ^ (sample >> 31)) as u32;

    // Rice coding: quotient and remainder
    let quotient = unsigned >> k;
    let remainder = unsigned & ((1 << k) - 1);

    // Unary code for quotient (capped to prevent huge outputs)
    let q_capped = quotient.min(255);
    for _ in 0..q_capped {
        bits.write_bit(1);
    }
    bits.write_bit(0);

    // Binary code for remainder
    for i in (0..k).rev() {
        bits.write_bit((remainder >> i) & 1);
    }
}

/// Rice decode to float residuals
pub fn decode(encoded: &[u8], k: u8, target_len: usize) -> Vec<f32> {
    let decoded_i32 = decode_i32(encoded, k, target_len);
    decoded_i32.iter().map(|&s| i32_to_f32(s)).collect()
}

/// Rice decode to integer residuals
pub fn decode_i32(encoded: &[u8], k: u8, target_len: usize) -> Vec<i32> {
    let mut bits = BitReader::new(encoded);
    let mut residuals = Vec::with_capacity(target_len);

    for _ in 0..target_len {
        if bits.is_exhausted() {
            residuals.push(0);
            continue;
        }

        // Read unary quotient
        let mut quotient = 0u32;
        while !bits.is_exhausted() && bits.read_bit() == 1 {
            quotient += 1;
            if quotient > 255 {
                break;
            }
        }

        // Read binary remainder
        let mut remainder = 0u32;
        for _ in 0..k {
            remainder = (remainder << 1) | bits.read_bit();
        }

        // Reconstruct unsigned value
        let unsigned = (quotient << k) | remainder;

        // Zigzag decode
        // 0 → 0, 1 → -1, 2 → 1, 3 → -2, 4 → 2, ...
        let signed = ((unsigned >> 1) as i32) ^ (-((unsigned & 1) as i32));

        residuals.push(signed);
    }

    residuals
}

/// Bit-level writer
pub struct BitWriter {
    bytes: Vec<u8>,
    current_byte: u8,
    bit_pos: u8,
}

impl BitWriter {
    pub fn new() -> Self {
        BitWriter {
            bytes: Vec::new(),
            current_byte: 0,
            bit_pos: 0,
        }
    }

    pub fn write_bit(&mut self, bit: u32) {
        if bit != 0 {
            self.current_byte |= 1 << (7 - self.bit_pos);
        }

        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bytes.push(self.current_byte);
            self.current_byte = 0;
            self.bit_pos = 0;
        }
    }

    #[allow(dead_code)]
    pub fn write_bits(&mut self, value: u32, num_bits: u8) {
        for i in (0..num_bits).rev() {
            self.write_bit((value >> i) & 1);
        }
    }

    pub fn into_bytes(mut self) -> Vec<u8> {
        if self.bit_pos > 0 {
            self.bytes.push(self.current_byte);
        }
        self.bytes
    }

    #[allow(dead_code)]
    pub fn byte_count(&self) -> usize {
        self.bytes.len() + if self.bit_pos > 0 { 1 } else { 0 }
    }
}

impl Default for BitWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Bit-level reader
pub struct BitReader<'a> {
    bytes: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> BitReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        BitReader {
            bytes,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    pub fn read_bit(&mut self) -> u32 {
        if self.byte_pos >= self.bytes.len() {
            return 0;
        }

        let bit = (self.bytes[self.byte_pos] >> (7 - self.bit_pos)) & 1;

        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }

        bit as u32
    }

    #[allow(dead_code)]
    pub fn read_bits(&mut self, num_bits: u8) -> u32 {
        let mut value = 0u32;
        for _ in 0..num_bits {
            value = (value << 1) | self.read_bit();
        }
        value
    }

    pub fn is_exhausted(&self) -> bool {
        self.byte_pos >= self.bytes.len()
    }

    #[allow(dead_code)]
    pub fn remaining_bytes(&self) -> usize {
        if self.byte_pos >= self.bytes.len() {
            0
        } else {
            self.bytes.len() - self.byte_pos
        }
    }
}
