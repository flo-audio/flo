use libflo_audio::{extract_waveform_peaks, Encoder, Reader, StreamingDecoder};

fn header_with_chunks(toc_size: u64, data_size: u64, extra_size: u64, meta_size: u64) -> Vec<u8> {
    let mut bytes = Vec::from(libflo_audio::MAGIC);
    bytes.extend_from_slice(&[1, 2]);
    bytes.extend_from_slice(&(0u16).to_le_bytes());
    bytes.extend_from_slice(&(44100u32).to_le_bytes());
    bytes.extend_from_slice(&(1u8).to_le_bytes());
    bytes.extend_from_slice(&(16u8).to_le_bytes());
    bytes.extend_from_slice(&(0u64).to_le_bytes());
    bytes.push(5);
    bytes.extend_from_slice(&[0, 0, 0]);
    bytes.extend_from_slice(&(0u32).to_le_bytes());
    bytes.extend_from_slice(&(66u64).to_le_bytes());
    bytes.extend_from_slice(&toc_size.to_le_bytes());
    bytes.extend_from_slice(&data_size.to_le_bytes());
    bytes.extend_from_slice(&extra_size.to_le_bytes());
    bytes.extend_from_slice(&meta_size.to_le_bytes());
    bytes
}

#[test]
fn reader_rejects_truncated_extra_chunk() {
    let mut data = header_with_chunks(4, 0, 1, 0);
    data.extend_from_slice(&(0u32).to_le_bytes());

    assert!(Reader::new().read(&data).is_err());
}

#[test]
fn streaming_decoder_rejects_unbounded_toc() {
    let mut data = header_with_chunks(4, 0, 0, 0);
    data.extend_from_slice(&u32::MAX.to_le_bytes());

    let mut decoder = StreamingDecoder::new();
    assert!(decoder.feed(&data).is_err());
}

#[test]
fn encoder_rejects_invalid_audio_shapes() {
    assert!(Encoder::new(0, 1, 16).encode(&[], &[]).is_err());
    assert!(Encoder::new(44100, 0, 16).encode(&[], &[]).is_err());
    assert!(Encoder::new(44100, 2, 16).encode(&[0.0], &[]).is_err());
}

#[test]
fn waveform_analysis_handles_invalid_parameters() {
    let waveform = extract_waveform_peaks(&[1.0], 0, 44100, 50);
    assert!(waveform.peaks.is_empty());

    let waveform = extract_waveform_peaks(&[1.0], 1, 0, 50);
    assert!(waveform.peaks.is_empty());

    let waveform = extract_waveform_peaks(&[1.0], 1, 44100, 0);
    assert!(waveform.peaks.is_empty());
}
