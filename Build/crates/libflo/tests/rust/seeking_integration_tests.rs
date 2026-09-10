#[cfg(test)]
mod seeking_integration_tests {
    use libflo_audio::seeking;
    use libflo_audio::{decode, encode, encode_lossy, info};

    /// Helper: Create predictable test audio for verification
    fn create_sine_wave(sample_rate: u32, channels: u8, duration_secs: f64, freq: f32) -> Vec<f32> {
        let num_samples = (sample_rate as f64 * duration_secs) as usize * channels as usize;
        let mut samples = Vec::with_capacity(num_samples);

        for i in 0..num_samples {
            let sample_idx = i / channels as usize;
            let phase =
                (sample_idx as f32 / sample_rate as f32) * 2.0 * std::f32::consts::PI * freq;
            samples.push(phase.sin() * 0.7);
        }

        samples
    }

    /// Helper: Verify audio similarity (for lossy comparison)
    fn audio_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let mut sum_sq_diff = 0.0;
        let mut sum_sq_a = 0.0;

        for i in 0..a.len() {
            let diff = a[i] - b[i];
            sum_sq_diff += diff * diff;
            sum_sq_a += a[i] * a[i];
        }

        if sum_sq_a == 0.0 {
            return 1.0;
        }

        1.0 - (sum_sq_diff / sum_sq_a).sqrt()
    }

    #[test]
    fn test_lossless_encode_seek_decode() {
        // Create audio
        let original = create_sine_wave(44100, 2, 5.0, 440.0);

        // Encode lossless
        let flo_data = encode(&original, 44100, 2, 16, None).expect("Failed to encode lossless");

        // Get file info
        let file_info = info(&flo_data).expect("Failed to get info");
        assert!(!file_info.is_lossy, "Should be lossless");

        // Seek to middle (2.5 seconds)
        let seek_result = seeking::seek_to_time(&flo_data, 2500).expect("Failed to seek");

        // Decode the frame
        let frame_samples = seeking::decode_frame_at(&flo_data, seek_result.frame_index)
            .expect("Failed to decode frame");

        assert!(!frame_samples.is_empty(), "Frame should have samples");

        // Verify we're in the right place
        assert!(frame_samples.len() > 0, "Should have decoded valid samples");
    }

    #[test]
    fn test_lossy_encode_seek_decode() {
        // Create audio
        let original = create_sine_wave(48000, 2, 4.0, 1000.0);

        // Encode lossy with different quality levels
        for quality in 0..=4 {
            let flo_data = encode_lossy(&original, 48000, 2, 16, quality, None)
                .expect(&format!("Failed to encode lossy quality {}", quality));

            let file_info = info(&flo_data).expect("Failed to get info");
            assert!(file_info.is_lossy, "Should be lossy");
            assert_eq!(file_info.lossy_quality, quality, "Quality should match");

            // Seek to 2 seconds
            let seek_result = seeking::seek_to_time(&flo_data, 2000)
                .expect(&format!("Failed to seek quality {}", quality));

            // Decode frame
            let frame_samples = seeking::decode_frame_at(&flo_data, seek_result.frame_index)
                .expect(&format!("Failed to decode lossy frame quality {}", quality));

            assert!(!frame_samples.is_empty(), "Lossy frame should decode");
        }
    }

    #[test]
    fn test_seek_multiple_frames_lossless() {
        let samples = create_sine_wave(44100, 2, 10.0, 880.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        // Get TOC
        let toc = seeking::get_toc(&flo_data).expect("Failed to get TOC");
        assert!(!toc.is_empty(), "TOC should exist");

        // Seek to each frame via time and decode
        for entry in toc.iter().take(5) {
            let seek_result = seeking::seek_to_time(&flo_data, entry.timestamp_ms)
                .expect(&format!("Failed to seek to frame {}", entry.frame_index));

            assert_eq!(
                seek_result.frame_index, entry.frame_index,
                "Seek should find correct frame"
            );

            let decoded = seeking::decode_frame_at(&flo_data, seek_result.frame_index)
                .expect("Failed to decode");

            assert!(!decoded.is_empty());
        }
    }

    #[test]
    fn test_seek_decode_consistency_across_sample_rates() {
        let sample_rates = [8000, 16000, 22050, 44100, 48000];

        for sr in &sample_rates {
            let samples = create_sine_wave(*sr, 2, 3.0, 440.0);
            let flo_data = encode(&samples, *sr, 2, 16, None)
                .expect(&format!("Failed to encode at {} Hz", sr));

            // Seek to 1.5 seconds
            let seek_result = seeking::seek_to_time(&flo_data, 1500)
                .expect(&format!("Failed to seek at {} Hz", sr));

            let decoded = seeking::decode_frame_at(&flo_data, seek_result.frame_index)
                .expect(&format!("Failed to decode at {} Hz", sr));

            // Expected samples per frame = sample_rate
            let expected_frame_samples = *sr as usize * 2; // 2 channels, 1 second
            assert!(
                (decoded.len() as i32 - expected_frame_samples as i32).abs() < 1000,
                "Frame size should match sample rate for {} Hz",
                sr
            );
        }
    }

    #[test]
    fn test_seek_decode_consistency_across_channels() {
        let channels = [1u8, 2];

        for ch in &channels {
            let samples = create_sine_wave(44100, *ch, 3.0, 440.0);
            let flo_data = encode(&samples, 44100, *ch, 16, None)
                .expect(&format!("Failed to encode {} channels", ch));

            let seek_result = seeking::seek_to_time(&flo_data, 1500)
                .expect(&format!("Failed to seek {} channels", ch));

            let decoded = seeking::decode_frame_at(&flo_data, seek_result.frame_index)
                .expect(&format!("Failed to decode {} channels", ch));

            // Frame should have 1 second of audio, interleaved
            let expected_samples = 44100 * (*ch as usize);
            assert!(
                (decoded.len() as i32 - expected_samples as i32).abs() < 1000,
                "Frame size should match channels for {} channels",
                ch
            );
        }
    }

    #[test]
    fn test_toc_entry_consistency() {
        let samples = create_sine_wave(44100, 2, 8.0, 550.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        let toc = seeking::get_toc(&flo_data).expect("Failed to get TOC");

        // Verify TOC structure
        for (i, entry) in toc.iter().enumerate() {
            // Frame index should be sequential
            assert_eq!(
                entry.frame_index as usize, i,
                "Frame indices should be sequential"
            );

            // Timestamps should increase monotonically
            if i > 0 {
                assert!(
                    entry.timestamp_ms > toc[i - 1].timestamp_ms,
                    "Timestamps should be strictly increasing"
                );
            }

            // Byte offsets should increase (frames take space)
            if i > 0 {
                assert!(
                    entry.byte_offset > toc[i - 1].byte_offset,
                    "Byte offsets should be increasing"
                );
            }

            // Frame size should be positive
            assert!(
                entry.frame_size > 0,
                "Frame {} should have positive size",
                i
            );
        }
    }

    #[test]
    fn test_seek_time_boundary_cases() {
        let samples = create_sine_wave(44100, 2, 5.0, 440.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        // Boundary test cases
        let test_times = vec![
            0,     // Start
            1,     // Very small offset
            1000,  // Exact frame boundary
            1001,  // Just after boundary
            2999,  // Just before next boundary
            3000,  // Exact boundary
            4999,  // Near end
            5000,  // Exact end
            6000,  // Beyond duration
            10000, // Way beyond
        ];

        for time_ms in test_times {
            let seek_result = seeking::seek_to_time(&flo_data, time_ms)
                .expect(&format!("Failed to seek to {} ms", time_ms));

            // Result should be valid
            assert!(
                seek_result.frame_index < 100,
                "Frame index should be reasonable for {} ms",
                time_ms
            );

            // Timestamp should be <= seek position
            assert!(
                seek_result.timestamp_ms <= time_ms,
                "Timestamp should be at or before seek position for {} ms",
                time_ms
            );

            // Should be decodable
            let decoded = seeking::decode_frame_at(&flo_data, seek_result.frame_index)
                .expect(&format!("Failed to decode for {} ms", time_ms));

            assert!(!decoded.is_empty());
        }
    }

    #[test]
    fn test_full_roundtrip_encode_seek_decode() {
        let original = create_sine_wave(44100, 2, 6.0, 350.0);

        // Encode
        let flo_data = encode(&original, 44100, 2, 16, None).expect("Failed to encode");

        // Decode full file
        let full_decoded = decode(&flo_data).expect("Failed to decode full");

        // Seek and decode individual frames, reassemble
        let toc = seeking::get_toc(&flo_data).expect("Failed to get TOC");
        let mut reassembled: Vec<f32> = Vec::new();

        for entry in &toc {
            let frame_samples = seeking::decode_frame_at(&flo_data, entry.frame_index)
                .expect(&format!("Failed to decode frame {}", entry.frame_index));
            reassembled.extend(&frame_samples);
        }

        // Full decode and reassembled should match
        assert_eq!(
            full_decoded.len(),
            reassembled.len(),
            "Full and reassembled should have same length"
        );

        // Audio should be identical
        let similarity = audio_similarity(&full_decoded, &reassembled);
        assert!(
            similarity > 0.99,
            "Full and reassembled audio should be identical (similarity: {})",
            similarity
        );
    }

    #[test]
    fn test_lossy_roundtrip_with_seeking() {
        let original = create_sine_wave(48000, 2, 4.0, 800.0);

        // Encode lossy
        let flo_data =
            encode_lossy(&original, 48000, 2, 16, 2, None).expect("Failed to encode lossy");

        // Decode full
        let full_decoded = decode(&flo_data).expect("Failed to decode lossy full");

        // Seek and decode frames, skipping the first frame (which is pre-roll for lossy)
        let toc = seeking::get_toc(&flo_data).expect("Failed to get TOC");
        let mut reassembled: Vec<f32> = Vec::new();

        for (i, entry) in toc.iter().enumerate() {
            let frame_samples = seeking::decode_frame_at(&flo_data, entry.frame_index)
                .expect("Failed to decode lossy frame");

            // For lossy, skip the first frame (pre-roll for overlap-add), just like decode() does
            if i > 0 {
                reassembled.extend(&frame_samples);
            }
        }

        // For lossy, decoded output should be approximately similar
        // (within a few frame's worth of samples due to overlap-add boundaries)
        let frame_samples = 48000; // 1 second of audio
        let tolerance = frame_samples * 2; // Allow 2 frames of difference
        assert!(
            (full_decoded.len() as i32 - reassembled.len() as i32).abs() <= tolerance as i32,
            "Lossy frame lengths should be similar (full: {}, reassembled: {})",
            full_decoded.len(),
            reassembled.len()
        );
    }

    #[test]
    fn test_seek_preserves_audio_quality_lossless() {
        let original = create_sine_wave(44100, 2, 3.0, 440.0);

        // Encode and decode normally
        let encoded = encode(&original, 44100, 2, 16, None).expect("Failed to encode");
        let normal_decoded = decode(&encoded).expect("Failed to decode");

        // Encode and seek to frame 1, decode that frame
        let encoded2 = encode(&original, 44100, 2, 16, None).expect("Failed to encode");
        let frame_samples = seeking::decode_frame_at(&encoded2, 1).expect("Failed to decode frame");

        // Frame 1 decoded via seeking should match normal decode's frame 1
        let frame_start = 44100 * 2; // Second frame starts here
        let frame_end = frame_start + (44100 * 2); // One frame of stereo

        let normal_frame =
            normal_decoded[frame_start..frame_end.min(normal_decoded.len())].to_vec();

        let similarity = audio_similarity(&normal_frame, &frame_samples);
        assert!(
            similarity > 0.99,
            "Seeking should preserve quality (similarity: {})",
            similarity
        );
    }

    #[test]
    fn test_concurrent_seeking() {
        let samples = create_sine_wave(44100, 2, 5.0, 440.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        let flo_data_clone = flo_data.clone();

        // Simulate concurrent seeks
        let seek1 = seeking::seek_to_time(&flo_data, 1000);
        let seek2 = seeking::seek_to_time(&flo_data_clone, 3000);

        let sr1 = seek1.expect("First seek failed");
        let sr2 = seek2.expect("Second seek failed");

        // Both should succeed and be different frames
        assert_ne!(sr1.frame_index, sr2.frame_index);

        // Both should decode
        let frame1 = seeking::decode_frame_at(&flo_data, sr1.frame_index)
            .expect("Failed to decode first frame");
        let frame2 = seeking::decode_frame_at(&flo_data_clone, sr2.frame_index)
            .expect("Failed to decode second frame");

        assert!(!frame1.is_empty());
        assert!(!frame2.is_empty());
    }
}
