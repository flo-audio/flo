use libflo_audio::{encode, info};

#[test]
fn test_duration_accuracy_regression() {
    let sample_rate = 44100u32;
    let channels = 1u8;

    // Test exactly 2 seconds of audio
    let duration_secs = 2.0;
    let expected_samples = (sample_rate as f32 * duration_secs) as usize;
    let samples: Vec<f32> = (0..expected_samples)
        .map(|i| ((i as f32) * 0.01).sin() * 0.5)
        .collect();

    let flo_data = encode(&samples, sample_rate, channels, 16, None).expect("Encoding failed");
    let file_info = info(&flo_data).expect("Info failed");

    // Total frames represents duration in seconds (number of 1-second frames)
    assert_eq!(
        file_info.total_frames, duration_secs as u64,
        "Total frames should equal duration in seconds"
    );

    // Duration is total_frames directly (not total_frames / sample_rate)
    let expected_duration = duration_secs as f64;
    assert!(
        (file_info.duration_secs - expected_duration).abs() < 0.001,
        "Duration should be total_frames (seconds): expected {}, got {}",
        expected_duration,
        file_info.duration_secs
    );
}
