mod encoder_tests {
    use libflo_audio::lossy::encoder::serialize_sparse;
    use libflo_audio::{decode, encode_lossy, info, LossyEncoder, QualityPreset};

    #[test]
    fn test_sparse_encoding() {
        // Test with mostly zeros
        let coeffs = vec![0i16, 0, 0, 100, 0, 0, 0, 0, -50, 25, 0, 0];
        let encoded = serialize_sparse(&coeffs);

        // Should be much smaller than 24 bytes (12 * 2)
        assert!(encoded.len() < 20, "Sparse encoding should compress");
    }

    // ============================================================================
    // Lossy Mode Tests (Transform-based)
    // ============================================================================

    #[test]
    fn test_lossy_encoding_mono() {
        let sample_rate = 44100u32;
        let channels = 1u8;

        // Generate 1 second of audio
        let samples: Vec<f32> = (0..sample_rate as usize)
            .map(|i| ((i as f32) * 0.01).sin() * 0.5)
            .collect();

        // Encode lossy with medium quality (1 = Medium)
        let flo_data = encode_lossy(&samples, sample_rate, channels, 16, 1, None)
            .expect("Lossy encoding failed");

        // Decode
        let decoded = decode(&flo_data).expect("Decoding failed");

        // Lossy encoding may produce slightly different lengths due to MDCT blocking
        // Just verify we got audio back
        assert!(!decoded.is_empty(), "Decoded output should not be empty");

        // Check file info shows lossy mode
        let file_info = info(&flo_data).expect("Info failed");
        assert!(file_info.is_lossy, "File should be marked as lossy");
    }

    #[test]
    fn test_lossy_encoding_stereo() {
        let sample_rate = 44100u32;
        let channels = 2u8;

        // Generate 1 second of stereo audio
        let mut samples = Vec::with_capacity((sample_rate as usize) * 2);
        for i in 0..sample_rate as usize {
            samples.push(((i as f32) * 0.01).sin() * 0.5); // Left
            samples.push(((i as f32) * 0.015).sin() * 0.5); // Right
        }

        // Encode lossy with high quality (2 = High)
        let flo_data = encode_lossy(&samples, sample_rate, channels, 16, 2, None)
            .expect("Lossy encoding failed");

        // Decode
        let decoded = decode(&flo_data).expect("Decoding failed");

        // Verify we got audio back
        assert!(!decoded.is_empty());

        // Check file info
        let file_info = info(&flo_data).expect("Info failed");
        assert!(file_info.is_lossy);
    }

    #[test]
    fn test_lossy_all_quality_levels() {
        let sample_rate = 44100u32;
        let channels = 1u8;

        // Generate test audio
        let samples: Vec<f32> = (0..sample_rate as usize)
            .map(|i| ((i as f32) * 0.01).sin() * 0.5)
            .collect();

        // Test each quality level (0-4)
        for quality in 0..=4 {
            let flo_data = encode_lossy(&samples, sample_rate, channels, 16, quality, None)
                .unwrap_or_else(|_| panic!("Lossy encoding failed for quality {}", quality));

            let decoded = decode(&flo_data).expect("Decoding failed");
            assert!(
                !decoded.is_empty(),
                "Quality {} should produce output",
                quality
            );

            let file_info = info(&flo_data).expect("Info failed");
            assert!(file_info.is_lossy);
        }
    }

    #[test]
    fn test_encode_transform_api() {
        let sample_rate = 44100u32;
        let channels = 1u8;

        let samples: Vec<f32> = (0..sample_rate as usize)
            .map(|i| ((i as f32) * 0.01).sin() * 0.5)
            .collect();

        // Test LossyEncoder with continuous quality (builder pattern)
        let mut encoder = LossyEncoder::new(sample_rate, channels, 0.5);
        let flo_data = encoder
            .encode_to_flo(&samples, &[])
            .expect("Transform encoding failed");

        let file_info = info(&flo_data).expect("Info failed");
        assert!(file_info.is_lossy);

        let decoded = decode(&flo_data).expect("Decoding failed");
        assert!(!decoded.is_empty());
    }

    #[test]
    fn test_lossy_quality_enum() {
        // Verify quality preset values
        assert_eq!(QualityPreset::Low.as_f32(), 0.0);
        assert_eq!(QualityPreset::Transparent.as_f32(), 1.0);

        // Medium should be between low and transparent
        let medium = QualityPreset::Medium.as_f32();
        assert!(medium > 0.0 && medium < 1.0);
    }

    // ============================================================================
    // Audio Quality Tests
    // ============================================================================

    #[test]
    fn test_lossy_audio_quality() {
        let sample_rate = 44100u32;
        let channels = 1u8;

        // Generate a clean sine wave
        let freq = 440.0;
        let samples: Vec<f32> = (0..sample_rate as usize)
            .map(|i| {
                ((2.0 * std::f32::consts::PI * freq * (i as f32)) / (sample_rate as f32)).sin()
                    * 0.8
            })
            .collect();

        // Encode at highest quality (4 = Transparent)
        let flo_data =
            encode_lossy(&samples, sample_rate, channels, 16, 4, None).expect("Encoding failed");

        let decoded = decode(&flo_data).expect("Decoding failed");

        // Just verify we got reasonable output
        assert!(!decoded.is_empty());

        // Check output is in valid range
        let max_val = decoded.iter().map(|&s| s.abs()).fold(0.0f32, f32::max);
        assert!(max_val <= 1.0, "Output should be normalized");
    }

    #[test]
    fn test_lossy_preserves_silence() {
        let sample_rate = 44100u32;
        let channels = 1u8;

        // Generate silence
        let samples = vec![0.0f32; sample_rate as usize];

        let flo_data =
            encode_lossy(&samples, sample_rate, channels, 16, 2, None).expect("Encoding failed");

        let decoded = decode(&flo_data).expect("Decoding failed");

        // Decoded silence should have very low energy
        let energy: f32 = decoded.iter().map(|&s| s * s).sum();
        let rms = (energy / (decoded.len() as f32)).sqrt();

        assert!(
            rms < 0.01,
            "Silence should decode to near-silence, got RMS {}",
            rms
        );
    }

    // ============================================================================
    // Metadata Tests
    // ============================================================================

    #[test]
    fn test_lossy_with_metadata() {
        let sample_rate = 44100u32;
        let channels = 1u8;

        let samples: Vec<f32> = (0..sample_rate as usize)
            .map(|i| ((i as f32) * 0.01).sin() * 0.5)
            .collect();

        // Create metadata
        let metadata = libflo_audio::create_metadata(
            Some("Test Song".to_string()),
            Some("Test Artist".to_string()),
            Some("Test Album".to_string()),
        )
        .expect("Failed to create metadata");

        // Encode with metadata
        let flo_data = encode_lossy(
            &samples,
            sample_rate,
            channels,
            16,
            2,
            Some(metadata.clone()),
        )
        .expect("Encoding failed");

        // Decode and check metadata preserved
        let decoded = decode(&flo_data).expect("Decoding failed");
        assert!(!decoded.is_empty());

        // Read metadata back
        let reader = libflo_audio::Reader::new();
        let file = reader.read(&flo_data).expect("Failed to read file");
        let meta = libflo_audio::FloMetadata::from_msgpack(&file.metadata)
            .expect("Failed to parse metadata");

        assert_eq!(meta.title.as_deref(), Some("Test Song"));
        assert_eq!(meta.artist.as_deref(), Some("Test Artist"));
        assert_eq!(meta.album.as_deref(), Some("Test Album"));
    }
}
