//! High-level tests that verify the full encode/decode pipeline works correctly.

use libflo_audio::{decode, encode, info, validate, version};

// ============================================================================
// Version Tests
// ============================================================================

#[test]
fn test_version() {
    assert_eq!(version(), "1.2");
}

// ============================================================================
// Validation Tests
// ============================================================================

#[test]
fn test_validate_good_file() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let flo_data = encode(&samples, 44100, 1, 16, None).unwrap();

    assert!(validate(&flo_data).unwrap());
}

#[test]
fn test_validate_bad_magic() {
    let bad_data = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(!validate(&bad_data).unwrap());
}

#[test]
fn test_validate_empty() {
    assert!(!validate(&[]).unwrap());
}

#[test]
fn test_validate_truncated() {
    let samples: Vec<f32> = (0..44100).map(|i| (i as f32 * 0.01).sin()).collect();
    let flo_data = encode(&samples, 44100, 1, 16, None).unwrap();

    let truncated = &flo_data[..flo_data.len() / 2];
    assert!(!validate(truncated).unwrap());
}

// ============================================================================
// Info Tests
// ============================================================================

#[test]
fn test_info() {
    let sample_rate = 48000u32;
    let channels = 2u8;
    let samples: Vec<f32> = vec![0.1; sample_rate as usize * channels as usize * 3];

    let flo_data = encode(&samples, sample_rate, channels, 24, None).unwrap();
    let file_info = info(&flo_data).unwrap();

    assert_eq!(file_info.sample_rate, sample_rate);
    assert_eq!(file_info.channels, channels);
    assert_eq!(file_info.bit_depth, 24);
    // 3 seconds of audio at 48000Hz stereo = 288000 samples
    // total_frames = duration in seconds = 3
    assert_eq!(file_info.total_frames, 3);
    assert!(file_info.crc_valid);
    assert!(file_info.compression_ratio > 1.0);
}

// ============================================================================
// Full Pipeline Tests
// ============================================================================

#[test]
fn test_full_pipeline_mono() {
    let sample_rate = 44100u32;
    let channels = 1u8;

    let samples: Vec<f32> = (0..(sample_rate as usize * 2))
        .map(|i| (i as f32 * 0.01).sin() * 0.5)
        .collect();

    let flo_data = encode(&samples, sample_rate, channels, 16, None).expect("Encoding failed");

    assert!(validate(&flo_data).unwrap());

    let decoded = decode(&flo_data).expect("Decoding failed");
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_full_pipeline_stereo() {
    let sample_rate = 44100u32;
    let channels = 2u8;

    let mut samples = Vec::with_capacity(sample_rate as usize * 2);
    for i in 0..sample_rate as usize {
        samples.push((i as f32 * 0.01).sin() * 0.5);
        samples.push((i as f32 * 0.01).cos() * 0.5);
    }

    let flo_data = encode(&samples, sample_rate, channels, 16, None).expect("Encoding failed");

    assert!(validate(&flo_data).unwrap());

    let decoded = decode(&flo_data).expect("Decoding failed");
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_different_sample_rates() {
    for rate in [22050, 44100, 48000, 96000] {
        let samples: Vec<f32> = (0..rate as usize)
            .map(|i| (i as f32 * 0.01).sin())
            .collect();

        let flo_data = encode(&samples, rate, 1, 16, None)
            .unwrap_or_else(|_| panic!("Encoding failed for rate {}", rate));

        let file_info = info(&flo_data).unwrap();
        assert_eq!(file_info.sample_rate, rate);
    }
}

#[test]
fn test_very_short_audio() {
    // Edge case: very short audio
    let samples: Vec<f32> = vec![0.5, -0.5, 0.3, -0.3];

    let flo_data = encode(&samples, 44100, 1, 16, None).expect("Encoding failed");

    let decoded = decode(&flo_data).expect("Decoding failed");
    assert_eq!(decoded.len(), samples.len());
}
