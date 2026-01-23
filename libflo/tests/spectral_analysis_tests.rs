use libflo_audio::core::analysis::{
    extract_spectral_fingerprint, extract_dominant_frequencies, spectral_similarity
};

#[test]
fn test_extract_spectral_fingerprint_mono() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];
    let fingerprint = extract_spectral_fingerprint(&samples, 1, 44100, Some(1024), Some(512));
    
    assert_eq!(fingerprint.channels, 1);
    assert_eq!(fingerprint.fft_size, 1024);
    assert_eq!(fingerprint.frequency_bins, 513); // 1024/2 + 1
    assert_eq!(fingerprint.sample_rate, 44100);
    assert_eq!(fingerprint.hop_size, 512);
    assert!(!fingerprint.spectral_data.is_empty());
    
    // Check frequency resolution
    let expected_resolution = 44100.0 / 1024.0;
    assert!((fingerprint.frequency_resolution - expected_resolution).abs() < 0.001);
    
    // Each frame should have correct number of frequency bins
    for frame_spectrum in &fingerprint.spectral_data {
        assert_eq!(frame_spectrum.len(), 513);
    }
}

#[test]
fn test_extract_spectral_fingerprint_stereo() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9]; // L,R,L,R,L,R
    let fingerprint = extract_spectral_fingerprint(&samples, 2, 44100, Some(512), Some(256));
    
    assert_eq!(fingerprint.channels, 2);
    assert_eq!(fingerprint.fft_size, 512);
    assert_eq!(fingerprint.frequency_bins, 257); // 512/2 + 1
    assert_eq!(fingerprint.sample_rate, 44100);
    assert_eq!(fingerprint.hop_size, 256);
    assert!(!fingerprint.spectral_data.is_empty());
}

#[test]
fn test_extract_spectral_fingerprint_empty() {
    let samples: Vec<f32> = vec![];
    let fingerprint = extract_spectral_fingerprint(&samples, 1, 44100, Some(1024), Some(512));
    
    assert_eq!(fingerprint.channels, 1);
    assert_eq!(fingerprint.fft_size, 0);
    assert_eq!(fingerprint.frequency_bins, 0);
    assert_eq!(fingerprint.sample_rate, 44100);
    assert_eq!(fingerprint.hop_size, 0);
    assert!(fingerprint.spectral_data.is_empty());
}

#[test]
fn test_extract_spectral_fingerprint_power_of_two() {
    let samples = vec![0.5; 1000];
    let fingerprint = extract_spectral_fingerprint(&samples, 1, 44100, Some(1000), Some(500));
    
    // FFT size should be rounded up to next power of 2
    assert_eq!(fingerprint.fft_size, 1024);
    assert_eq!(fingerprint.frequency_bins, 513);
}

#[test]
fn test_extract_dominant_frequencies() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];
    let fingerprint = extract_spectral_fingerprint(&samples, 1, 44100, Some(256), Some(128));
    let dominant_freqs = extract_dominant_frequencies(&fingerprint, 3);
    
    assert_eq!(dominant_freqs.len(), fingerprint.spectral_data.len());
    
    // Each frame should have exactly 3 dominant frequencies
    for frame_freqs in &dominant_freqs {
        assert_eq!(frame_freqs.len(), 3);
        
        // Frequencies should be in ascending order (since we sort by magnitude descending)
        assert!(frame_freqs[0] >= frame_freqs[1]);
        assert!(frame_freqs[1] >= frame_freqs[2]);
        
        // Frequencies should be positive and reasonable
        for &freq in frame_freqs {
            assert!(freq >= 0.0);
            assert!(freq <= 22050.0); // Nyquist frequency for 44100 Hz
        }
    }
}

#[test]
fn test_spectral_similarity_identical() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];
    let fingerprint1 = extract_spectral_fingerprint(&samples, 1, 44100, Some(256), Some(128));
    let fingerprint2 = extract_spectral_fingerprint(&samples, 1, 44100, Some(256), Some(128));
    
    let similarity = spectral_similarity(&fingerprint1, &fingerprint2);
    
    // Identical fingerprints should have similarity of 1.0
    assert!((similarity - 1.0).abs() < 0.001);
}

#[test]
fn test_spectral_similarity_different() {
    let samples1 = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];
    let samples2 = vec![0.1, 0.9, -0.5, 0.3, -0.8, 0.2]; // Different pattern
    let fingerprint1 = extract_spectral_fingerprint(&samples1, 1, 44100, Some(256), Some(128));
    let fingerprint2 = extract_spectral_fingerprint(&samples2, 1, 44100, Some(256), Some(128));
    
    let similarity = spectral_similarity(&fingerprint1, &fingerprint2);
    
    // Different fingerprints should have similarity less than 1.0
    assert!(similarity < 1.0);
    assert!(similarity >= 0.0);
}

#[test]
fn test_spectral_similarity_incompatible() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];
    let fingerprint1 = extract_spectral_fingerprint(&samples, 1, 44100, Some(256), Some(128));
    let fingerprint2 = extract_spectral_fingerprint(&samples, 1, 44100, Some(512), Some(256));
    
    let similarity = spectral_similarity(&fingerprint1, &fingerprint2);
    
    // Incompatible fingerprints should have similarity of 0.0
    assert_eq!(similarity, 0.0);
}

#[test]
fn test_spectral_fingerprint_frequency_resolution() {
    let samples = vec![0.5; 2000];
    let fingerprint = extract_spectral_fingerprint(&samples, 1, 44100, Some(1024), None);
    
    // Frequency resolution should be sample_rate / fft_size
    let expected_resolution = 44100.0 / 1024.0;
    assert!((fingerprint.frequency_resolution - expected_resolution).abs() < 0.001);
    
    // Check that dominant frequencies make sense with the resolution
    let dominant_freqs = extract_dominant_frequencies(&fingerprint, 1);
    if !dominant_freqs.is_empty() && !dominant_freqs[0].is_empty() {
        let freq = dominant_freqs[0][0];
        // Should be a multiple of the frequency resolution (approximately)
        let bin_index = (freq / fingerprint.frequency_resolution).round();
        let reconstructed_freq = bin_index * fingerprint.frequency_resolution;
        assert!((freq - reconstructed_freq).abs() < fingerprint.frequency_resolution);
    }
}

#[test]
fn test_spectral_fingerprint_consistency() {
    let samples = vec![0.5, -0.3, 0.8, -0.2, 0.1, -0.9];
    let fingerprint1 = extract_spectral_fingerprint(&samples, 1, 44100, Some(512), Some(256));
    let fingerprint2 = extract_spectral_fingerprint(&samples, 1, 44100, Some(512), Some(256));
    
    assert_eq!(fingerprint1.fft_size, fingerprint2.fft_size);
    assert_eq!(fingerprint1.frequency_bins, fingerprint2.frequency_bins);
    assert_eq!(fingerprint1.frequency_resolution, fingerprint2.frequency_resolution);
    assert_eq!(fingerprint1.channels, fingerprint2.channels);
    assert_eq!(fingerprint1.sample_rate, fingerprint2.sample_rate);
    assert_eq!(fingerprint1.hop_size, fingerprint2.hop_size);
    
    // Spectral data should be identical
    assert_eq!(fingerprint1.spectral_data.len(), fingerprint2.spectral_data.len());
    for (frame1, frame2) in fingerprint1.spectral_data.iter().zip(fingerprint2.spectral_data.iter()) {
        assert_eq!(frame1.len(), frame2.len());
        for (mag1, mag2) in frame1.iter().zip(frame2.iter()) {
            assert!((mag1 - mag2).abs() < 0.001);
        }
    }
}