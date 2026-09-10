//! LPC algorithm tests for libflo

use libflo_audio::core::rice;
use libflo_audio::lossless::lpc::*;

// ============================================================================
// Autocorrelation Tests
// ============================================================================

#[test]
fn test_autocorrelation() {
    // Simple test signal: constant
    let constant: Vec<f32> = vec![1.0; 100];
    let ac = autocorrelation(&constant, 4);

    // For constant signal, all lags should be equal (within numerical precision)
    assert_eq!(ac.len(), 5); // 0..=4
                             // R[0] should be the largest (energy)
    assert!(ac[0] > 0.0);
}

#[test]
fn test_autocorrelation_sine() {
    // Sine wave should have periodic autocorrelation
    let sine: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.1).sin()).collect();
    let ac = autocorrelation(&sine, 10);

    // R[0] should be the largest
    for lag in 1..=10 {
        assert!(ac[0] >= ac[lag].abs());
    }
}

// ============================================================================
// Quantization Tests
// ============================================================================

#[test]
fn test_quantize_dequantize() {
    let coeffs = vec![0.5, -0.3, 0.1, -0.05];
    let (quantized, shift) = quantize_coefficients(&coeffs);
    let dequantized = dequantize_coefficients(&quantized, shift);

    // Should be close to original
    for (orig, deq) in coeffs.iter().zip(dequantized.iter()) {
        assert!((orig - deq).abs() < 0.01, "Quantization error too large");
    }
}

// ============================================================================
// Stability Tests
// ============================================================================

#[test]
fn test_stability_check() {
    // Stable filter
    let stable = vec![0.5, -0.2, 0.1];
    assert!(is_stable(&stable));

    // Unstable filter (coefficients too large)
    let unstable = vec![1.5, -0.9, 0.8];
    assert!(!is_stable(&unstable));
}

// ============================================================================
// Residual Tests
// ============================================================================

#[test]
fn test_residuals_reconstruction() {
    // Test that residuals can reconstruct the signal
    let original: Vec<f32> = (0..100).map(|i| (i as f32 * 0.05).sin()).collect();

    // Simple predictor
    let coeffs = vec![0.9, -0.2];

    let residuals = calculate_residuals(&original, &coeffs);
    let reconstructed = reconstruct_samples(&coeffs, &residuals, original.len());

    // Should match closely
    for (orig, rec) in original.iter().zip(reconstructed.iter()) {
        assert!((orig - rec).abs() < 1e-4, "Reconstruction failed");
    }
}

// ============================================================================
// Integer LPC Tests
// ============================================================================

#[test]
fn test_autocorr_int() {
    let samples: Vec<i32> = (0..100).map(|i| (i * 100) % 32767).collect();
    let ac = autocorr_int(&samples, 4);

    assert_eq!(ac.len(), 5);
    // R[0] should be >= all other values
    for i in 1..5 {
        assert!(ac[0] >= ac[i].abs());
    }
}

#[test]
fn test_fixed_predictor_order_0() {
    let samples = vec![100i32, 200, 300, 400, 500];
    let residuals = fixed_predictor_residuals(&samples, 0);

    // Order 0: residuals == samples
    assert_eq!(residuals, samples);
}

#[test]
fn test_fixed_predictor_order_1() {
    let samples = vec![100i32, 200, 300, 400, 500];
    let residuals = fixed_predictor_residuals(&samples, 1);

    // Order 1: residuals[i] = samples[i] - samples[i-1]
    assert_eq!(residuals[0], 100); // First sample unchanged
    assert_eq!(residuals[1], 100); // 200 - 100
    assert_eq!(residuals[2], 100); // 300 - 200
}

#[test]
fn test_rice_parameter_estimation() {
    // Small residuals should give small k
    let small: Vec<i32> = vec![0, 1, -1, 2, -2, 1, 0, -1];
    let k_small = rice::estimate_rice_parameter_i32(&small);

    // Large residuals should give larger k
    let large: Vec<i32> = vec![1000, -2000, 1500, -1800, 2200];
    let k_large = rice::estimate_rice_parameter_i32(&large);

    assert!(k_large > k_small);
}
