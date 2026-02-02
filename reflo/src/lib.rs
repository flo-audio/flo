//! reflo - Audio format converter library
//!
//! This library provides cross-platform audio conversion to and from flo™ format.
//! It works on native targets and can be compiled to WebAssembly.
//!

pub mod audio;

#[cfg(all(target_arch = "wasm32", feature = "wasm"))]
pub mod wasm;

use anyhow::{Context, Result};

/// Re-export libflo types
pub use libflo_audio::FloMetadata;

/// Information about a flo™ file
#[derive(Debug, Clone, serde::Serialize)]
pub struct FloInfo {
    pub version: String,
    pub sample_rate: u32,
    pub channels: u8,
    pub bit_depth: u8,
    pub duration_secs: f64,
    pub total_frames: u64,
    pub file_size: usize,
    pub compression_ratio: f64,
    pub crc_valid: bool,
    pub is_lossy: bool,
    pub lossy_quality: u8,
}

/// Get information about a flo™ file
pub fn get_flo_info(data: &[u8]) -> Result<FloInfo> {
    let reader = libflo_audio::Reader::new();
    let file = reader
        .read(data)
        .map_err(|e| anyhow::anyhow!("Failed to read flo file: {}", e))?;

    let duration_secs = file.header.total_frames as f64 / file.header.sample_rate as f64;
    let original_size = ((file.header.total_frames as f64)
        * (file.header.sample_rate as f64)
        * (file.header.channels as f64)
        * ((file.header.bit_depth as f64) / 8.0)) as usize;
    let compression_ratio = if !data.is_empty() {
        (original_size as f64) / (data.len() as f64)
    } else {
        0.0
    };

    // Check CRC
    let start = (4 + file.header.header_size + file.header.toc_size) as usize;
    let end = start + (file.header.data_size as usize);
    let crc_valid = if end <= data.len() {
        let data_slice = &data[start..end];
        let computed_crc = libflo_audio::compute_crc32(data_slice);
        computed_crc == file.header.data_crc32
    } else {
        false
    };

    // Check lossy mode from flags
    let is_lossy = (file.header.flags & 0x01) != 0;
    let lossy_quality = ((file.header.flags >> 8) & 0x0f) as u8;

    Ok(FloInfo {
        version: format!(
            "{}.{}",
            file.header.version_major, file.header.version_minor
        ),
        sample_rate: file.header.sample_rate,
        channels: file.header.channels,
        bit_depth: file.header.bit_depth,
        total_frames: file.header.total_frames,
        duration_secs,
        file_size: data.len(),
        compression_ratio,
        crc_valid,
        is_lossy,
        lossy_quality,
    })
}

/// Validate a flo™ file
pub fn validate_flo(data: &[u8]) -> Result<bool> {
    let info = get_flo_info(data)?;
    Ok(info.crc_valid)
}

/// Encoding options for converting audio to flo™ format
#[derive(Debug, Clone)]
pub struct EncodeOptions {
    /// Compression level (0-9) for lossless mode
    pub level: u8,
    /// Enable lossy compression
    pub lossy: bool,
    /// Lossy quality (0.0-1.0)
    pub quality: f32,
    /// Target bitrate in kbps (overrides quality)
    pub bitrate: Option<u32>,
    /// Metadata to embed
    pub metadata: Option<FloMetadata>,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            level: 5,
            lossy: false,
            quality: 0.6, // High quality
            bitrate: None,
            metadata: None,
        }
    }
}

impl EncodeOptions {
    /// Create options for lossless encoding
    pub fn lossless() -> Self {
        Self {
            lossy: false,
            ..Default::default()
        }
    }

    /// Create options for lossy encoding with specified quality
    /// Quality ranges from 0.0 (low) to 1.0 (transparent)
    pub fn lossy(quality: f32) -> Self {
        Self {
            lossy: true,
            quality: quality.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Create options for lossy encoding with target bitrate
    pub fn lossy_bitrate(bitrate: u32) -> Self {
        Self {
            lossy: true,
            bitrate: Some(bitrate),
            ..Default::default()
        }
    }

    /// Set compression level (0-9) for lossless mode
    pub fn with_level(mut self, level: u8) -> Self {
        self.level = level.min(9);
        self
    }

    /// Set metadata to embed in the file
    pub fn with_metadata(mut self, metadata: FloMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Information about a decoded audio file
#[derive(Debug, Clone)]
pub struct AudioInfo {
    pub sample_rate: u32,
    pub channels: usize,
    pub duration_secs: f32,
}

/// Encode audio file bytes to flo™ format
///
/// # Arguments
/// * `audio_bytes` - Raw bytes of an audio file (MP3, WAV, FLAC, OGG, etc.)
/// * `options` - Encoding options
///
/// # Returns
/// Raw bytes of the flo™ file
pub fn encode_from_audio(audio_bytes: &[u8], options: EncodeOptions) -> Result<Vec<u8>> {
    // Read audio file
    let (samples, sample_rate, channels, source_meta) =
        audio::read_audio_from_bytes(audio_bytes).context("Failed to read audio file")?;

    encode_from_samples(&samples, sample_rate, channels, source_meta, options)
}

/// Encode raw audio samples to flo™ format
///
/// # Arguments
/// * `samples` - Interleaved f32 samples in range [-1.0, 1.0]
/// * `sample_rate` - Sample rate in Hz
/// * `channels` - Number of channels
/// * `source_metadata` - Optional source metadata to preserve
/// * `options` - Encoding options
///
/// # Returns
/// Raw bytes of the flo™ file
pub fn encode_from_samples(
    samples: &[f32],
    sample_rate: u32,
    channels: usize,
    source_metadata: audio::AudioMetadata,
    options: EncodeOptions,
) -> Result<Vec<u8>> {
    // Build metadata - options override source metadata
    let mut meta = options.metadata.unwrap_or_else(|| {
        let mut m = FloMetadata::new();

        // Start with source metadata
        m.title = source_metadata.title;
        m.artist = source_metadata.artist;
        m.album = source_metadata.album;
        m.album_artist = source_metadata.album_artist;
        m.year = source_metadata.year.map(|y| y as u32);
        m.genre = source_metadata.genre;
        if let Some(t) = source_metadata.track_number {
            m.track_number = Some(t);
        }
        if let Some(b) = source_metadata.bpm {
            m.bpm = Some(b as u32);
        }
        if let Some(c) = source_metadata.comment {
            m.comments = vec![libflo_audio::Comment {
                language: Some("eng".to_string()),
                description: None,
                text: c,
            }];
        }

        // Add cover art
        if let Some((mime, data)) = source_metadata.cover_art {
            m.pictures = vec![libflo_audio::Picture {
                picture_type: libflo_audio::PictureType::CoverFront,
                mime_type: mime,
                description: None,
                data,
            }];
        }

        m
    });
    
    // Always set encoding info fields
    meta.flo_encoder_version = Some(format!("reflo {}", env!("CARGO_PKG_VERSION")));
    
    // Get current time - use js_sys for WASM, chrono for native
    #[cfg(all(target_arch = "wasm32", feature = "wasm"))]
    let encoding_time = {
        let date = js_sys::Date::new_0();
        date.to_iso_string().as_string().unwrap_or_default()
    };
    #[cfg(not(all(target_arch = "wasm32", feature = "wasm")))]
    let encoding_time = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    
    meta.encoding_time = Some(encoding_time);
    meta.source_format = source_metadata.source_format.or(meta.source_format);
    meta.original_filename = source_metadata.original_filename.or(meta.original_filename);
    
    // Set encoder settings description
    let settings_desc = if options.lossy || options.bitrate.is_some() {
        if let Some(br) = options.bitrate {
            format!("Lossy, target {}kbps", br)
        } else {
            format!("Lossy, quality {:.0}%", options.quality * 100.0)
        }
    } else {
        format!("Lossless, level {}", options.level)
    };
    meta.encoder_settings = Some(settings_desc);

    // Always serialize metadata since we now have encoding info
    let metadata_bytes = meta.to_msgpack().ok();

    let metadata_data = metadata_bytes.unwrap_or_default();

    // Handle lossy vs lossless mode
    let flo_data = if options.lossy || options.bitrate.is_some() {
        // Lossy encoding using TransformEncoder
        let quality_value = if let Some(br) = options.bitrate {
            libflo_audio::QualityPreset::from_bitrate(br, sample_rate, channels as u8).as_f32()
        } else {
            options.quality
        };

        let mut encoder =
            libflo_audio::LossyEncoder::new(sample_rate, channels as u8, quality_value);
        encoder
            .encode_to_flo(samples, &metadata_data)
            .map_err(|e| anyhow::anyhow!("Encoding failed: {}", e))?
    } else {
        // Lossless encoding
        let encoder = libflo_audio::Encoder::new(sample_rate, channels as u8, 16)
            .with_compression(options.level);
        encoder
            .encode(samples, &metadata_data)
            .map_err(|e| anyhow::anyhow!("Encoding failed: {}", e))?
    };

    Ok(flo_data)
}

/// Decode flo™ file to raw samples
///
/// # Arguments
/// * `flo_bytes` - Raw bytes of a flo™ file
///
/// # Returns
/// Tuple of (samples, sample_rate, channels) where samples are interleaved f32
pub fn decode_to_samples(flo_bytes: &[u8]) -> Result<(Vec<f32>, u32, usize)> {
    // Read file using Reader
    let reader = libflo_audio::Reader::new();
    let file = reader
        .read(flo_bytes)
        .map_err(|e| anyhow::anyhow!("Invalid flo™ file: {}", e))?;

    let sample_rate = file.header.sample_rate;
    let channels = file.header.channels as usize;

    // Check if lossy or lossless
    let is_lossy = (file.header.flags & 0x01) != 0;

    let samples = if is_lossy {
        // Lossy decoding using TransformDecoder
        let mut decoder = libflo_audio::LossyDecoder::new(sample_rate, file.header.channels);
        let mut all_samples = Vec::new();
        let mut frame_count = 0;

        for frame in &file.frames {
            if frame.channels.is_empty() {
                continue;
            }

            // Transform frames store serialized data in first channel's residuals
            let frame_data = &frame.channels[0].residuals;

            // Deserialize and decode
            let transform_frame = libflo_audio::deserialize_frame(frame_data)
                .ok_or_else(|| anyhow::anyhow!("Failed to deserialize lossy frame"))?;

            let frame_samples = decoder.decode_frame(&transform_frame);

            // Skip the first frame's output (pre-roll silence for proper MDCT overlap-add)
            if frame_count > 0 {
                all_samples.extend(frame_samples);
            }
            frame_count += 1;
        }
        all_samples
    } else {
        // Lossless decoding
        let decoder = libflo_audio::Decoder::new();
        decoder
            .decode_file(&file)
            .map_err(|e| anyhow::anyhow!("Lossless decoding failed: {}", e))?
    };

    Ok((samples, sample_rate, channels))
}

/// Decode flo™ file to WAV format
///
/// # Arguments
/// * `flo_bytes` - Raw bytes of a flo™ file
///
/// # Returns
/// Raw bytes of a WAV file
pub fn decode_to_wav(flo_bytes: &[u8]) -> Result<Vec<u8>> {
    let (samples, sample_rate, channels) = decode_to_samples(flo_bytes)?;

    audio::write_wav_to_bytes(&samples, sample_rate, channels).context("Failed to write WAV data")
}

/// Get metadata from a flo™ file
///
/// # Arguments
/// * `flo_bytes` - Raw bytes of a flo™ file
///
/// # Returns
/// Metadata if present, or None
pub fn get_metadata(flo_bytes: &[u8]) -> Result<Option<FloMetadata>> {
    let reader = libflo_audio::Reader::new();
    let file = reader
        .read(flo_bytes)
        .map_err(|e| anyhow::anyhow!("Invalid flo™ file: {}", e))?;

    if file.metadata.is_empty() {
        return Ok(None);
    }

    let meta = FloMetadata::from_msgpack(&file.metadata)
        .map_err(|e| anyhow::anyhow!("Invalid metadata: {}", e))?;

    Ok(Some(meta))
}

/// Get information about an audio file
///
/// # Arguments
/// * `audio_bytes` - Raw bytes of an audio file (MP3, WAV, FLAC, OGG, etc.)
///
/// # Returns
/// Audio information
pub fn get_audio_info(audio_bytes: &[u8]) -> Result<AudioInfo> {
    let (samples, sample_rate, channels, _) =
        audio::read_audio_from_bytes(audio_bytes).context("Failed to read audio file")?;

    Ok(AudioInfo {
        sample_rate,
        channels,
        duration_secs: samples.len() as f32 / channels as f32 / sample_rate as f32,
    })
}

// ============================================================================
// Metadata Editing
// ============================================================================

/// Update metadata in a flo™ file WITHOUT re-encoding the audio!
/// This is instant because flo™ stores metadata in a separate chunk.
///
/// # Arguments
/// * `flo_bytes` - Original flo™ file bytes
/// * `metadata` - Metadata object (will be converted to MessagePack)
///
/// # Returns
/// New flo™ file bytes with updated metadata
#[cfg(target_arch = "wasm32")]
pub fn update_metadata_no_reencode(
    flo_bytes: &[u8],
    metadata: wasm_bindgen::JsValue,
) -> Result<Vec<u8>> {
    // Convert JS object to FloMetadata
    let meta = metadata_from_js(metadata)?;
    let meta_bytes = meta
        .to_msgpack()
        .map_err(|e| anyhow::anyhow!("Failed to serialize metadata: {}", e))?;

    update_metadata_bytes(flo_bytes, &meta_bytes)
}

/// Update metadata bytes in a flo™ file (internal implementation)
pub fn update_metadata_bytes(flo_bytes: &[u8], new_metadata: &[u8]) -> Result<Vec<u8>> {
    // Use libflo's efficient update function
    libflo_audio::update_metadata_bytes(flo_bytes, new_metadata)
        .map_err(|e| anyhow::anyhow!("Failed to update metadata: {}", e))
}

/// Strip all metadata from a flo™ file WITHOUT re-encoding
pub fn strip_metadata_no_reencode(flo_bytes: &[u8]) -> Result<Vec<u8>> {
    update_metadata_bytes(flo_bytes, &[])
}

/// Check if a flo™ file has metadata (fast - reads header only)
pub fn has_metadata(flo_bytes: &[u8]) -> bool {
    libflo_audio::has_metadata(flo_bytes)
}

/// Convert JavaScript metadata object to FloMetadata
#[cfg(target_arch = "wasm32")]
fn metadata_from_js(metadata: wasm_bindgen::JsValue) -> Result<FloMetadata> {
    use wasm_bindgen::JsCast;

    let obj = metadata
        .dyn_ref::<js_sys::Object>()
        .ok_or_else(|| anyhow::anyhow!("Metadata must be an object"))?;

    let mut meta = FloMetadata::default();

    // Helper to get string field
    let get_str = |key: &str| -> Option<String> {
        js_sys::Reflect::get(obj, &wasm_bindgen::JsValue::from_str(key))
            .ok()
            .and_then(|v| v.as_string())
            .filter(|s| !s.is_empty())
    };

    // Helper to get u32 field
    let get_u32 = |key: &str| -> Option<u32> {
        js_sys::Reflect::get(obj, &wasm_bindgen::JsValue::from_str(key))
            .ok()
            .and_then(|v| v.as_f64())
            .map(|n| n as u32)
            .filter(|&n| n > 0)
    };

    meta.title = get_str("title");
    meta.artist = get_str("artist");
    meta.album = get_str("album");
    meta.year = get_u32("year");
    meta.genre = get_str("genre");
    meta.track_number = get_u32("track");
    meta.bpm = get_u32("bpm");
    meta.key = get_str("key");

    // Handle comment as a single comment in the comments vector
    if let Some(comment_text) = get_str("comment") {
        meta.comments = vec![libflo_audio::Comment {
            language: None,
            description: None,
            text: comment_text,
        }];
    }

    Ok(meta)
}
