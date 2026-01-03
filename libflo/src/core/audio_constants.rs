/// Maximum positive value for 16-bit signed integer (2^15 - 1)
pub const I16_MAX_F32: f32 = 32767.0;

/// Minimum value for 16-bit signed integer (-2^15)
pub const I16_MIN_F32: f32 = -32768.0;

/// Maximum absolute value for 16-bit signed integer as f64
pub const I16_MAX_F64: f64 = 32767.0;

/// Inverse of I16_MAX_F32, used for int→float conversion (1/32767)
pub const I16_TO_F32_SCALE: f32 = 1.0 / 32767.0;

/// Inverse of I16_MIN_F32 absolute value, used for alternate int→float (1/32768)
pub const I16_TO_F32_SCALE_ALT: f32 = 1.0 / 32768.0;

/// Convert f32 sample to i32 for processing
#[inline]
pub fn f32_to_i32(sample: f32) -> i32 {
    (sample * I16_MAX_F32).clamp(I16_MIN_F32, I16_MAX_F32) as i32
}

/// Convert i32 sample to f32
#[inline]
pub fn i32_to_f32(sample: i32) -> f32 {
    sample as f32 * I16_TO_F32_SCALE
}

/// Convert f32 sample to i16
#[inline]
pub fn f32_to_i16(sample: f32) -> i16 {
    (sample * I16_MAX_F32).clamp(I16_MIN_F32, I16_MAX_F32) as i16
}

/// Convert i16 sample to f32
#[inline]
pub fn i16_to_f32(sample: i16) -> f32 {
    sample as f32 * I16_TO_F32_SCALE
}
