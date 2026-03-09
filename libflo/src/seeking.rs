/// TOC-based seeking and frame access functionality
/// gives access to frames without fully decoding the entire file.
use crate::core::{FloFile, FloResult, TocEntry};
use crate::reader::Reader;

/// Represents the result of a seek operation
#[derive(Debug, Clone)]
pub struct SeekResult {
    /// Frame index where seeking occurred
    pub frame_index: u32,
    /// Byte offset in the file's data chunk
    pub byte_offset: u64,
    /// Timestamp of this frame in milliseconds
    pub timestamp_ms: u32,
    /// Number of samples to skip at the start of the frame (for sub-frame seeking)
    pub sample_offset: u32,
    /// The next frame's timestamp for duration calculation
    pub next_timestamp_ms: u32,
}

/// Extract TOC entries from a flo file
///
/// # Arguments
/// * `flo_data` - Complete flo file bytes
///
/// # Returns
/// Vector of TOC entries with frame indices, byte offsets, and timestamps
pub fn get_toc(flo_data: &[u8]) -> FloResult<Vec<TocEntry>> {
    let reader = Reader::new();
    let file = reader.read(flo_data)?;
    Ok(file.toc)
}

/// Decode a single frame at a specific index without decoding the entire file
///
/// # Arguments
/// * `flo_data` - Complete flo file bytes
/// * `frame_index` - Zero-based frame index
///
/// # Returns
/// Raw interleaved audio samples for that frame (f32, -1.0 to 1.0)
///
/// # Note
/// This function decodes only the requested frame
pub fn decode_frame_at(flo_data: &[u8], frame_index: u32) -> FloResult<Vec<f32>> {
    let reader = Reader::new();
    let file = reader.read(flo_data)?;

    if frame_index as usize >= file.frames.len() {
        return Err(format!(
            "Frame index {} out of bounds (total frames: {})",
            frame_index,
            file.frames.len()
        ));
    }

    let frame = &file.frames[frame_index as usize];

    // Detect if this is a lossy (transform) frame
    let is_transform = frame.frame_type == (crate::FrameType::Transform as u8);

    if is_transform {
        // Decode lossy frame
        decode_frame_lossy(&file, frame_index as usize)
    } else {
        // Decode lossless frame
        decode_frame_lossless(&file, frame_index as usize)
    }
}

/// Seek to a specific time in milliseconds and get frame information
///
/// # Arguments
/// * `flo_data` - Complete flo file bytes
/// * `time_ms` - Target time in milliseconds
///
/// # Returns
/// SeekResult with frame information and sample offset for sub-frame seeking
pub fn seek_to_time(flo_data: &[u8], target_ms: u32) -> FloResult<SeekResult> {
    let reader = Reader::new();
    let file = reader.read(flo_data)?;

    if file.toc.is_empty() {
        return Err("No TOC available for seeking".to_string());
    }

    // Binary search for the frame containing target_ms
    let mut frame_index = binary_search_frame(&file.toc, target_ms);

    // Clamp to valid range
    if frame_index as usize >= file.frames.len() {
        frame_index = (file.frames.len() - 1) as u32;
    }

    let toc_entry = &file.toc[frame_index as usize];

    // Calculate sample offset within this frame for sub-frame accuracy
    let frame_duration_ms = if frame_index + 1 < file.toc.len() as u32 {
        file.toc[(frame_index + 1) as usize].timestamp_ms - toc_entry.timestamp_ms
    } else {
        // Last frame: estimate from duration
        let last_frame_samples = file.frames[frame_index as usize].frame_samples;
        ((last_frame_samples as u64 * 1000) / file.header.sample_rate as u64) as u32
    };

    // How far into this frame should we start?
    let ms_into_frame = target_ms.saturating_sub(toc_entry.timestamp_ms);

    // Convert to sample offset
    let sample_offset = ((ms_into_frame as u64 * file.header.sample_rate as u64) / 1000) as u32;

    // Clamp to actual frame size
    let frame = &file.frames[frame_index as usize];
    let sample_offset = sample_offset.min(frame.frame_samples);

    let next_timestamp_ms = if frame_index + 1 < file.toc.len() as u32 {
        file.toc[(frame_index + 1) as usize].timestamp_ms
    } else {
        // Estimate next frame time based on last frame samples
        toc_entry.timestamp_ms + frame_duration_ms
    };

    Ok(SeekResult {
        frame_index,
        byte_offset: toc_entry.byte_offset,
        timestamp_ms: toc_entry.timestamp_ms,
        sample_offset,
        next_timestamp_ms,
    })
}

/// Find the best frame for a given timestamp using binary search
/// Returns the frame index that should be played for the given time
fn binary_search_frame(toc: &[TocEntry], target_ms: u32) -> u32 {
    if toc.is_empty() {
        return 0;
    }

    let mut left = 0;
    let mut right = toc.len() - 1;

    // Binary search for the rightmost frame where timestamp_ms <= target_ms
    while left < right {
        let mid = left + (right - left + 1) / 2;

        if toc[mid].timestamp_ms <= target_ms {
            left = mid;
        } else {
            right = mid - 1;
        }
    }

    left as u32
}

/// Internal: Decode a lossless frame
fn decode_frame_lossless(file: &FloFile, frame_index: usize) -> FloResult<Vec<f32>> {
    let frame = &file.frames[frame_index];
    let decoder = crate::Decoder::new();

    // Create a temporary FloFile with just this frame
    let frames = vec![frame.clone()];
    let temp_file = FloFile {
        header: file.header.clone(),
        toc: file.toc.clone(),
        frames: frames,
        extra: vec![],
        metadata: file.metadata.clone(),
    };

    decoder.decode_file(&temp_file)
}

/// Internal: Decode a lossy frame
fn decode_frame_lossy(file: &FloFile, frame_index: usize) -> FloResult<Vec<f32>> {
    let frame = &file.frames[frame_index];

    if frame.channels.is_empty() {
        return Err("Transform frame has no channel data".to_string());
    }

    // Transform data is in first channel's residuals
    let frame_data = &frame.channels[0].residuals;

    if let Some(transform_frame) = crate::lossy::deserialize_frame(frame_data) {
        let mut decoder =
            crate::lossy::TransformDecoder::new(file.header.sample_rate, file.header.channels);

        // For lossy frames, we need to maintain decoder state across frames
        // Skip frames before the target to maintain state
        for i in 0..frame_index {
            let f = &file.frames[i];
            if f.channels.is_empty() {
                continue;
            }
            let ch_data = &f.channels[0].residuals;
            if let Some(tf) = crate::lossy::deserialize_frame(ch_data) {
                let _ = decoder.decode_frame(&tf);
            }
        }

        // Now decode the target frame
        Ok(decoder.decode_frame(&transform_frame))
    } else {
        Err("Failed to deserialize transform frame".to_string())
    }
}
