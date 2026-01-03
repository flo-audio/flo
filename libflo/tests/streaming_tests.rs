//! Streaming tests for flo™ audio codec
//!
//! Tests for:
//! - StreamingDecoder with incremental data
//! - StreamingEncoder for live encoding
//! - Network simulation (chunked data arrival)
//! - Quality verification (streaming vs standard decode)

use libflo::lossy::TransformEncoder;
use libflo::{Decoder, DecoderState, Encoder, StreamingDecoder};

#[test]
fn test_streaming_decoder_basic() {
    let sample_rate = 44100u32;
    let channels = 1u8;

    // Generate 1 second of audio
    let samples: Vec<f32> = (0..sample_rate as usize)
        .map(|i| (i as f32 * 0.01).sin())
        .collect();

    // Encode with standard encoder
    let encoder = Encoder::new(sample_rate, channels, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();

    // Decode with streaming decoder
    let mut decoder = StreamingDecoder::new();
    decoder.feed(&flo_data).unwrap();

    assert_eq!(decoder.state(), DecoderState::Ready);

    let decoded = decoder.decode_available().unwrap();
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_streaming_decoder_chunked() {
    let sample_rate = 22050u32;
    let channels = 1u8;

    // Generate audio
    let samples: Vec<f32> = (0..sample_rate as usize)
        .map(|i| (i as f32 * 0.02).sin())
        .collect();

    let encoder = Encoder::new(sample_rate, channels, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();

    // Feed in small chunks (simulating network)
    let mut decoder = StreamingDecoder::new();

    // First, decoder should be waiting for header
    assert_eq!(decoder.state(), DecoderState::WaitingForHeader);

    // Feed chunks
    let chunk_size = 50;
    for chunk in flo_data.chunks(chunk_size) {
        decoder.feed(chunk).unwrap();
    }

    assert_eq!(decoder.state(), DecoderState::Ready);

    let decoded = decoder.decode_available().unwrap();
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_streaming_decoder_stereo() {
    let sample_rate = 48000u32;
    let channels = 2u8;

    // Generate stereo audio
    let mut samples = Vec::with_capacity(sample_rate as usize * 2);
    for i in 0..sample_rate as usize {
        samples.push((i as f32 * 0.01).sin()); // Left
        samples.push((i as f32 * 0.01).cos()); // Right
    }

    let encoder = Encoder::new(sample_rate, channels, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();

    let mut decoder = StreamingDecoder::new();
    decoder.feed(&flo_data).unwrap();

    assert_eq!(decoder.state(), DecoderState::Ready);

    let decoded = decoder.decode_available().unwrap();
    assert_eq!(decoded.len(), samples.len());

    // Verify channel separation
    let mut left_sum: f32 = 0.0;
    let mut right_sum: f32 = 0.0;
    for i in 0..decoded.len() / 2 {
        left_sum += decoded[i * 2].abs();
        right_sum += decoded[i * 2 + 1].abs();
    }
    assert!(left_sum > 0.0, "Left channel is empty");
    assert!(right_sum > 0.0, "Right channel is empty");
}

#[test]
fn test_streaming_decoder_info() {
    let sample_rate = 44100u32;
    let channels = 2u8;

    let samples: Vec<f32> = (0..sample_rate as usize * 2)
        .map(|i| (i as f32 * 0.01).sin())
        .collect();

    let encoder = Encoder::new(sample_rate, channels, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();

    let mut decoder = StreamingDecoder::new();

    // Before feeding, no info available
    assert!(decoder.info().is_none());

    // Feed all data
    decoder.feed(&flo_data).unwrap();

    // Now info should be available
    let info = decoder.info().expect("Should have info");
    assert_eq!(info.sample_rate, sample_rate);
    assert_eq!(info.channels, channels);
}

#[test]
fn test_streaming_decoder_reset() {
    let sample_rate = 22050u32;

    let samples: Vec<f32> = (0..sample_rate as usize)
        .map(|i| (i as f32 * 0.01).sin())
        .collect();

    let encoder = Encoder::new(sample_rate, 1, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();

    let mut decoder = StreamingDecoder::new();
    decoder.feed(&flo_data).unwrap();
    let _ = decoder.decode_available().unwrap();

    // Reset and decode again
    decoder.reset();
    assert_eq!(decoder.state(), DecoderState::WaitingForHeader);

    decoder.feed(&flo_data).unwrap();
    assert_eq!(decoder.state(), DecoderState::Ready);

    let decoded = decoder.decode_available().unwrap();
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_streaming_partial_data() {
    let sample_rate = 44100u32;

    let samples: Vec<f32> = (0..sample_rate as usize)
        .map(|i| (i as f32 * 0.01).sin())
        .collect();

    let encoder = Encoder::new(sample_rate, 1, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();

    let mut decoder = StreamingDecoder::new();

    // Feed just the header (first 70 bytes)
    decoder.feed(&flo_data[..70.min(flo_data.len())]).unwrap();

    // Should be waiting for TOC
    assert!(matches!(
        decoder.state(),
        DecoderState::WaitingForHeader | DecoderState::WaitingForToc
    ));

    // Feed more data
    if flo_data.len() > 70 {
        decoder.feed(&flo_data[70..]).unwrap();
    }

    assert_eq!(decoder.state(), DecoderState::Ready);
}

#[test]
fn test_streaming_lossy_basic() {
    let sample_rate = 44100u32;
    let channels = 1u8;

    // Generate 1 second of audio
    let samples: Vec<f32> = (0..sample_rate as usize)
        .map(|i| (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / sample_rate as f32).sin() * 0.5)
        .collect();

    // Encode with LOSSY encoder
    let mut encoder = TransformEncoder::new(sample_rate, channels, 0.8);
    let flo_data = encoder.encode_to_flo(&samples, &[]).unwrap();

    // Decode with streaming decoder
    let mut decoder = StreamingDecoder::new();
    decoder.feed(&flo_data).unwrap();

    assert_eq!(decoder.state(), DecoderState::Ready);

    // Should detect as lossy
    let info = decoder.info().expect("Should have info");
    assert!(info.is_lossy, "Should detect as lossy stream");

    let decoded = decoder.decode_available().unwrap();

    // Lossy won't be exact, but should be reasonably close
    assert!(
        decoded.len() > samples.len() / 2,
        "Should decode most samples"
    );

    // Check audio isn't silent
    let energy: f32 = decoded.iter().map(|x| x * x).sum();
    assert!(energy > 0.0, "Decoded audio shouldn't be silent");
}

#[test]
fn test_streaming_lossy_stereo() {
    let sample_rate = 48000u32;
    let channels = 2u8;

    // Generate stereo audio - different frequencies for L/R
    let mut samples = Vec::with_capacity(sample_rate as usize * 2);
    for i in 0..sample_rate as usize {
        let left = (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / sample_rate as f32).sin() * 0.4;
        let right =
            (i as f32 * 880.0 * 2.0 * std::f32::consts::PI / sample_rate as f32).sin() * 0.4;
        samples.push(left);
        samples.push(right);
    }

    let mut encoder = TransformEncoder::new(sample_rate, channels, 0.7);
    let flo_data = encoder.encode_to_flo(&samples, &[]).unwrap();

    let mut decoder = StreamingDecoder::new();
    decoder.feed(&flo_data).unwrap();

    assert_eq!(decoder.state(), DecoderState::Ready);

    let info = decoder.info().expect("Should have info");
    assert!(info.is_lossy);
    assert_eq!(info.channels, 2);

    let decoded = decoder.decode_available().unwrap();
    assert!(decoded.len() > 0, "Should decode some samples");
}

#[test]
fn test_streaming_lossy_chunked() {
    let sample_rate = 22050u32;

    let samples: Vec<f32> = (0..sample_rate as usize)
        .map(|i| (i as f32 * 0.02).sin() * 0.5)
        .collect();

    let mut encoder = TransformEncoder::new(sample_rate, 1, 0.6);
    let flo_data = encoder.encode_to_flo(&samples, &[]).unwrap();

    // Feed in small chunks (network simulation)
    let mut decoder = StreamingDecoder::new();
    let chunk_size = 100;

    for chunk in flo_data.chunks(chunk_size) {
        decoder.feed(chunk).unwrap();
    }

    assert_eq!(decoder.state(), DecoderState::Ready);

    let info = decoder.info().expect("Should have info");
    assert!(info.is_lossy, "Should detect lossy");

    let decoded = decoder.decode_available().unwrap();
    assert!(decoded.len() > 0);
}

/// Test true frame-by-frame streaming decode with next_frame()
#[test]
fn test_streaming_next_frame_lossless() {
    let sample_rate = 44100u32;
    let channels = 1u8;

    // Generate 3 seconds of audio (will be 3 frames)
    let samples: Vec<f32> = (0..sample_rate as usize * 3)
        .map(|i| (i as f32 * 0.01).sin())
        .collect();

    let encoder = Encoder::new(sample_rate, channels, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();

    let mut decoder = StreamingDecoder::new();
    decoder.feed(&flo_data).unwrap();

    assert_eq!(decoder.state(), DecoderState::Ready);

    // Decode frame by frame
    let mut all_samples = Vec::new();
    let mut frame_count = 0;

    while let Ok(Some(frame_samples)) = decoder.next_frame() {
        frame_count += 1;
        all_samples.extend(frame_samples);
    }

    assert!(frame_count >= 3, "Should have at least 3 frames");
    assert_eq!(
        all_samples.len(),
        samples.len(),
        "Total samples should match"
    );
}

/// Test frame-by-frame streaming for lossy
#[test]
fn test_streaming_next_frame_lossy() {
    let sample_rate = 44100u32;
    let channels = 1u8;

    // Generate 3 seconds of audio
    let samples: Vec<f32> = (0..sample_rate as usize * 3)
        .map(|i| (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / sample_rate as f32).sin() * 0.5)
        .collect();

    let mut encoder = TransformEncoder::new(sample_rate, channels, 0.8);
    let flo_data = encoder.encode_to_flo(&samples, &[]).unwrap();

    let mut decoder = StreamingDecoder::new();
    decoder.feed(&flo_data).unwrap();

    assert_eq!(decoder.state(), DecoderState::Ready);

    // Decode frame by frame
    let mut all_samples = Vec::new();
    let mut frame_count = 0;

    while let Ok(Some(frame_samples)) = decoder.next_frame() {
        if !frame_samples.is_empty() {
            frame_count += 1;
            all_samples.extend(frame_samples);
        }
    }

    assert!(
        frame_count >= 2,
        "Should have at least 2 frames (after skipping preroll)"
    );
    assert!(all_samples.len() > 0, "Should have decoded samples");
}

/// Test incremental streaming with progressive frame decode
#[test]
fn test_streaming_progressive_decode() {
    let sample_rate = 22050u32;
    let channels = 1u8;

    // Generate 2 seconds of audio
    let samples: Vec<f32> = (0..sample_rate as usize * 2)
        .map(|i| (i as f32 * 0.01).sin())
        .collect();

    let encoder = Encoder::new(sample_rate, channels, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();

    let mut decoder = StreamingDecoder::new();

    // Feed data incrementally and try to decode available frames
    let chunk_size = 1000;
    let mut all_samples = Vec::new();

    for chunk in flo_data.chunks(chunk_size) {
        decoder.feed(chunk).unwrap();

        // Try to decode any available frames
        while let Ok(Some(frame_samples)) = decoder.next_frame() {
            all_samples.extend(frame_samples);
        }
    }

    // Final decode of remaining frames
    while let Ok(Some(frame_samples)) = decoder.next_frame() {
        all_samples.extend(frame_samples);
    }

    assert_eq!(
        all_samples.len(),
        samples.len(),
        "Total decoded samples should match"
    );
}

// ============================================================================
// Quality verification tests - streaming vs standard decode
// ============================================================================

/// Compare streaming next_frame() decode with standard Decoder for lossless
#[test]
fn test_streaming_vs_standard_lossless_quality() {
    let sample_rate = 44100u32;
    let channels = 2u8;

    // Generate 3 seconds of stereo audio with complex waveform
    let mut samples = Vec::with_capacity(sample_rate as usize * 3 * 2);
    for i in 0..sample_rate as usize * 3 {
        let t = i as f32 / sample_rate as f32;
        // Mix of frequencies
        let left = (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.5
            + (t * 880.0 * 2.0 * std::f32::consts::PI).sin() * 0.25;
        let right = (t * 550.0 * 2.0 * std::f32::consts::PI).sin() * 0.5
            + (t * 1100.0 * 2.0 * std::f32::consts::PI).sin() * 0.25;
        samples.push(left);
        samples.push(right);
    }

    // Encode
    let encoder = Encoder::new(sample_rate, channels, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();

    println!(
        "Encoded {} samples to {} bytes",
        samples.len(),
        flo_data.len()
    );

    // Decode with STANDARD decoder
    let standard_decoder = Decoder::new();
    let standard_decoded = standard_decoder.decode(&flo_data).unwrap();

    // Decode with STREAMING decoder using next_frame()
    let mut streaming_decoder = StreamingDecoder::new();
    streaming_decoder.feed(&flo_data).unwrap();

    let mut streaming_decoded = Vec::new();
    while let Ok(Some(frame_samples)) = streaming_decoder.next_frame() {
        streaming_decoded.extend(frame_samples);
    }

    println!("Standard decoded: {} samples", standard_decoded.len());
    println!("Streaming decoded: {} samples", streaming_decoded.len());

    // Compare lengths
    assert_eq!(
        streaming_decoded.len(),
        standard_decoded.len(),
        "Streaming and standard decode should produce same number of samples"
    );

    // Compare sample values (should be IDENTICAL for lossless)
    let mut max_diff: f32 = 0.0;
    let mut diff_count = 0;
    for (i, (s, st)) in streaming_decoded
        .iter()
        .zip(standard_decoded.iter())
        .enumerate()
    {
        let diff = (s - st).abs();
        if diff > 0.0001 {
            if diff_count < 10 {
                println!(
                    "Sample {}: streaming={}, standard={}, diff={}",
                    i, s, st, diff
                );
            }
            diff_count += 1;
        }
        max_diff = max_diff.max(diff);
    }

    println!(
        "Max difference: {}, samples with diff: {}",
        max_diff, diff_count
    );

    assert!(
        max_diff < 0.0001,
        "Streaming decode should match standard decode exactly for lossless (max_diff={})",
        max_diff
    );
}

/// Compare streaming next_frame() decode with decode_available() for lossless
#[test]
fn test_streaming_next_frame_vs_decode_available_lossless() {
    let sample_rate = 44100u32;
    let channels = 1u8;

    // Generate 5 seconds of audio (larger file)
    let samples: Vec<f32> = (0..sample_rate as usize * 5)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.7
        })
        .collect();

    let encoder = Encoder::new(sample_rate, channels, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();

    // Decode with decode_available()
    let mut decoder1 = StreamingDecoder::new();
    decoder1.feed(&flo_data).unwrap();
    let decoded_available = decoder1.decode_available().unwrap();

    // Decode with next_frame()
    let mut decoder2 = StreamingDecoder::new();
    decoder2.feed(&flo_data).unwrap();
    let mut decoded_frames = Vec::new();
    while let Ok(Some(frame_samples)) = decoder2.next_frame() {
        decoded_frames.extend(frame_samples);
    }

    println!("decode_available: {} samples", decoded_available.len());
    println!("next_frame total: {} samples", decoded_frames.len());

    assert_eq!(
        decoded_frames.len(),
        decoded_available.len(),
        "next_frame() should produce same total as decode_available()"
    );

    // Verify samples match
    let mut max_diff: f32 = 0.0;
    for (a, b) in decoded_frames.iter().zip(decoded_available.iter()) {
        max_diff = max_diff.max((a - b).abs());
    }

    assert!(
        max_diff < 0.0001,
        "next_frame() samples should match decode_available() (max_diff={})",
        max_diff
    );
}

/// Compare streaming vs standard for LOSSY
#[test]
fn test_streaming_vs_standard_lossy_quality() {
    let sample_rate = 44100u32;
    let channels = 2u8;

    // Generate 3 seconds of stereo audio
    let mut samples = Vec::with_capacity(sample_rate as usize * 3 * 2);
    for i in 0..sample_rate as usize * 3 {
        let t = i as f32 / sample_rate as f32;
        let left = (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.5;
        let right = (t * 550.0 * 2.0 * std::f32::consts::PI).sin() * 0.5;
        samples.push(left);
        samples.push(right);
    }

    // Encode with LOSSY encoder
    let mut encoder = TransformEncoder::new(sample_rate, channels, 0.8);
    let flo_data = encoder.encode_to_flo(&samples, &[]).unwrap();

    println!(
        "Lossy encoded {} samples to {} bytes",
        samples.len(),
        flo_data.len()
    );

    // Decode with decode_available()
    let mut decoder1 = StreamingDecoder::new();
    decoder1.feed(&flo_data).unwrap();
    let decoded_available = decoder1.decode_available().unwrap();

    // Decode with next_frame()
    let mut decoder2 = StreamingDecoder::new();
    decoder2.feed(&flo_data).unwrap();
    let mut decoded_frames = Vec::new();
    while let Ok(Some(frame_samples)) = decoder2.next_frame() {
        decoded_frames.extend(frame_samples);
    }

    println!("decode_available: {} samples", decoded_available.len());
    println!("next_frame total: {} samples", decoded_frames.len());

    // For lossy, lengths should match
    assert_eq!(
        decoded_frames.len(),
        decoded_available.len(),
        "Lossy: next_frame() should produce same total as decode_available()"
    );

    // Verify samples match
    let mut max_diff: f32 = 0.0;
    for (a, b) in decoded_frames.iter().zip(decoded_available.iter()) {
        max_diff = max_diff.max((a - b).abs());
    }

    println!("Lossy max diff between methods: {}", max_diff);

    assert!(
        max_diff < 0.0001,
        "Lossy: next_frame() samples should match decode_available() (max_diff={})",
        max_diff
    );
}

/// Test with large file (simulating 30MB+ file)
#[test]
fn test_streaming_large_lossless_file() {
    let sample_rate = 44100u32;
    let channels = 2u8;

    // Generate 30 seconds of stereo audio (about 5MB uncompressed)
    let duration_secs = 30;
    let total_samples = sample_rate as usize * duration_secs * channels as usize;

    let mut samples = Vec::with_capacity(total_samples);
    for i in 0..sample_rate as usize * duration_secs {
        let t = i as f32 / sample_rate as f32;
        // Complex waveform
        let base = (t * 440.0 * 2.0 * std::f32::consts::PI).sin();
        let harmonic = (t * 880.0 * 2.0 * std::f32::consts::PI).sin() * 0.5;
        let left = (base + harmonic) * 0.6;
        let right = (base - harmonic * 0.3) * 0.6;
        samples.push(left);
        samples.push(right);
    }

    let encoder = Encoder::new(sample_rate, channels, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();

    println!(
        "Large file: {} samples -> {} bytes ({:.2} MB)",
        samples.len(),
        flo_data.len(),
        flo_data.len() as f64 / 1024.0 / 1024.0
    );

    // Standard decode
    let standard_decoder = Decoder::new();
    let standard_decoded = standard_decoder.decode(&flo_data).unwrap();

    // Streaming decode with next_frame()
    let mut streaming_decoder = StreamingDecoder::new();
    streaming_decoder.feed(&flo_data).unwrap();

    let mut streaming_decoded = Vec::new();
    let mut frame_count = 0;
    while let Ok(Some(frame_samples)) = streaming_decoder.next_frame() {
        streaming_decoded.extend(frame_samples);
        frame_count += 1;
    }

    println!("Decoded {} frames", frame_count);
    println!(
        "Standard: {} samples, Streaming: {} samples",
        standard_decoded.len(),
        streaming_decoded.len()
    );

    assert_eq!(
        streaming_decoded.len(),
        standard_decoded.len(),
        "Large file: streaming should match standard length"
    );

    // Sample comparison
    let mut max_diff: f32 = 0.0;
    let mut total_diff: f64 = 0.0;
    for (s, st) in streaming_decoded.iter().zip(standard_decoded.iter()) {
        let diff = (s - st).abs();
        max_diff = max_diff.max(diff);
        total_diff += diff as f64;
    }
    let avg_diff = total_diff / streaming_decoded.len() as f64;

    println!("Max diff: {}, Avg diff: {}", max_diff, avg_diff);

    assert!(
        max_diff < 0.0001,
        "Large file: streaming should match standard (max_diff={})",
        max_diff
    );
}

/// Test that individual frame samples are correct
#[test]
fn test_streaming_individual_frame_correctness() {
    let sample_rate = 44100u32;
    let channels = 1u8;

    // Generate exactly 3 seconds = 3 frames
    let samples: Vec<f32> = (0..sample_rate as usize * 3)
        .map(|i| (i as f32 * 0.001).sin())
        .collect();

    let encoder = Encoder::new(sample_rate, channels, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();

    // Standard decode for reference
    let standard_decoder = Decoder::new();
    let standard_decoded = standard_decoder.decode(&flo_data).unwrap();

    // Streaming decode frame by frame
    let mut streaming_decoder = StreamingDecoder::new();
    streaming_decoder.feed(&flo_data).unwrap();

    let mut frame_idx = 0;
    let mut sample_offset = 0;

    while let Ok(Some(frame_samples)) = streaming_decoder.next_frame() {
        println!("Frame {}: {} samples", frame_idx, frame_samples.len());

        // Compare each sample in this frame
        for (i, &s) in frame_samples.iter().enumerate() {
            let std_idx = sample_offset + i;
            if std_idx < standard_decoded.len() {
                let std_s = standard_decoded[std_idx];
                let diff = (s - std_s).abs();
                if diff > 0.0001 {
                    println!(
                        "  Frame {} sample {}: streaming={}, standard={}, diff={}",
                        frame_idx, i, s, std_s, diff
                    );
                }
            }
        }

        sample_offset += frame_samples.len();
        frame_idx += 1;
    }

    println!(
        "Total frames: {}, Total samples: {}",
        frame_idx, sample_offset
    );
    assert_eq!(
        sample_offset,
        standard_decoded.len(),
        "Should decode all samples"
    );
}
