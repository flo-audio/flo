use libflo_audio::core::analysis::{extract_waveform_peaks, extract_waveform_rms};

#[test]
fn test_extract_waveform_peaks_mono() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];
    let waveform = extract_waveform_peaks(&samples, 1, 44100, 10);

    assert_eq!(waveform.channels, 1);
    assert_eq!(waveform.peaks_per_second, 10);
    assert!(!waveform.peaks.is_empty());

    // All peaks should be between 0.0 and 1.0
    for &peak in &waveform.peaks {
        assert!(peak >= 0.0 && peak <= 1.0);
    }
}

#[test]
fn test_extract_waveform_peaks_stereo() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9]; // L,R,L,R,L,R
    let waveform = extract_waveform_peaks(&samples, 2, 44100, 10);

    assert_eq!(waveform.channels, 2);
    assert_eq!(waveform.peaks_per_second, 10);
    assert!(!waveform.peaks.is_empty());

    // All peaks should be between 0.0 and 1.0
    for &peak in &waveform.peaks {
        assert!(peak >= 0.0 && peak <= 1.0);
    }
}

#[test]
fn test_extract_waveform_peaks_empty() {
    let samples: Vec<f32> = vec![];
    let waveform = extract_waveform_peaks(&samples, 1, 44100, 10);

    assert_eq!(waveform.channels, 1);
    assert_eq!(waveform.peaks_per_second, 10);
    assert!(waveform.peaks.is_empty());
}

#[test]
fn test_extract_waveform_rms_mono() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];
    let waveform = extract_waveform_rms(&samples, 1, 44100, 10);

    assert_eq!(waveform.channels, 1);
    assert_eq!(waveform.peaks_per_second, 10);
    assert!(!waveform.peaks.is_empty());

    // All RMS values should be between 0.0 and 1.0
    for &peak in &waveform.peaks {
        assert!(peak >= 0.0 && peak <= 1.0);
    }
}

#[test]
fn test_extract_waveform_peaks_consistency() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];

    let waveform1 = extract_waveform_peaks(&samples, 1, 44100, 10);
    let waveform2 = extract_waveform_peaks(&samples, 1, 44100, 10);

    assert_eq!(waveform1.peaks, waveform2.peaks);
    assert_eq!(waveform1.channels, waveform2.channels);
    assert_eq!(waveform1.peaks_per_second, waveform2.peaks_per_second);
}

#[test]
fn test_extract_waveform_rms_consistency() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];

    let waveform1 = extract_waveform_rms(&samples, 1, 44100, 10);
    let waveform2 = extract_waveform_rms(&samples, 1, 44100, 10);

    assert_eq!(waveform1.peaks, waveform2.peaks);
    assert_eq!(waveform1.channels, waveform2.channels);
    assert_eq!(waveform1.peaks_per_second, waveform2.peaks_per_second);
}

#[test]
fn test_waveform_peaks_vs_rms() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];

    let waveform_peaks = extract_waveform_peaks(&samples, 1, 44100, 10);
    let waveform_rms = extract_waveform_rms(&samples, 1, 44100, 10);

    // Both should have same structure
    assert_eq!(waveform_peaks.channels, waveform_rms.channels);
    assert_eq!(
        waveform_peaks.peaks_per_second,
        waveform_rms.peaks_per_second
    );
    assert_eq!(waveform_peaks.peaks.len(), waveform_rms.peaks.len());

    // RMS should generally be less than or equal to peaks
    for (peak, rms) in waveform_peaks.peaks.iter().zip(waveform_rms.peaks.iter()) {
        assert!(*rms <= peak + 0.01); // Small tolerance for floating point
    }
}
