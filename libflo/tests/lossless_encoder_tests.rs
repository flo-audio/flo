//! Lossless encoder tests for libflo

use libflo_audio::{encode, info, Encoder};

// ============================================================================
// Encoder API Tests
// ============================================================================

#[test]
fn test_encoder_new() {
    let encoder = Encoder::new(44100, 2, 16);
    let samples: Vec<f32> = vec![0.5; 44100 * 2];

    let result = encoder.encode(&samples, &[]);
    assert!(result.is_ok());
}

#[test]
fn test_encoder_with_compression() {
    let encoder = Encoder::new(44100, 1, 16).with_compression(9);
    let samples: Vec<f32> = vec![0.5; 44100];

    let result = encoder.encode(&samples, &[]);
    assert!(result.is_ok());
}

#[test]
fn test_encoder_with_metadata() {
    let encoder = Encoder::new(44100, 1, 16);
    let samples: Vec<f32> = vec![0.1; 44100];
    let metadata = b"test metadata bytes";

    let flo_data = encoder.encode(&samples, metadata).unwrap();
    assert!(!flo_data.is_empty());
}

#[test]
fn test_encoder_builder() {
    let encoder = Encoder::new(96000, 2, 32).with_compression(9);
    let samples: Vec<f32> = vec![0.5; 96000 * 2];

    let result = encoder.encode(&samples, &[]);
    assert!(result.is_ok());
}

// ============================================================================
// Sample Rate Tests
// ============================================================================

#[test]
fn test_sample_rate_44100() {
    test_sample_rate(44100);
}

#[test]
fn test_sample_rate_48000() {
    test_sample_rate(48000);
}

#[test]
fn test_sample_rate_96000() {
    test_sample_rate(96000);
}

#[test]
fn test_sample_rate_22050() {
    test_sample_rate(22050);
}

fn test_sample_rate(rate: u32) {
    let samples: Vec<f32> = (0..rate as usize)
        .map(|i| (i as f32 * 0.01).sin())
        .collect();

    let flo_data = encode(&samples, rate, 1, 16, None).expect("Encoding failed");
    let file_info = info(&flo_data).unwrap();

    assert_eq!(file_info.sample_rate, rate);
}

// ============================================================================
// Bit Depth Tests
// ============================================================================

#[test]
fn test_bit_depth_16() {
    test_bit_depth(16);
}

#[test]
fn test_bit_depth_24() {
    test_bit_depth(24);
}

#[test]
fn test_bit_depth_32() {
    test_bit_depth(32);
}

fn test_bit_depth(depth: u8) {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();

    let flo_data = encode(&samples, 44100, 1, depth, None).expect("Encoding failed");
    let file_info = info(&flo_data).unwrap();

    assert_eq!(file_info.bit_depth, depth);
}

// ============================================================================
// Compression Tests
// ============================================================================

#[test]
fn test_compression_ratio() {
    let sample_rate = 44100u32;
    let channels = 2u8;

    // Generate "realistic" audio (not just silence or pure sine)
    let mut samples = Vec::with_capacity(sample_rate as usize * 2);
    for i in 0..sample_rate as usize {
        let t = i as f32 / sample_rate as f32;
        let left = (440.0 * 2.0 * std::f32::consts::PI * t).sin() * 0.3
            + (880.0 * 2.0 * std::f32::consts::PI * t).sin() * 0.2;
        let right = (440.0 * 2.0 * std::f32::consts::PI * t).cos() * 0.3
            + (660.0 * 2.0 * std::f32::consts::PI * t).sin() * 0.2;
        samples.push(left);
        samples.push(right);
    }

    let flo_data = encode(&samples, sample_rate, channels, 16, None).unwrap();

    let raw_size = samples.len() * 2; // 16-bit = 2 bytes per sample
    let compressed_size = flo_data.len();
    let ratio = raw_size as f64 / compressed_size as f64;

    // Should achieve at least 2x compression on tonal content
    assert!(ratio > 2.0, "Compression ratio {} is too low", ratio);
}
