//! Lossless encoding for flo™
//!
//! Uses Adaptive Linear Predictive Coding (ALPC) with rice/entropy coding.
//! Achieves 2-3x compression on typical audio while preserving every bit.

pub mod decoder;
pub mod encoder;
pub mod lpc;

pub use lpc::{
    // Integer-based LPC (for encoding)
    autocorr_int,
    // Float-based LPC (for analysis)
    autocorrelation,
    calc_residuals_int,
    calculate_residuals,
    dequantize_coefficients,
    fixed_predictor_residuals,
    is_stable,
    is_stable_after_quantization,
    levinson_durbin,
    levinson_durbin_int,
    quantize_coefficients,
    reconstruct_samples,
};

pub use decoder::Decoder;
pub use encoder::Encoder;
