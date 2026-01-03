//! Internal streaming tests

use super::*;
use crate::Encoder;

#[test]
fn test_streaming_encode_decode_roundtrip() {
    let sample_rate = 44100u32;
    let channels = 1u8;

    // Generate test audio
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
fn test_streaming_incremental_feed() {
    let sample_rate = 22050u32;
    let channels = 1u8;

    let samples: Vec<f32> = (0..sample_rate as usize)
        .map(|i| (i as f32 * 0.02).sin())
        .collect();

    let encoder = Encoder::new(sample_rate, channels, 16);
    let flo_data = encoder.encode(&samples, &[]).unwrap();

    let mut decoder = StreamingDecoder::new();

    // Feed in small chunks
    let chunk_size = 100;
    for chunk in flo_data.chunks(chunk_size) {
        decoder.feed(chunk).unwrap();
    }

    assert_eq!(decoder.state(), DecoderState::Ready);

    let decoded = decoder.decode_available().unwrap();
    assert_eq!(decoded.len(), samples.len());
}

#[test]
fn test_streaming_encoder_frame_output() {
    let sample_rate = 8000u32;
    let channels = 1u8;

    // Generate 2.5 seconds of audio
    let total_samples = (sample_rate as usize) * 5 / 2;
    let samples: Vec<f32> = (0..total_samples)
        .map(|i| (i as f32 * 0.01).sin())
        .collect();

    let mut encoder = StreamingEncoder::new(sample_rate, channels, 16);
    encoder.push_samples(&samples).unwrap();

    // Should have 2 complete frames (2 seconds)
    assert_eq!(encoder.pending_frames(), 2);

    // Get frames
    let _frame1 = encoder.next_frame().unwrap();
    let _frame2 = encoder.next_frame().unwrap();
    assert!(encoder.next_frame().is_none());

    // Finalize to get the remaining 0.5 seconds
    let flo_data = encoder.finalize(&[]).unwrap();
    assert!(!flo_data.is_empty());
}
