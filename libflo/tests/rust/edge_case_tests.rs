//! Edge case and stability tests for flo™ audio codec
use libflo_audio::{Decoder, Encoder, Reader};

// Helper to encode and decode
fn roundtrip(samples: &[f32], sample_rate: u32, channels: u8, bit_depth: u8) -> Vec<f32> {
    let encoder = Encoder::new(sample_rate, channels, bit_depth);
    let flo_data = encoder.encode(samples, &[]).expect("Encoding failed");
    let decoder = Decoder::new();
    decoder.decode(&flo_data).expect("Decoding failed")
}

// Helper to just encode
fn encode_samples(samples: &[f32], sample_rate: u32, channels: u8, bit_depth: u8) -> Vec<u8> {
    let encoder = Encoder::new(sample_rate, channels, bit_depth);
    encoder.encode(samples, &[]).expect("Encoding failed")
}

// Helper to check if data is valid (can be parsed)
fn is_valid_flo(data: &[u8]) -> bool {
    let reader = Reader::new();
    reader.read(data).is_ok()
}

// ============================================================================
// Edge Case: Extreme Audio Values
// ============================================================================

#[test]
fn test_max_sample_values() {
    let samples: Vec<f32> = vec![1.0; 4096];
    let decoded = roundtrip(&samples, 44100, 1, 16);
    for &s in &decoded {
        assert!(s >= 0.99, "Max sample degraded too much: {}", s);
    }
}

#[test]
fn test_min_sample_values() {
    let samples: Vec<f32> = vec![-1.0; 4096];
    let decoded = roundtrip(&samples, 44100, 1, 16);
    for &s in &decoded {
        assert!(s <= -0.99, "Min sample degraded too much: {}", s);
    }
}

#[test]
fn test_alternating_extremes() {
    let samples: Vec<f32> = (0..4096)
        .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
        .collect();
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_near_zero_values() {
    let samples: Vec<f32> = vec![0.00001; 4096];
    let decoded = roundtrip(&samples, 44100, 1, 24);
    for &s in &decoded {
        assert!(s >= 0.0, "Near-zero became negative: {}", s);
    }
}

#[test]
fn test_dc_offset() {
    let samples: Vec<f32> = vec![0.5; 44100];
    let decoded = roundtrip(&samples, 44100, 1, 16);
    let avg: f32 = decoded.iter().sum::<f32>() / decoded.len() as f32;
    assert!((avg - 0.5).abs() < 0.01, "DC offset changed: {}", avg);
}

// ============================================================================
// Edge Case: Boundary Conditions
// ============================================================================

#[test]
fn test_single_sample() {
    let samples = vec![0.5f32];
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert!(!decoded.is_empty());
}

#[test]
fn test_two_samples() {
    let samples = vec![0.5f32, -0.5];
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert!(decoded.len() >= 2);
}

#[test]
fn test_exact_frame_boundary() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_frame_boundary_plus_one() {
    let samples: Vec<f32> = (0..44101).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_frame_boundary_minus_one() {
    let samples: Vec<f32> = (0..44099).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_prime_sample_count() {
    let samples: Vec<f32> = (0..9973).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

// ============================================================================
// Edge Case: Sample Rates
// ============================================================================

#[test]
fn test_8khz() {
    let samples: Vec<f32> = (0..8000).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 8000, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_16khz() {
    let samples: Vec<f32> = (0..16000).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 16000, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_22050hz() {
    let samples: Vec<f32> = (0..22050).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 22050, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_48khz() {
    let samples: Vec<f32> = (0..48000).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 48000, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_96khz() {
    let samples: Vec<f32> = (0..96000).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 96000, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_192khz() {
    let samples: Vec<f32> = (0..192000).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 192000, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_non_standard_sample_rate() {
    let samples: Vec<f32> = (0..12345).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 12345, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

// ============================================================================
// Edge Case: Channel Configurations
// ============================================================================

#[test]
fn test_mono() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_stereo() {
    let mut samples = Vec::with_capacity(44100 * 2);
    for i in 0..44100 {
        samples.push((i as f32 * 0.01).sin());
        samples.push((i as f32 * 0.01).cos());
    }
    let decoded = roundtrip(&samples, 44100, 2, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_5_1_surround() {
    let mut samples = Vec::with_capacity(44100 * 6);
    for i in 0..44100 {
        for ch in 0..6 {
            samples.push((i as f32 * 0.01 * (ch + 1) as f32).sin());
        }
    }
    let decoded = roundtrip(&samples, 44100, 6, 16);
    assert_eq!(decoded.len(), samples.len());
}

// ============================================================================
// Stability: Corrupted Data Handling
// ============================================================================

#[test]
fn test_corrupted_magic_bytes() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let mut flo_data = encode_samples(&samples, 44100, 1, 16);
    flo_data[0] = 0x00;
    flo_data[1] = 0x00;
    assert!(!is_valid_flo(&flo_data));
}

#[test]
fn test_corrupted_header() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let mut flo_data = encode_samples(&samples, 44100, 1, 16);
    if flo_data.len() > 10 {
        flo_data[10] ^= 0xFF;
    }
    let decoder = Decoder::new();
    let _ = decoder.decode(&flo_data);
}

#[test]
fn test_truncated_file_early() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let flo_data = encode_samples(&samples, 44100, 1, 16);
    let truncated = &flo_data[..flo_data.len().min(32)];
    assert!(!is_valid_flo(truncated));
}

#[test]
fn test_truncated_file_middle() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let flo_data = encode_samples(&samples, 44100, 1, 16);
    let truncated = &flo_data[..flo_data.len() / 2];
    assert!(!is_valid_flo(truncated));
}

#[test]
fn test_truncated_file_late() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let flo_data = encode_samples(&samples, 44100, 1, 16);
    let truncated = &flo_data[..flo_data.len() - 10];
    assert!(!is_valid_flo(truncated));
}

#[test]
fn test_random_corruption_in_data() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let mut flo_data = encode_samples(&samples, 44100, 1, 16);
    for i in (100..flo_data.len().saturating_sub(100)).step_by(100) {
        flo_data[i] ^= 0xAA;
    }
    let decoder = Decoder::new();
    let _ = decoder.decode(&flo_data);
}

// ============================================================================
// Stability: Network Dropout Simulation
// ============================================================================

#[test]
fn test_sudden_stream_cutoff() {
    let samples: Vec<f32> = (0..44100 * 5).map(|i| (i as f32 * 0.01).sin()).collect();
    let flo_data = encode_samples(&samples, 44100, 1, 16);
    for cutoff in [
        flo_data.len() / 10,
        flo_data.len() / 4,
        flo_data.len() / 2,
        flo_data.len() * 3 / 4,
    ] {
        let truncated = &flo_data[..cutoff];
        let _ = is_valid_flo(truncated);
    }
}

#[test]
fn test_missing_end_marker() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let mut flo_data = encode_samples(&samples, 44100, 1, 16);
    flo_data.truncate(flo_data.len().saturating_sub(4));
    assert!(!is_valid_flo(&flo_data));
}

#[test]
fn test_garbage_appended() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let mut flo_data = encode_samples(&samples, 44100, 1, 16);
    flo_data.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11, 0x22, 0x33]);
    let decoder = Decoder::new();
    let _ = decoder.decode(&flo_data);
}

// ============================================================================
// Stability: Invalid Input Data
// ============================================================================

#[test]
fn test_empty_input() {
    assert!(!is_valid_flo(&[]));
}

#[test]
fn test_single_byte_input() {
    assert!(!is_valid_flo(&[0x66]));
}

#[test]
fn test_just_magic_bytes() {
    let magic = b"flo\0";
    assert!(!is_valid_flo(magic));
}

#[test]
fn test_random_garbage() {
    let garbage: Vec<u8> = (0..1000).map(|i| (i * 17 % 256) as u8).collect();
    assert!(!is_valid_flo(&garbage));
}

#[test]
fn test_all_zeros() {
    let zeros = vec![0u8; 1000];
    assert!(!is_valid_flo(&zeros));
}

#[test]
fn test_all_ones() {
    let ones = vec![0xFFu8; 1000];
    assert!(!is_valid_flo(&ones));
}

// ============================================================================
// Edge Case: Silence Handling
// ============================================================================

#[test]
fn test_pure_silence() {
    let samples = vec![0.0f32; 44100 * 5];
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), samples.len());
    for &s in &decoded {
        assert!(s.abs() < 0.001, "Silence sample not zero: {}", s);
    }
}

#[test]
fn test_silence_with_single_click() {
    let mut samples = vec![0.0f32; 44100];
    samples[22050] = 1.0;
    let decoded = roundtrip(&samples, 44100, 1, 16);
    let max_val = decoded.iter().map(|&s| s.abs()).fold(0.0f32, f32::max);
    assert!(max_val > 0.5, "Click was lost: max={}", max_val);
}

#[test]
fn test_silence_at_start() {
    let mut samples = vec![0.0f32; 22050];
    samples.extend((0..22050).map(|i| (i as f32 * 0.1).sin()));
    let decoded = roundtrip(&samples, 44100, 1, 16);
    let first_half_max = decoded[..22050]
        .iter()
        .map(|&s| s.abs())
        .fold(0.0f32, f32::max);
    assert!(first_half_max < 0.01, "Silence at start wasn't preserved");
}

#[test]
fn test_silence_at_end() {
    let mut samples: Vec<f32> = (0..22050).map(|i| (i as f32 * 0.1).sin()).collect();
    samples.extend(vec![0.0f32; 22050]);
    let decoded = roundtrip(&samples, 44100, 1, 16);
    let second_half_max = decoded[22050..]
        .iter()
        .map(|&s| s.abs())
        .fold(0.0f32, f32::max);
    assert!(second_half_max < 0.01, "Silence at end wasn't preserved");
}

// ============================================================================
// Edge Case: Special Signals
// ============================================================================

#[test]
fn test_impulse_response() {
    let mut samples = vec![0.0f32; 4096];
    samples[0] = 1.0;
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert!(!decoded.is_empty());
}

#[test]
fn test_step_function() {
    let mut samples = vec![0.0f32; 2048];
    samples.extend(vec![1.0f32; 2048]);
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), 4096);
}

#[test]
fn test_square_wave() {
    let samples: Vec<f32> = (0..4096)
        .map(|i| if (i / 100) % 2 == 0 { 1.0 } else { -1.0 })
        .collect();
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_sawtooth_wave() {
    let samples: Vec<f32> = (0..4096).map(|i| ((i % 100) as f32 / 50.0) - 1.0).collect();
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

// ============================================================================
// Edge Case: NaN and Infinity
// ============================================================================

#[test]
fn test_nan_samples() {
    let samples = vec![f32::NAN; 100];
    let encoder = Encoder::new(44100, 1, 16);
    let _ = encoder.encode(&samples, &[]);
}

#[test]
fn test_infinity_samples() {
    let samples = vec![f32::INFINITY; 100];
    let encoder = Encoder::new(44100, 1, 16);
    let _ = encoder.encode(&samples, &[]);
}

#[test]
fn test_neg_infinity_samples() {
    let samples = vec![f32::NEG_INFINITY; 100];
    let encoder = Encoder::new(44100, 1, 16);
    let _ = encoder.encode(&samples, &[]);
}

#[test]
fn test_mixed_special_values() {
    let samples = vec![
        0.0,
        1.0,
        -1.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.5,
    ];
    let encoder = Encoder::new(44100, 1, 16);
    let _ = encoder.encode(&samples, &[]);
}

// ============================================================================
// Stress Test: Long Audio
// ============================================================================

#[test]
fn test_10_minutes_audio() {
    let sample_count = 44100 * 60 * 10;
    let samples: Vec<f32> = (0..sample_count)
        .map(|i| (i as f32 * 0.001).sin())
        .collect();
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_high_frequency_content() {
    let samples: Vec<f32> = (0..44100)
        .map(|i| (i as f32 * std::f32::consts::PI * 0.99).sin())
        .collect();
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_complex_waveform() {
    let samples: Vec<f32> = (0..44100)
        .map(|i| {
            let t = i as f32 / 44100.0;
            (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.3
                + (t * 880.0 * 2.0 * std::f32::consts::PI).sin() * 0.2
                + (t * 1320.0 * 2.0 * std::f32::consts::PI).sin() * 0.15
                + (t * 1760.0 * 2.0 * std::f32::consts::PI).sin() * 0.1
        })
        .collect();
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

// ============================================================================
// Bit Depth Edge Cases
// ============================================================================

#[test]
fn test_8bit() {
    let samples: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 44100, 1, 8);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_16bit() {
    let samples: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 44100, 1, 16);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_24bit() {
    let samples: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 44100, 1, 24);
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_32bit() {
    let samples: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin()).collect();
    let decoded = roundtrip(&samples, 44100, 1, 32);
    assert_eq!(decoded.len(), samples.len());
}

// ============================================================================
// Metadata Edge Cases
// ============================================================================

#[test]
fn test_empty_metadata() {
    let samples: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin()).collect();
    let encoder = Encoder::new(44100, 1, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();
    let decoder = Decoder::new();
    let decoded = decoder.decode(&flo_data).unwrap();
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_large_metadata() {
    let samples: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin()).collect();
    let metadata = vec![0x42u8; 64 * 1024];
    let encoder = Encoder::new(44100, 1, 16);
    let flo_data = encoder.encode(&samples, &metadata).unwrap();
    let decoder = Decoder::new();
    let decoded = decoder.decode(&flo_data).unwrap();
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_binary_metadata() {
    let samples: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.01).sin()).collect();
    let metadata: Vec<u8> = (0..256).map(|i| i as u8).collect();
    let encoder = Encoder::new(44100, 1, 16);
    let flo_data = encoder.encode(&samples, &metadata).unwrap();
    let decoder = Decoder::new();
    let decoded = decoder.decode(&flo_data).unwrap();
    assert_eq!(decoded.len(), samples.len());
}

// ============================================================================
// VERIFICATION: Make sure tests are actually checking data, not just lengths!
// ============================================================================

#[test]
fn test_verify_data_survives_not_just_length() {
    // Create distinctive pattern that would be obvious if corrupted
    let samples: Vec<f32> = (0..4096)
        .map(|i| {
            // A specific pattern: ramp + sine
            let ramp = (i as f32 / 4096.0) * 2.0 - 1.0;
            let sine = (i as f32 * 0.1).sin() * 0.5;
            (ramp + sine).clamp(-1.0, 1.0)
        })
        .collect();

    let decoded = roundtrip(&samples, 44100, 1, 16);

    // Check length
    assert_eq!(decoded.len(), samples.len());

    // Check correlation - decoded should match original closely
    let mut sum_diff_sq = 0.0f64;
    for (orig, dec) in samples.iter().zip(decoded.iter()) {
        let diff = (*orig as f64) - (*dec as f64);
        sum_diff_sq += diff * diff;
    }
    let mse = sum_diff_sq / samples.len() as f64;

    // For lossless, MSE should be very small (quantization noise only)
    assert!(
        mse < 0.0001,
        "MSE too high: {} - data may not have survived!",
        mse
    );
}

#[test]
fn test_surround_channels_are_independent() {
    // Create 6-channel audio where each channel has unique content
    let sample_count = 4096;
    let channels = 6u8;
    let mut samples = Vec::with_capacity(sample_count * channels as usize);

    for i in 0..sample_count {
        // Each channel gets a different frequency
        samples.push((i as f32 * 0.01).sin()); // Ch 0: low freq
        samples.push((i as f32 * 0.05).sin()); // Ch 1: medium freq
        samples.push((i as f32 * 0.10).sin()); // Ch 2: higher freq
        samples.push((i as f32 * 0.15).sin()); // Ch 3: LFE-ish
        samples.push((i as f32 * 0.02).cos()); // Ch 4: rear L
        samples.push((i as f32 * 0.02).sin()); // Ch 5: rear R
    }

    let decoded = roundtrip(&samples, 48000, channels, 16);

    // Verify length
    assert_eq!(decoded.len(), samples.len());

    // Verify each channel independently
    for ch in 0..channels as usize {
        let orig_ch: Vec<f32> = samples.iter().skip(ch).step_by(6).copied().collect();
        let dec_ch: Vec<f32> = decoded.iter().skip(ch).step_by(6).copied().collect();

        let mut sum_diff_sq = 0.0f64;
        for (o, d) in orig_ch.iter().zip(dec_ch.iter()) {
            let diff = (*o as f64) - (*d as f64);
            sum_diff_sq += diff * diff;
        }
        let mse = sum_diff_sq / orig_ch.len() as f64;
        assert!(
            mse < 0.0001,
            "Channel {} has high MSE: {} - channels may be mixed up!",
            ch,
            mse
        );
    }
}

#[test]
fn test_corruption_actually_detected() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let flo_data = encode_samples(&samples, 44100, 1, 16);

    // Make sure valid data IS valid
    assert!(is_valid_flo(&flo_data), "Valid data should be valid!");

    // Now corrupt it and make sure it's detected
    let mut corrupted = flo_data.clone();
    corrupted[0] = 0x00; // Break magic
    assert!(
        !is_valid_flo(&corrupted),
        "Corrupted magic should be detected!"
    );
}
