use libflo_audio::core::ebu_r128::compute_ebu_r128_loudness;

#[test]
fn test_ebu_r128_empty_samples() {
    let samples: Vec<f32> = vec![];
    let metrics = compute_ebu_r128_loudness(&samples, 1, 44100);

    assert_eq!(metrics.integrated_lufs, -23.0);
    assert_eq!(metrics.loudness_range_lu, 0.0);
    assert_eq!(metrics.true_peak_dbtp, -150.0);
    assert_eq!(metrics.sample_peak_dbfs, -150.0);
}

#[test]
fn test_ebu_r128_silence() {
    let samples = vec![0.0; 44100];
    let metrics = compute_ebu_r128_loudness(&samples, 1, 44100);

    assert_eq!(metrics.integrated_lufs, -23.0);
    assert_eq!(metrics.loudness_range_lu, 0.0);
    assert_eq!(metrics.true_peak_dbtp, -150.0);
    assert_eq!(metrics.sample_peak_dbfs, -150.0);
}

#[test]
fn test_ebu_r128_mono_sine_wave() {
    let sr = 44100;
    let freq = 440.0;
    let amp = 0.5;
    let samples: Vec<f32> = (0..sr)
        .map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin())
        .collect();

    let m = compute_ebu_r128_loudness(&samples, 1, sr);

    assert!(m.integrated_lufs < -5.0 && m.integrated_lufs > -25.0);
    assert!(m.loudness_range_lu >= 0.0);
    assert!(m.true_peak_dbtp < -5.0 && m.true_peak_dbtp > -7.0);
    assert!(m.sample_peak_dbfs < -5.0 && m.sample_peak_dbfs > -7.0);
}

#[test]
fn test_ebu_r128_stereo_sine_wave() {
    let sr = 44100;
    let freq = 440.0;
    let amp = 0.5;

    let samples: Vec<f32> = (0..sr * 2)
        .flat_map(|i| {
            let s = amp * (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin();
            vec![s, s]
        })
        .collect();

    let m = compute_ebu_r128_loudness(&samples, 2, sr);

    assert!(m.integrated_lufs < -5.0 && m.integrated_lufs > -25.0);
    assert!(m.true_peak_dbtp < -5.0 && m.true_peak_dbtp > -7.0);
}

#[test]
fn test_ebu_r128_white_noise() {
    let sr = 44100;
    let amp = 0.1;

    let samples: Vec<f32> = (0..sr * 2)
        .map(|i| {
            let seed = ((i as u64).wrapping_mul(1103515245).wrapping_add(12345)) & 0x7fffffff;
            let r = (seed as f64) / (i32::MAX as f64);
            (amp as f64 * (r - 0.5) * 2.0) as f32
        })
        .collect();

    let m = compute_ebu_r128_loudness(&samples, 1, sr);

    assert!(m.integrated_lufs < -10.0 && m.integrated_lufs > -40.0);
    assert!(m.loudness_range_lu >= 0.0);
    assert!(m.true_peak_dbtp <= 0.0);
    assert!(m.sample_peak_dbfs <= 0.0);
}

#[test]
fn test_ebu_r128_different_sample_rates() {
    let freq = 440.0;
    let amp = 0.5;

    for sr in [22050, 44100, 48000, 96000] {
        let samples: Vec<f32> = (0..sr)
            .map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin())
            .collect();

        let m = compute_ebu_r128_loudness(&samples, 1, sr);

        assert!(m.integrated_lufs < -5.0 && m.integrated_lufs > -25.0);
        assert!(m.true_peak_dbtp < 0.0 && m.true_peak_dbtp > -15.0);
    }
}

#[test]
fn test_ebu_r128_channel_count() {
    let sr = 44100;
    let amp = 0.3;

    for ch in [1, 2, 4, 6] {
        let samples: Vec<f32> = (0..sr)
            .flat_map(|i| {
                let s = amp * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin();
                vec![s; ch]
            })
            .collect();

        let m = compute_ebu_r128_loudness(&samples, ch as u8, sr);

        assert!(m.integrated_lufs < -5.0 && m.integrated_lufs > -35.0);
        assert!(m.true_peak_dbtp < 0.0 && m.true_peak_dbtp > -15.0);
    }
}

#[test]
fn test_ebu_r128_different_amplitudes() {
    let sr = 44100;
    let freq = 440.0;

    for amp_db in [-30.0, -20.0, -12.0, -6.0, -3.0, 0.0] {
        let amp = 10.0f32.powf(amp_db / 20.0);

        let samples: Vec<f32> = (0..sr)
            .map(|i| amp * (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin())
            .collect();

        let m = compute_ebu_r128_loudness(&samples, 1, sr);

        let expected_peak = amp_db as f64;
        assert!((m.true_peak_dbtp - expected_peak).abs() < 1.0);
        assert!(m.integrated_lufs < expected_peak + 10.0);
        assert!(m.integrated_lufs > expected_peak - 30.0);
    }
}

#[test]
fn test_ebu_r128_dynamic_content() {
    let sr = 44100;
    let total = sr * 5;

    let samples: Vec<f32> = (0..total)
        .map(|i| {
            let t = i as f32 / sr as f32;
            let amp = match t {
                x if x < 1.0 => 0.1,
                x if x < 2.0 => 0.3,
                x if x < 3.0 => 0.7,
                x if x < 4.0 => 0.2,
                _ => 0.5,
            };
            amp * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin()
        })
        .collect();

    let m = compute_ebu_r128_loudness(&samples, 1, sr);

    assert!(m.integrated_lufs < -10.0 && m.integrated_lufs > -25.0);
    assert!(m.loudness_range_lu > 2.0);
    assert!(m.true_peak_dbtp < -3.0);
}

#[test]
fn test_ebu_r128_consistency() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9, 0.4, -0.6];

    let m1 = compute_ebu_r128_loudness(&samples, 1, 44100);
    let m2 = compute_ebu_r128_loudness(&samples, 1, 44100);

    assert_eq!(m1.integrated_lufs, m2.integrated_lufs);
    assert_eq!(m1.loudness_range_lu, m2.loudness_range_lu);
    assert_eq!(m1.true_peak_dbtp, m2.true_peak_dbtp);
    assert_eq!(m1.sample_peak_dbfs, m2.sample_peak_dbfs);
}

#[test]
fn test_ebu_r128_short_duration() {
    let samples = vec![0.5, -0.3, 0.8, -0.2];
    let m = compute_ebu_r128_loudness(&samples, 1, 44100);

    assert!(m.integrated_lufs < 0.0 && m.integrated_lufs > -150.0);
    assert!(m.true_peak_dbtp <= 0.0);
    assert!(m.sample_peak_dbfs <= 0.0);
}

#[test]
fn test_ebu_r128_peak_accuracy() {
    let samples = vec![0.0, 1.0, -1.0, 0.0];
    let m = compute_ebu_r128_loudness(&samples, 1, 44100);

    assert!((m.true_peak_dbtp - 0.0).abs() < 0.1);
    assert!((m.sample_peak_dbfs - 0.0).abs() < 0.1);
}

#[test]
fn test_ebu_r128_gating_threshold() {
    let sr = 44100;
    let amp = 10.0f32.powf(-80.0 / 20.0);

    let samples: Vec<f32> = (0..sr)
        .map(|i| amp * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
        .collect();

    let m = compute_ebu_r128_loudness(&samples, 1, sr);

    assert!(m.integrated_lufs <= -23.0);
    assert_eq!(m.loudness_range_lu, 0.0);
}
