//! Lossless decoder tests for libflo

use libflo_audio::{decode, encode, Decoder};

// ============================================================================
// Decoder API Tests
// ============================================================================

#[test]
fn test_decoder_new() {
    let decoder = Decoder::new();

    // Create valid file first
    let samples: Vec<f32> = vec![0.5; 1000];
    let flo_data = encode(&samples, 44100, 1, 16, None).unwrap();

    let result = decoder.decode(&flo_data);
    assert!(result.is_ok());
}

#[test]
fn test_decoder_roundtrip_mono() {
    let sample_rate = 44100u32;
    let channels = 1u8;

    let samples: Vec<f32> = (0..sample_rate as usize)
        .map(|i| (i as f32 * 0.01).sin() * 0.5)
        .collect();

    let flo_data = encode(&samples, sample_rate, channels, 16, None).expect("Encoding failed");
    let decoded = decode(&flo_data).expect("Decoding failed");

    assert_eq!(decoded.len(), samples.len());
    verify_lossless(&samples, &decoded);
}

#[test]
fn test_decoder_roundtrip_stereo() {
    let sample_rate = 44100u32;
    let channels = 2u8;

    let mut samples = Vec::with_capacity(sample_rate as usize * 2);
    for i in 0..sample_rate as usize {
        samples.push((i as f32 * 0.01).sin() * 0.5); // Left
        samples.push((i as f32 * 0.01).cos() * 0.5); // Right
    }

    let flo_data = encode(&samples, sample_rate, channels, 16, None).expect("Encoding failed");
    let decoded = decode(&flo_data).expect("Decoding failed");

    assert_eq!(decoded.len(), samples.len());
    verify_lossless(&samples, &decoded);
}

#[test]
fn test_decoder_silence() {
    let samples: Vec<f32> = vec![0.0; 44100];

    let flo_data = encode(&samples, 44100, 1, 16, None).expect("Encoding failed");
    let decoded = decode(&flo_data).expect("Decoding failed");

    for &s in &decoded {
        assert!(s.abs() < 1e-6, "Silence not preserved");
    }
}

#[test]
fn test_decoder_extreme_values() {
    // Test full range
    let samples = vec![1.0f32, -1.0, 0.999, -0.999, 0.0, 0.5, -0.5];

    let flo_data = encode(&samples, 44100, 1, 16, None).expect("Encoding failed");
    let decoded = decode(&flo_data).expect("Decoding failed");

    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_decoder_short_audio() {
    let samples: Vec<f32> = (0..100).map(|i| (i as f32 * 0.1).sin() * 0.5).collect();

    let flo_data = encode(&samples, 44100, 1, 16, None).expect("Encoding failed");
    let decoded = decode(&flo_data).expect("Decoding failed");

    assert_eq!(decoded.len(), samples.len());
}

// ============================================================================
// Helper Functions
// ============================================================================

fn verify_lossless(original: &[f32], decoded: &[f32]) {
    // For 16-bit quantization, max error is 1/32768 ≈ 0.000031
    let max_quantization_error = 1.0 / 32768.0 + 0.000001; // Small epsilon

    let mut max_error = 0.0f32;
    for (orig, dec) in original.iter().zip(decoded.iter()) {
        let error = (orig - dec).abs();
        if error > max_error {
            max_error = error;
        }
    }

    assert!(
        max_error <= max_quantization_error,
        "Max error {} exceeds quantization limit {}",
        max_error,
        max_quantization_error
    );
}
