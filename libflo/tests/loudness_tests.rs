use libflo_audio::core::ebu_r128::compute_ebu_r128_loudness;

#[test]
fn test_ebu_r128_empty_samples() {
    let samples: Vec<f32> = vec![];
    let metrics = compute_ebu_r128_loudness(&samples, 1, 44100);

    // Should return default values for empty input
    assert_eq!(metrics.integrated_lufs, -23.0);
    assert_eq!(metrics.loudness_range_lu, 0.0);
    assert_eq!(metrics.true_peak_dbtp, -150.0);
    assert_eq!(metrics.sample_peak_dbfs, -150.0);
}

#[test]
fn test_ebu_r128_silence() {
    let samples = vec![0.0; 44100]; // 1 second of silence at 44.1kHz
    let metrics = compute_ebu_r128_loudness(&samples, 1, 44100);

    // Silence should have very low loudness
    assert!(metrics.integrated_lufs <= -23.0); // Default value when gated
    assert_eq!(metrics.loudness_range_lu, 0.0); // No range in silence
    assert_eq!(metrics.true_peak_dbtp, -150.0); // No peak
    assert_eq!(metrics.sample_peak_dbfs, -150.0); // No peak
}

#[test]
fn test_ebu_r128_mono_sine_wave() {
    let sample_rate = 44100;
    let frequency = 440.0; // A4 note
    let duration_samples = sample_rate; // 1 second
    let amplitude = 0.5; // Half scale (-6 dBFS)

    let samples: Vec<f32> = (0..duration_samples)
        .map(|i| {
            let phase = 2.0 * std::f32::consts::PI * frequency * i as f32 / sample_rate as f32;
            amplitude * phase.sin()
        })
        .collect();

    let metrics = compute_ebu_r128_loudness(&samples, 1, sample_rate);

    // Should have reasonable loudness values
    assert!(metrics.integrated_lufs > -50.0 && metrics.integrated_lufs < 0.0);
    assert!(metrics.loudness_range_lu >= 0.0);
    assert!(metrics.true_peak_dbtp > -10.0 && metrics.true_peak_dbtp <= 0.0);
    assert!(metrics.sample_peak_dbfs > -10.0 && metrics.sample_peak_dbfs <= 0.0);
}

#[test]
fn test_ebu_r128_stereo_sine_wave() {
    let sample_rate = 44100;
    let frequency = 440.0; // A4 note
    let duration_samples = sample_rate * 2; // 2 seconds
    let amplitude = 0.5; // Half scale (-6 dBFS)

    let samples: Vec<f32> = (0..duration_samples)
        .flat_map(|i| {
            let phase = 2.0 * std::f32::consts::PI * frequency * i as f32 / sample_rate as f32;
            let sample = amplitude * phase.sin();
            // Stereo interleaved
            vec![sample, sample]
        })
        .collect();

    let metrics = compute_ebu_r128_loudness(&samples, 2, sample_rate);

    // Should have reasonable loudness values
    assert!(metrics.integrated_lufs > -50.0 && metrics.integrated_lufs < 0.0);
    assert!(metrics.loudness_range_lu >= 0.0);
    assert!(metrics.true_peak_dbtp > -10.0 && metrics.true_peak_dbtp <= 0.0);
    assert!(metrics.sample_peak_dbfs > -10.0 && metrics.sample_peak_dbfs <= 0.0);
}

#[test]
fn test_ebu_r128_white_noise() {
    let sample_rate = 44100;
    let duration_samples = sample_rate * 2; // 2 seconds

    // Generate white noise at -20 dBFS
    let amplitude = 0.1;
    let samples: Vec<f32> = (0..duration_samples)
        .map(|i| {
            // Simple pseudo-random generator - fixed point to avoid overflow
            let seed = ((i as u64).wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff;
            let random_f32 = (seed as f64) / (i32::MAX as f64);
            (amplitude as f64 * (random_f32 - 0.5) * 2.0) as f32
        })
        .collect();

    let metrics = compute_ebu_r128_loudness(&samples, 1, sample_rate);

    // White noise should have relatively stable loudness
    assert!(metrics.integrated_lufs > -50.0 && metrics.integrated_lufs < 0.0);
    assert!(metrics.loudness_range_lu >= 0.0); // Should have some range
    assert!(metrics.true_peak_dbtp <= 0.0);
    assert!(metrics.sample_peak_dbfs <= 0.0);
}

#[test]
fn test_ebu_r128_different_sample_rates() {
    let duration_ms = 1000; // 1 second
    let amplitude = 0.5;
    let frequency = 440.0;

    for sample_rate in [22050, 44100, 48000, 96000] {
        let duration_samples = (sample_rate * duration_ms / 1000) as usize;
        let samples: Vec<f32> = (0..duration_samples)
            .map(|i| {
                let phase = 2.0 * std::f32::consts::PI * frequency * i as f32 / sample_rate as f32;
                amplitude * phase.sin()
            })
            .collect();

        let metrics = compute_ebu_r128_loudness(&samples, 1, sample_rate);

        // Should give consistent results across sample rates
        assert!(metrics.integrated_lufs > -50.0 && metrics.integrated_lufs < 0.0);
        assert!(metrics.loudness_range_lu >= 0.0);
        assert!(metrics.true_peak_dbtp > -15.0 && metrics.true_peak_dbtp <= 0.0);
    }
}

#[test]
fn test_ebu_r128_channel_count() {
    let sample_rate = 44100;
    let duration_samples = sample_rate; // 1 second
    let amplitude = 0.3;

    for channels in [1, 2, 4, 6] {
        let samples: Vec<f32> = (0..duration_samples)
            .flat_map(|i| {
                let phase = 2.0 * std::f32::consts::PI * 440.0 * i as f32 / sample_rate as f32;
                let sample = amplitude * phase.sin();
                vec![sample; channels]
            })
            .collect();

        let metrics = compute_ebu_r128_loudness(&samples, channels as u8, sample_rate);

        // All channel counts should work
        assert!(metrics.integrated_lufs > -35.0 && metrics.integrated_lufs < -5.0);
        assert!(metrics.loudness_range_lu >= 0.0);
        assert!(metrics.true_peak_dbtp > -15.0 && metrics.true_peak_dbtp < 0.0);
    }
}

#[test]
fn test_ebu_r128_different_amplitudes() {
    let sample_rate = 44100;
    let duration_samples = sample_rate; // 1 second
    let frequency = 440.0;

    for amplitude_dbfs in [-30.0, -20.0, -12.0, -6.0, -3.0, 0.0] {
        let amplitude = 10.0f32.powf(amplitude_dbfs / 20.0);
        let samples: Vec<f32> = (0..duration_samples)
            .map(|i| {
                let phase = 2.0 * std::f32::consts::PI * frequency * i as f32 / sample_rate as f32;
                amplitude * phase.sin()
            })
            .collect();

        let metrics = compute_ebu_r128_loudness(&samples, 1, sample_rate);

        // Integrated loudness should roughly correlate with amplitude
        let expected_lufs = amplitude_dbfs as f64 - 3.0; // Rough approximation for sine wave
        assert!(
            (metrics.integrated_lufs - expected_lufs).abs() < 5.0,
            "For amplitude {} dBFS, expected ~{} LUFS, got {} LUFS",
            amplitude_dbfs,
            expected_lufs,
            metrics.integrated_lufs
        );

        // True peak should be close to the input amplitude
        assert!(
            (metrics.true_peak_dbtp - amplitude_dbfs as f64).abs() < 1.0,
            "True peak should be close to input amplitude"
        );
    }
}

#[test]
fn test_ebu_r128_dynamic_content() {
    let sample_rate = 44100;
    let total_samples = sample_rate * 5; // 5 seconds

    // Create content with varying amplitude to test loudness range
    let samples: Vec<f32> = (0..total_samples)
        .map(|i| {
            let second = i as f32 / sample_rate as f32;
            let amplitude = match second {
                s if s < 1.0 => 0.1, // Quiet
                s if s < 2.0 => 0.3, // Medium
                s if s < 3.0 => 0.7, // Loud
                s if s < 4.0 => 0.2, // Medium-quiet
                _ => 0.5,            // Medium-loud
            };
            let phase = 2.0 * std::f32::consts::PI * 440.0 * i as f32 / sample_rate as f32;
            amplitude * phase.sin()
        })
        .collect();

    let metrics = compute_ebu_r128_loudness(&samples, 1, sample_rate);

    // Should have reasonable loudness and measurable range
    assert!(metrics.integrated_lufs > -25.0 && metrics.integrated_lufs < -10.0);
    assert!(metrics.loudness_range_lu > 2.0); // Should have significant range
    assert!(metrics.true_peak_dbtp < -3.0); // Peak from the loud section
    assert!(metrics.sample_peak_dbfs < -3.0);
}

#[test]
fn test_ebu_r128_consistency() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9, 0.4, -0.6];

    let metrics1 = compute_ebu_r128_loudness(&samples, 1, 44100);
    let metrics2 = compute_ebu_r128_loudness(&samples, 1, 44100);

    // Results should be deterministic
    assert_eq!(metrics1.integrated_lufs, metrics2.integrated_lufs);
    assert_eq!(metrics1.loudness_range_lu, metrics2.loudness_range_lu);
    assert_eq!(metrics1.true_peak_dbtp, metrics2.true_peak_dbtp);
    assert_eq!(metrics1.sample_peak_dbfs, metrics2.sample_peak_dbfs);
}

#[test]
fn test_ebu_r128_short_duration() {
    // Test with very short audio (less than one 400ms block)
    let samples = vec![0.5, -0.3, 0.8, -0.2]; // Very short
    let metrics = compute_ebu_r128_loudness(&samples, 1, 44100);

    // Should handle gracefully
    assert!(metrics.integrated_lufs > -150.0 && metrics.integrated_lufs < 0.0);
    assert!(metrics.loudness_range_lu >= 0.0);
    assert!(metrics.true_peak_dbtp > -150.0 && metrics.true_peak_dbtp <= 0.0);
    assert!(metrics.sample_peak_dbfs > -150.0 && metrics.sample_peak_dbfs <= 0.0);
}

#[test]
fn test_ebu_r128_peak_accuracy() {
    let sample_rate = 44100;
    let samples = vec![0.0, 1.0, -1.0, 0.0]; // Include max positive and negative peaks

    let metrics = compute_ebu_r128_loudness(&samples, 1, sample_rate);

    // Should detect the exact peak
    assert!((metrics.true_peak_dbtp - 0.0).abs() < 0.1);
    assert!((metrics.sample_peak_dbfs - 0.0).abs() < 0.1);
}

#[test]
fn test_ebu_r128_gating_threshold() {
    let sample_rate = 44100;

    // Create audio with very low amplitude (below gating threshold)
    let amplitude = 10.0f32.powf(-80.0 / 20.0); // -80 dBFS
    let samples: Vec<f32> = (0..sample_rate)
        .map(|i| {
            let phase = 2.0 * std::f32::consts::PI * 440.0 * i as f32 / sample_rate as f32;
            amplitude * phase.sin()
        })
        .collect();

    let metrics = compute_ebu_r128_loudness(&samples, 1, sample_rate);

    // Most blocks should be gated out, resulting in default loudness
    assert!(metrics.integrated_lufs <= -23.0); // Should be near or at default
    assert_eq!(metrics.loudness_range_lu, 0.0); // No range when most is gated
}
