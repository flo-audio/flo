use libflo_audio::{
    encode, info, Reader,
};

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

    // Total frames should be actual sample count (not frame count)
    assert_eq!(
        file_info.total_frames,
        expected_samples as u64,
        "Total frames should match sample count"
    );

    // Duration should be calculated from total_frames / sample_rate
    let expected_duration = expected_samples as f64 / sample_rate as f64;
    assert!(
        (file_info.duration_secs - expected_duration).abs() < 0.001,
        "Duration should be calculated from total_frames / sample_rate: expected {}, got {}",
        expected_duration,
        file_info.duration_secs
    );
}