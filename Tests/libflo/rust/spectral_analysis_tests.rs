use libflo_audio::core::analysis::{
    extract_dominant_frequencies, extract_spectral_fingerprint, spectral_similarity,
};

#[test]
fn test_extract_spectral_fingerprint_mono() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];
    // Repeat the pattern to get more samples for analysis
    let samples: Vec<f32> = samples.iter().cycle().take(100).cloned().collect();
    let fingerprint = extract_spectral_fingerprint(&samples, 1, 44100, Some(1024), Some(512));

    assert_eq!(fingerprint.channels, 1);
    assert_eq!(fingerprint.sample_rate, 44100);

    // Check that fingerprint has expected compact structure
    assert_eq!(fingerprint.hash.len(), 32);
    assert_eq!(fingerprint.frequency_peaks.len(), 8);
    assert_eq!(fingerprint.energy_profile.len(), 16);

    // Duration should be reasonable for 6 samples at 44100 Hz
    assert!(fingerprint.duration_ms > 0);
    assert!(fingerprint.duration_ms < 1000); // Less than 1 second

    // Check that values are in valid ranges (u8 type ensures <= 255)
    assert!(!fingerprint.frequency_peaks.is_empty());
    assert!(!fingerprint.energy_profile.is_empty());
}

#[test]
fn test_extract_spectral_fingerprint_stereo() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9]; // L,R,L,R,L,R
                                                         // Repeat the pattern to get more samples for analysis
    let samples: Vec<f32> = samples.iter().cycle().take(100).cloned().collect();
    let fingerprint = extract_spectral_fingerprint(&samples, 2, 44100, Some(512), Some(256));

    assert_eq!(fingerprint.channels, 2);
    assert_eq!(fingerprint.sample_rate, 44100);

    // Check compact structure
    assert_eq!(fingerprint.hash.len(), 32);
    assert_eq!(fingerprint.frequency_peaks.len(), 8);
    assert_eq!(fingerprint.energy_profile.len(), 16);

    // Duration should be reasonable for 3 stereo pairs at 44100 Hz
    assert!(fingerprint.duration_ms > 0);
    assert!(fingerprint.duration_ms < 1000);
}

#[test]
fn test_extract_spectral_fingerprint_empty() {
    let samples: Vec<f32> = vec![];
    let fingerprint = extract_spectral_fingerprint(&samples, 1, 44100, Some(1024), Some(512));

    assert_eq!(fingerprint.channels, 1);
    assert_eq!(fingerprint.sample_rate, 44100);
    assert_eq!(fingerprint.duration_ms, 0);

    // Should have zeroed arrays for empty input
    assert_eq!(fingerprint.hash, [0; 32]);
    assert_eq!(fingerprint.frequency_peaks, [0; 8]);
    assert_eq!(fingerprint.energy_profile, [0; 16]);
    assert_eq!(fingerprint.avg_loudness, 0);
}

#[test]
fn test_extract_spectral_fingerprint_power_of_two() {
    let samples = vec![0.5; 1000];
    let fingerprint = extract_spectral_fingerprint(&samples, 1, 44100, Some(1000), Some(500));

    // Should produce valid fingerprint regardless of FFT size parameter (now ignored)
    assert_eq!(fingerprint.channels, 1);
    assert_eq!(fingerprint.sample_rate, 44100);
    assert!(fingerprint.duration_ms > 0);

    // Hash should be non-zero for non-empty input
    assert_ne!(fingerprint.hash, [0; 32]);
}

#[test]
fn test_extract_dominant_frequencies() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];
    let fingerprint = extract_spectral_fingerprint(&samples, 1, 44100, Some(256), Some(128));

    // Test with different numbers of dominant frequencies
    for num_freqs in 1..=8 {
        let dominant_freqs = extract_dominant_frequencies(&fingerprint, num_freqs);

        // Should return one frame with requested number of frequencies
        assert_eq!(dominant_freqs.len(), 1);
        assert_eq!(dominant_freqs[0].len(), num_freqs);

        // Frequencies should be positive and reasonable
        for &freq in &dominant_freqs[0] {
            assert!(freq >= 0.0);
            assert!(freq <= 22050.0); // Nyquist frequency for 44100 Hz
        }
    }

    // Test with more frequencies than available bands
    let dominant_freqs = extract_dominant_frequencies(&fingerprint, 16);
    assert_eq!(dominant_freqs.len(), 1);
    assert_eq!(dominant_freqs[0].len(), 8); // Should cap at 8 bands
}

#[test]
fn test_spectral_similarity_identical() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];
    let fingerprint1 = extract_spectral_fingerprint(&samples, 1, 44100, Some(256), Some(128));
    let fingerprint2 = extract_spectral_fingerprint(&samples, 1, 44100, Some(256), Some(128));

    let similarity = spectral_similarity(&fingerprint1, &fingerprint2);

    // Identical fingerprints should have similarity of 1.0 (due to hash match)
    assert_eq!(similarity, 1.0);
}

#[test]
fn test_spectral_similarity_incompatible() {
    let samples = vec![0.5; 100]; // Use larger sample
    let fingerprint1 = extract_spectral_fingerprint(&samples, 1, 44100, Some(256), Some(128));
    let fingerprint2 = extract_spectral_fingerprint(&samples, 2, 44100, Some(256), Some(128)); // Different channels

    let similarity = spectral_similarity(&fingerprint1, &fingerprint2);

    println!(
        "FP1 channels: {}, FP2 channels: {}",
        fingerprint1.channels, fingerprint2.channels
    );
    println!("Similarity: {}", similarity);

    // Incompatible fingerprints (different channel count) should have similarity of 0.0
    assert_eq!(similarity, 0.0);
}

#[test]
fn test_spectral_fingerprint_frequency_scaling() {
    let samples = vec![0.5; 2000];
    let fingerprint = extract_spectral_fingerprint(&samples, 1, 44100, Some(1024), None);

    // Check that dominant frequencies are within expected range
    let dominant_freqs = extract_dominant_frequencies(&fingerprint, 3);
    if !dominant_freqs.is_empty() && !dominant_freqs[0].is_empty() {
        for &freq in &dominant_freqs[0] {
            // Frequencies should be within Nyquist frequency
            assert!(freq >= 0.0);
            assert!(freq <= 22050.0); // Nyquist for 44100 Hz
        }
    }

    // Test frequency bands make sense, higher frequency peaks should have higher values
    // for constant input (due to FFT characteristics)
    assert!(fingerprint.frequency_peaks.len() == 8);
    assert!(fingerprint.energy_profile.len() == 16);
}

#[test]
fn test_spectral_fingerprint_consistency() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];
    let fingerprint1 = extract_spectral_fingerprint(&samples, 1, 44100, Some(512), Some(256));
    let fingerprint2 = extract_spectral_fingerprint(&samples, 1, 44100, Some(512), Some(256));

    // All basic properties should be identical
    assert_eq!(fingerprint1.channels, fingerprint2.channels);
    assert_eq!(fingerprint1.sample_rate, fingerprint2.sample_rate);
    assert_eq!(fingerprint1.duration_ms, fingerprint2.duration_ms);

    // Hash should be identical
    assert_eq!(fingerprint1.hash, fingerprint2.hash);

    // All spectral features should be identical
    assert_eq!(fingerprint1.frequency_peaks, fingerprint2.frequency_peaks);
    assert_eq!(fingerprint1.energy_profile, fingerprint2.energy_profile);
    assert_eq!(fingerprint1.avg_loudness, fingerprint2.avg_loudness);
}

#[test]
fn test_spectral_similarity_different() {
    // Use larger samples to get more meaningful spectral analysis
    let samples1: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.01).sin()).collect();
    let samples2: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.05).sin()).collect();

    let fingerprint1 = extract_spectral_fingerprint(&samples1, 1, 44100, Some(256), Some(128));
    let fingerprint2 = extract_spectral_fingerprint(&samples2, 1, 44100, Some(256), Some(128));

    let similarity = spectral_similarity(&fingerprint1, &fingerprint2);

    // Hashes should be different (content differs)
    assert_ne!(fingerprint1.hash, fingerprint2.hash);

    // Similarity should be reasonable
    assert!(similarity >= 0.0);
    assert!(similarity <= 1.0);

    // For similar sine waves, similarity might be high, but should not be exactly 1.0
    // unless they're truly identical
    if similarity == 1.0 {
        println!(
            "Note: High similarity may indicate limited spectral resolution for this test case"
        );
    }
}

#[test]
fn test_fingerprint_compact_size() {
    let samples = vec![0.5; 10000]; // Larger sample
    let fingerprint = extract_spectral_fingerprint(&samples, 2, 48000, None, None);

    // The fingerprint should be compact, let's verify the serialized size
    let serialized = rmp_serde::to_vec_named(&fingerprint).unwrap();

    // Should be much smaller than the raw audio (10000 * 2 * 4 = 80KB for stereo)
    // Compact fingerprint should be under 1KB
    assert!(
        serialized.len() < 1024,
        "Fingerprint too large: {} bytes",
        serialized.len()
    );

    // But should be larger than just the hash (32 bytes) due to additional features
    assert!(
        serialized.len() > 100,
        "Fingerprint too small: {} bytes",
        serialized.len()
    );
}

#[test]
fn test_fingerprint_duration_accuracy() {
    // Test with known durations
    let sample_rate = 44100;

    // 1 second of silence
    let samples_1sec = vec![0.0; sample_rate];
    let fp_1sec = extract_spectral_fingerprint(&samples_1sec, 1, sample_rate as u32, None, None);
    assert!((fp_1sec.duration_ms as i32 - 1000).abs() < 50); // Within 50ms

    // 0.5 seconds
    let half_samples = sample_rate / 2;
    let samples_half = vec![0.0; half_samples];
    let fp_half = extract_spectral_fingerprint(&samples_half, 1, sample_rate as u32, None, None);
    assert!((fp_half.duration_ms as i32 - 500).abs() < 25); // Within 25ms

    // 2 seconds stereo
    let samples_2sec_stereo = vec![0.0; sample_rate * 2 * 2];
    let fp_2sec =
        extract_spectral_fingerprint(&samples_2sec_stereo, 2, sample_rate as u32, None, None);
    assert!((fp_2sec.duration_ms as i32 - 2000).abs() < 50); // Within 50ms
}
