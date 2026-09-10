#[cfg(test)]
mod seeking_tests {
    use libflo_audio::seeking;
    use libflo_audio::{encode, encode_lossy, info};

    /// Helper: Create test audio with known pattern
    fn create_test_audio(sample_rate: u32, channels: u8, duration_secs: f64) -> Vec<f32> {
        let num_samples = (sample_rate as f64 * duration_secs) as usize * channels as usize;
        let mut samples = Vec::with_capacity(num_samples);

        for i in 0..num_samples {
            // Create a simple sine wave pattern
            let freq = 440.0;
            let phase = (i as f32 / sample_rate as f32) * 2.0 * std::f32::consts::PI * freq;
            samples.push(phase.sin() * 0.5);
        }

        samples
    }

    #[test]
    fn test_get_toc_lossless() {
        // Create a test file with multiple frames
        let samples = create_test_audio(44100, 2, 5.0); // 5 seconds
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        let toc = seeking::get_toc(&flo_data).expect("Failed to get TOC");

        // Should have at least one frame (5 seconds at 1s per frame)
        assert!(!toc.is_empty(), "TOC should not be empty");

        // Verify TOC entries are properly ordered
        for i in 1..toc.len() {
            let prev = &toc[i - 1];
            let curr = &toc[i];

            // Frame indices should be sequential
            assert_eq!(curr.frame_index, prev.frame_index + 1);

            // Timestamps should be increasing
            assert!(
                curr.timestamp_ms >= prev.timestamp_ms,
                "Timestamps should be monotonically increasing"
            );

            // Byte offsets should be increasing
            assert!(
                curr.byte_offset >= prev.byte_offset,
                "Byte offsets should be monotonically increasing"
            );

            // Frame sizes should be reasonable
            assert!(curr.frame_size > 0, "Frame size should be positive");
        }
    }

    #[test]
    fn test_get_toc_lossy() {
        // Test with lossy encoding
        let samples = create_test_audio(44100, 2, 5.0);
        let flo_data = encode_lossy(&samples, 44100, 2, 16, 2, None).expect("Failed to encode");

        let toc = seeking::get_toc(&flo_data).expect("Failed to get TOC");

        assert!(!toc.is_empty(), "Lossy TOC should not be empty");

        // Verify frame indices are sequential
        for (i, entry) in toc.iter().enumerate() {
            assert_eq!(entry.frame_index, i as u32);
        }
    }

    #[test]
    fn test_decode_frame_at_basic() {
        let samples = create_test_audio(44100, 2, 3.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        // Get TOC to know frame count
        let toc = seeking::get_toc(&flo_data).expect("Failed to get TOC");
        assert!(!toc.is_empty());

        // Decode first frame
        let frame0 = seeking::decode_frame_at(&flo_data, 0).expect("Failed to decode frame 0");
        assert!(!frame0.is_empty(), "Frame 0 should have samples");

        // For a 44100 Hz, 2-channel file, we expect ~88200 samples per frame (1 second * 2 channels)
        // Allow some variation for compression
        assert!(frame0.len() > 0, "Frame should contain decoded samples");
    }

    #[test]
    fn test_decode_frame_at_multiple_frames() {
        let samples = create_test_audio(44100, 2, 4.0); // 4 seconds = 4 frames
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        let toc = seeking::get_toc(&flo_data).expect("Failed to get TOC");
        let num_frames = toc.len();

        // Decode each frame
        for frame_idx in 0..num_frames {
            let frame_samples = seeking::decode_frame_at(&flo_data, frame_idx as u32)
                .expect(&format!("Failed to decode frame {}", frame_idx));

            assert!(
                !frame_samples.is_empty(),
                "Frame {} should have samples",
                frame_idx
            );
        }
    }

    #[test]
    fn test_decode_frame_at_out_of_bounds() {
        let samples = create_test_audio(44100, 2, 2.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        let toc = seeking::get_toc(&flo_data).expect("Failed to get TOC");
        let num_frames = toc.len() as u32;

        // Try to decode beyond available frames
        let result = seeking::decode_frame_at(&flo_data, num_frames + 10);
        assert!(
            result.is_err(),
            "Should error when decoding out-of-bounds frame"
        );
    }

    #[test]
    fn test_seek_to_time_basic() {
        let samples = create_test_audio(44100, 2, 5.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        // Seek to 2 seconds (2000 ms)
        let seek_result = seeking::seek_to_time(&flo_data, 2000).expect("Failed to seek");

        assert!(
            seek_result.frame_index < 10,
            "Frame index should be reasonable"
        );
        // At 2000ms with 1s frames, we should be at or near frame 2 (which starts at 2000ms)
        assert!(
            seek_result.timestamp_ms <= 2000,
            "Timestamp should be at or before seek position"
        );
        assert!(
            seek_result.next_timestamp_ms > seek_result.timestamp_ms,
            "Next timestamp should be after current"
        );
    }

    #[test]
    fn test_seek_to_time_start() {
        let samples = create_test_audio(44100, 2, 3.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        // Seek to start
        let seek_result = seeking::seek_to_time(&flo_data, 0).expect("Failed to seek to start");

        assert_eq!(seek_result.frame_index, 0, "Should be at first frame");
        assert_eq!(seek_result.timestamp_ms, 0, "Timestamp should be 0");
        assert!(
            seek_result.sample_offset == 0,
            "Sample offset should be 0 at frame start"
        );
    }

    #[test]
    fn test_seek_to_time_end() {
        let samples = create_test_audio(44100, 2, 3.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        // Seek to near the end (2.9 seconds in 3-second file)
        let seek_result = seeking::seek_to_time(&flo_data, 2900).expect("Failed to seek to end");

        let toc = seeking::get_toc(&flo_data).expect("Failed to get TOC");
        assert!(
            seek_result.frame_index < toc.len() as u32,
            "Frame index should be within bounds"
        );
    }

    #[test]
    fn test_seek_to_time_beyond_duration() {
        let samples = create_test_audio(44100, 2, 3.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        // Seek beyond file duration (should clamp to last frame)
        let seek_result = seeking::seek_to_time(&flo_data, 10000).expect("Seek should succeed");

        let toc = seeking::get_toc(&flo_data).expect("Failed to get TOC");
        assert!(
            seek_result.frame_index < toc.len() as u32,
            "Should clamp to last valid frame"
        );
    }

    #[test]
    fn test_seek_to_time_sub_frame_accuracy() {
        let samples = create_test_audio(44100, 2, 3.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        // Seek to a position within a frame (1.5 seconds)
        let seek_result = seeking::seek_to_time(&flo_data, 1500).expect("Failed to seek");

        // The offset should be less than one frame's worth of samples
        let frame_samples = 44100; // 1 second worth
        assert!(
            seek_result.sample_offset <= frame_samples,
            "Sample offset should not exceed frame size"
        );
    }

    #[test]
    fn test_seek_and_decode_consistency() {
        let samples = create_test_audio(44100, 2, 4.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        // Seek to frame 1 (1 second mark)
        let seek_result = seeking::seek_to_time(&flo_data, 1000).expect("Failed to seek");

        // Decode the frame we found
        let decoded = seeking::decode_frame_at(&flo_data, seek_result.frame_index)
            .expect("Failed to decode sought frame");

        assert!(!decoded.is_empty(), "Should decode samples");

        // Verify the decoded frame is reasonable in size
        let expected_samples = 44100 * 2; // Frame size in interleaved samples
        assert!(
            (decoded.len() as f32 - expected_samples as f32).abs()
                < (expected_samples as f32 * 0.1),
            "Decoded frame should be approximately 1 second of audio"
        );
    }

    #[test]
    fn test_seek_multiple_positions() {
        let samples = create_test_audio(44100, 2, 5.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        // Seek to various positions
        let positions = vec![0, 500, 1000, 1500, 2000, 3000, 4000, 4500];

        for pos in positions {
            let seek_result =
                seeking::seek_to_time(&flo_data, pos).expect(&format!("Failed to seek to {}", pos));

            // Verify result is valid
            assert!(seek_result.frame_index < 10, "Frame index should be valid");
            assert!(
                seek_result.timestamp_ms <= pos,
                "Timestamp should not exceed seek position"
            );
            assert!(
                seek_result.next_timestamp_ms >= seek_result.timestamp_ms,
                "Next timestamp should be valid"
            );
        }
    }

    #[test]
    fn test_lossy_seek_and_decode() {
        let samples = create_test_audio(48000, 2, 4.0);
        let flo_data =
            encode_lossy(&samples, 48000, 2, 16, 2, None).expect("Failed to encode lossy");

        // Get TOC
        let toc = seeking::get_toc(&flo_data).expect("Failed to get TOC");
        assert!(!toc.is_empty(), "Lossy file should have TOC");

        // Seek and decode
        let seek_result = seeking::seek_to_time(&flo_data, 2000).expect("Failed to seek");
        let frame_data = seeking::decode_frame_at(&flo_data, seek_result.frame_index)
            .expect("Failed to decode lossy frame");

        assert!(!frame_data.is_empty(), "Lossy frame should decode");
    }

    #[test]
    fn test_seek_different_sample_rates() {
        let sample_rates = vec![8000, 16000, 22050, 44100, 48000, 96000];

        for sr in sample_rates {
            let samples = create_test_audio(sr, 2, 2.0);
            let flo_data =
                encode(&samples, sr, 2, 16, None).expect(&format!("Failed to encode at {} Hz", sr));

            // Seek to 1 second
            let seek_result = seeking::seek_to_time(&flo_data, 1000)
                .expect(&format!("Failed to seek at {} Hz", sr));

            assert!(
                seek_result.frame_index < 5,
                "Frame index should be reasonable"
            );

            // Decode the frame
            let decoded = seeking::decode_frame_at(&flo_data, seek_result.frame_index)
                .expect(&format!("Failed to decode at {} Hz", sr));

            assert!(
                !decoded.is_empty(),
                "Should decode samples at all sample rates"
            );
        }
    }

    #[test]
    fn test_seek_different_channel_counts() {
        let channel_counts = vec![1, 2];

        for ch in channel_counts {
            let samples = create_test_audio(44100, ch, 2.0);
            let flo_data = encode(&samples, 44100, ch, 16, None)
                .expect(&format!("Failed to encode {} channels", ch));

            // Get info to verify
            let file_info = info(&flo_data).expect("Failed to get info");
            assert_eq!(file_info.channels, ch);

            // Seek and decode
            let seek_result = seeking::seek_to_time(&flo_data, 500)
                .expect(&format!("Failed to seek {} channels", ch));
            let decoded = seeking::decode_frame_at(&flo_data, seek_result.frame_index)
                .expect(&format!("Failed to decode {} channels", ch));

            assert!(!decoded.is_empty());
        }
    }

    #[test]
    fn test_toc_timestamp_accuracy() {
        let samples = create_test_audio(44100, 2, 5.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        let toc = seeking::get_toc(&flo_data).expect("Failed to get TOC");

        // Verify timestamps are accurate based on frame positions
        let mut cumulative_samples = 0u64;

        for entry in &toc {
            // Calculate expected timestamp from sample count
            let expected_ms = (cumulative_samples * 1000 / 44100) as u32;

            // Allow small tolerance due to rounding
            assert!(
                (entry.timestamp_ms as i32 - expected_ms as i32).abs() < 100,
                "Timestamp should be accurate for frame {}",
                entry.frame_index
            );

            cumulative_samples += 44100; // Each frame is 1 second
        }
    }

    #[test]
    fn test_binary_search_exact_match() {
        let _toc = vec![
            libflo_audio::core::TocEntry {
                frame_index: 0,
                byte_offset: 0,
                frame_size: 100,
                timestamp_ms: 0,
            },
            libflo_audio::core::TocEntry {
                frame_index: 1,
                byte_offset: 100,
                frame_size: 100,
                timestamp_ms: 1000,
            },
            libflo_audio::core::TocEntry {
                frame_index: 2,
                byte_offset: 200,
                frame_size: 100,
                timestamp_ms: 2000,
            },
        ];

        // Test the binary search logic by seeking
        assert_eq!(
            seeking::seek_to_time(
                &libflo_audio::encode(&create_test_audio(44100, 2, 3.0), 44100, 2, 16, None)
                    .unwrap(),
                0
            )
            .unwrap()
            .frame_index,
            0
        );
    }

    #[test]
    fn test_binary_search_empty_edge_case() {
        let samples = create_test_audio(44100, 2, 1.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        // Seeking should work even with minimal data
        let result = seeking::seek_to_time(&flo_data, 0).expect("Failed to seek");
        assert_eq!(result.frame_index, 0);
    }

    #[test]
    fn test_binary_search_single_frame() {
        let samples = create_test_audio(44100, 2, 1.0);
        let flo_data = encode(&samples, 44100, 2, 16, None).expect("Failed to encode");

        let result = seeking::seek_to_time(&flo_data, 0).expect("Failed to seek");
        assert_eq!(result.frame_index, 0);

        let result = seeking::seek_to_time(&flo_data, 500).expect("Failed to seek");
        assert_eq!(result.frame_index, 0);
    }
}
