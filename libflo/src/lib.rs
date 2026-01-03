#![allow(clippy::needless_range_loop)]

use wasm_bindgen::prelude::*;

pub mod core;
pub mod lossless;
pub mod lossy;
pub mod streaming;

mod reader;
mod writer;

pub use core::{
    compute_crc32, metadata::*, rice, ChannelData, FloFile, FloResult, FrameType, ResidualEncoding,
    HEADER_SIZE, MAGIC, VERSION_MAJOR, VERSION_MINOR,
};
pub use lossless::{lpc, Decoder, Encoder};
pub use lossy::{
    deserialize_frame, serialize_frame, BlockSize, Mdct, PsychoacousticModel, QualityPreset,
    TransformDecoder as LossyDecoder, TransformEncoder as LossyEncoder, TransformFrame, WindowType,
};
pub use reader::Reader;
pub use streaming::{
    DecoderState, EncodedFrame, StreamingAudioInfo, StreamingDecoder, StreamingEncoder,
};
pub use writer::Writer;

// audio info for the info() function

/// info about a flo file
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct AudioInfo {
    /// version string like "1.0"
    #[wasm_bindgen(skip)]
    pub version: String,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u8,
    /// Bits per sample
    pub bit_depth: u8,
    /// Total number of frames
    pub total_frames: u64,
    /// Duration in seconds
    pub duration_secs: f64,
    /// File size in bytes
    pub file_size: usize,
    /// Compression ratio (original / compressed)
    pub compression_ratio: f64,
    /// Is CRC valid?
    pub crc_valid: bool,
    /// Is lossy compression mode?
    pub is_lossy: bool,
    /// Lossy quality 0-4 (only valid if is_lossy)
    pub lossy_quality: u8,
}

#[wasm_bindgen]
impl AudioInfo {
    #[wasm_bindgen(getter)]
    pub fn version(&self) -> String {
        self.version.clone()
    }
}

// result helpers

/// turn an error into js
fn to_js_err(e: String) -> JsValue {
    JsValue::from_str(&e)
}

// api functions

/// encode samples to flo lossless
///
/// # Arguments
/// * `samples` - Interleaved audio samples (f32, -1.0 to 1.0)
/// * `sample_rate` - Sample rate in Hz (e.g., 44100)
/// * `channels` - Number of channels (1 or 2)
/// * `bit_depth` - Bits per sample (16, 24, or 32)
/// * `metadata` - Optional MessagePack metadata
///
/// # Returns
/// flo™ file as byte array
///
/// # Note
/// For advanced usage with custom compression levels (0-9),
/// use the `Encoder` builder pattern directly.
#[wasm_bindgen]
pub fn encode(
    samples: &[f32],
    sample_rate: u32,
    channels: u8,
    bit_depth: u8,
    metadata: Option<Vec<u8>>,
) -> Result<Vec<u8>, JsValue> {
    let encoder = Encoder::new(sample_rate, channels, bit_depth);
    encoder
        .encode(samples, &metadata.unwrap_or_default())
        .map_err(to_js_err)
}

/// encode samples to flo lossy
///
/// # Arguments
/// * `samples` - Interleaved audio samples (f32, -1.0 to 1.0)
/// * `sample_rate` - Sample rate in Hz (e.g., 44100)
/// * `channels` - Number of audio channels (1 or 2)
/// * `bit_depth` - Bits per sample (typically 16)
/// * `quality` - Quality level 0-4 (0=low/~64kbps, 4=transparent/~320kbps)
/// * `metadata` - Optional MessagePack metadata
///
/// # Returns
/// flo™ file as byte array
///
/// # Note
/// For advanced usage with continuous quality control (0.0-1.0) or custom settings,
/// use the `LossyEncoder` builder pattern directly.
#[wasm_bindgen]
pub fn encode_lossy(
    samples: &[f32],
    sample_rate: u32,
    channels: u8,
    _bit_depth: u8,
    quality: u8,
    metadata: Option<Vec<u8>>,
) -> Result<Vec<u8>, JsValue> {
    // quality levels to 0.0-1.0
    let quality_f32 = match quality {
        0 => 0.0,
        1 => 0.35,
        2 => 0.55,
        3 => 0.75,
        _ => 1.0,
    };

    let mut encoder = lossy::TransformEncoder::new(sample_rate, channels, quality_f32);
    encoder
        .encode_to_flo(samples, &metadata.unwrap_or_default())
        .map_err(to_js_err)
}

/// encode to flo lossy with target bitrate
///
/// # Arguments
/// * `samples` - Interleaved audio samples (f32, -1.0 to 1.0)
/// * `sample_rate` - Sample rate in Hz (e.g., 44100)
/// * `channels` - Number of audio channels
/// * `bit_depth` - Bits per sample (16, 24, or 32)
/// * `target_bitrate_kbps` - Target bitrate in kbps (e.g., 128, 192, 256, 320)
/// * `metadata` - Optional MessagePack metadata
///
/// # Returns
/// flo™ file as byte array
#[wasm_bindgen]
pub fn encode_with_bitrate(
    samples: &[f32],
    sample_rate: u32,
    channels: u8,
    _bit_depth: u8,
    target_bitrate_kbps: u32,
    metadata: Option<Vec<u8>>,
) -> Result<Vec<u8>, JsValue> {
    // bitrate to quality
    let quality =
        lossy::QualityPreset::from_bitrate(target_bitrate_kbps, sample_rate, channels).as_f32();
    let mut encoder = lossy::TransformEncoder::new(sample_rate, channels, quality);
    encoder
        .encode_to_flo(samples, &metadata.unwrap_or_default())
        .map_err(to_js_err)
}

/// decode flo file to samples
///
/// This automatically detects whether the file uses lossless or lossy encoding
/// and dispatches to the appropriate decoder.
///
/// # Arguments
/// * `data` - flo™ file bytes
///
/// # Returns
/// Interleaved audio samples (f32, -1.0 to 1.0)
#[wasm_bindgen]
pub fn decode(data: &[u8]) -> Result<Vec<f32>, JsValue> {
    // figure out if its transform/lossy
    let reader = Reader::new();
    let file = reader.read(data).map_err(to_js_err)?;

    // any transform frames means lossy
    let is_transform = file
        .frames
        .iter()
        .any(|f| f.frame_type == (FrameType::Transform as u8));

    if is_transform {
        // lossy
        decode_transform_file(&file).map_err(to_js_err)
    } else {
        // lossless
        let decoder = Decoder::new();
        decoder.decode_file(&file).map_err(to_js_err)
    }
}

/// Decode a lossy flo™ file that uses transform-based compression
///
/// # Arguments
/// * `file` - Parsed flo™ file
///
/// # Returns
/// Interleaved audio samples (f32, -1.0 to 1.0)
/// Decode a transform-based lossy file
fn decode_transform_file(file: &FloFile) -> FloResult<Vec<f32>> {
    let mut decoder = lossy::TransformDecoder::new(file.header.sample_rate, file.header.channels);
    let mut all_samples = Vec::new();
    let mut frame_count = 0;

    for frame in &file.frames {
        if frame.channels.is_empty() {
            continue;
        }

        // transform data is in first channels residuals
        let frame_data = &frame.channels[0].residuals;

        if let Some(transform_frame) = lossy::deserialize_frame(frame_data) {
            let samples = decoder.decode_frame(&transform_frame);

            // skip first frame (pre-roll for overlap-add)
            if frame_count > 0 {
                all_samples.extend(samples);
            }
            frame_count += 1;
        } else {
            return Err("Failed to deserialize transform frame".to_string());
        }
    }

    Ok(all_samples)
}

/// Validate flo™ file integrity
///
/// # Arguments
/// * `data` - flo™ file bytes
///
/// # Returns
/// true if file is valid and CRC matches
#[wasm_bindgen]
pub fn validate(data: &[u8]) -> Result<bool, JsValue> {
    let reader = Reader::new();
    match reader.read(data) {
        Ok(file) => {
            let start = (4 + file.header.header_size + file.header.toc_size) as usize;
            let end = start + (file.header.data_size as usize);
            if end <= data.len() {
                let computed = core::crc32::compute(&data[start..end]);
                Ok(computed == file.header.data_crc32)
            } else {
                Ok(false)
            }
        }
        Err(_) => Ok(false),
    }
}

/// Get information about a flo™ file
///
/// # Arguments
/// * `data` - flo™ file bytes
///
/// # Returns
/// AudioInfo struct with file details
#[wasm_bindgen]
pub fn info(data: &[u8]) -> Result<AudioInfo, JsValue> {
    let reader = Reader::new();
    let file = reader.read(data).map_err(to_js_err)?;

    let duration_secs = file.header.total_frames as f64;
    let original_size = ((file.header.total_frames as f64)
        * (file.header.sample_rate as f64)
        * (file.header.channels as f64)
        * ((file.header.bit_depth as f64) / 8.0)) as usize;
    let compression_ratio = if !data.is_empty() {
        (original_size as f64) / (data.len() as f64)
    } else {
        0.0
    };

    // check crc
    let start = (4 + file.header.header_size + file.header.toc_size) as usize;
    let end = start + (file.header.data_size as usize);
    let crc_valid = if end <= data.len() {
        core::crc32::compute(&data[start..end]) == file.header.data_crc32
    } else {
        false
    };

    // lossy mode from flags
    let is_lossy = (file.header.flags & 0x01) != 0;
    let lossy_quality = ((file.header.flags >> 8) & 0x0f) as u8;

    Ok(AudioInfo {
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

/// get lib version
#[wasm_bindgen]
pub fn version() -> String {
    format!("{}.{}", VERSION_MAJOR, VERSION_MINOR)
}

// streaming decoder wasm api

#[wasm_bindgen]
pub struct WasmStreamingDecoder {
    inner: StreamingDecoder,
}

#[wasm_bindgen]
impl WasmStreamingDecoder {
    /// new streaming decoder
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: StreamingDecoder::new(),
        }
    }

    /// feed data to the decoder, call as bytes come in from network
    #[wasm_bindgen]
    pub fn feed(&mut self, data: &[u8]) -> Result<bool, JsValue> {
        self.inner.feed(data).map_err(to_js_err)
    }

    /// Check if the decoder is ready to produce audio
    #[wasm_bindgen]
    pub fn is_ready(&self) -> bool {
        self.inner.state() == DecoderState::Ready
    }

    /// stream done?
    #[wasm_bindgen]
    pub fn is_finished(&self) -> bool {
        self.inner.state() == DecoderState::Finished
    }

    /// Check if there was an error
    #[wasm_bindgen]
    pub fn has_error(&self) -> bool {
        self.inner.state() == DecoderState::Error
    }

    /// Get the current state as a string
    #[wasm_bindgen]
    pub fn state(&self) -> String {
        match self.inner.state() {
            DecoderState::WaitingForHeader => "waiting_for_header".into(),
            DecoderState::WaitingForToc => "waiting_for_toc".into(),
            DecoderState::Ready => "ready".into(),
            DecoderState::Finished => "finished".into(),
            DecoderState::Error => "error".into(),
        }
    }

    /// Get audio information (available after header is parsed)
    ///
    /// Returns null if header hasn't been parsed yet.
    #[wasm_bindgen]
    pub fn get_info(&self) -> Result<JsValue, JsValue> {
        match self.inner.info() {
            Some(info) => {
                let obj = js_sys::Object::new();
                js_sys::Reflect::set(&obj, &"sample_rate".into(), &info.sample_rate.into())?;
                js_sys::Reflect::set(&obj, &"channels".into(), &info.channels.into())?;
                js_sys::Reflect::set(&obj, &"bit_depth".into(), &info.bit_depth.into())?;
                js_sys::Reflect::set(
                    &obj,
                    &"total_frames".into(),
                    &(info.total_frames as f64).into(),
                )?;
                js_sys::Reflect::set(&obj, &"is_lossy".into(), &info.is_lossy.into())?;
                Ok(obj.into())
            }
            None => Ok(JsValue::NULL),
        }
    }

    /// decode all currently available samples
    #[wasm_bindgen]
    pub fn decode_available(&mut self) -> Result<Vec<f32>, JsValue> {
        self.inner.decode_available().map_err(to_js_err)
    }

    /// Decode the next available frame
    ///
    /// Returns interleaved f32 samples for one frame, or null if no frame is ready.
    /// This enables true streaming: decode and play frames as they arrive.
    ///
    /// Usage pattern:
    /// ```js
    /// while (true) {
    ///     const samples = decoder.next_frame();
    ///     if (samples === null) break; // No more frames ready
    ///     playAudio(samples);
    /// }
    /// ```
    #[wasm_bindgen]
    pub fn next_frame(&mut self) -> Result<JsValue, JsValue> {
        match self.inner.next_frame() {
            Ok(Some(samples)) => {
                let array = js_sys::Float32Array::new_with_length(samples.len() as u32);
                array.copy_from(&samples);
                Ok(array.into())
            }
            Ok(None) => Ok(JsValue::NULL),
            Err(e) => Err(to_js_err(e)),
        }
    }

    /// how many frames ready to decode
    #[wasm_bindgen]
    pub fn available_frames(&self) -> usize {
        self.inner.available_frames()
    }

    /// current frame index
    #[wasm_bindgen]
    pub fn current_frame_index(&self) -> usize {
        self.inner.current_frame_index()
    }

    /// Reset the decoder to initial state
    ///
    /// Use this to start decoding a new stream.
    #[wasm_bindgen]
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    /// bytes currently buffered
    #[wasm_bindgen]
    pub fn buffered_bytes(&self) -> usize {
        self.inner.buffered_bytes()
    }
}

impl Default for WasmStreamingDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Create metadata from basic fields and serialize to MessagePack
///
/// # Arguments
/// * `title` - Optional title
/// * `artist` - Optional artist
/// * `album` - Optional album
///
/// # Returns
/// MessagePack bytes containing metadata
#[wasm_bindgen]
pub fn create_metadata(
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
) -> Result<Vec<u8>, JsValue> {
    let meta = FloMetadata::with_basic(title, artist, album);
    meta.to_msgpack()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Create metadata from a JavaScript object
///
/// Accepts an object with any of the supported metadata fields.
/// See FloMetadata for available fields.
///
/// # Returns
/// MessagePack bytes containing metadata
#[wasm_bindgen]
pub fn create_metadata_from_object(obj: JsValue) -> Result<Vec<u8>, JsValue> {
    let meta: FloMetadata = serde_wasm_bindgen::from_value(obj)
        .map_err(|e| JsValue::from_str(&format!("Invalid metadata: {}", e)))?;
    meta.to_msgpack()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Extract metadata from a flo™ file
///
/// # Arguments
/// * `data` - flo™ file bytes
///
/// # Returns
/// JavaScript object with metadata fields (or null if no metadata)
#[wasm_bindgen]
pub fn get_metadata(data: &[u8]) -> Result<JsValue, JsValue> {
    let reader = Reader::new();
    let file = reader.read(data).map_err(to_js_err)?;

    if file.metadata.is_empty() {
        return Ok(JsValue::NULL);
    }

    let meta = FloMetadata::from_msgpack(&file.metadata)
        .map_err(|e| JsValue::from_str(&format!("Invalid metadata: {}", e)))?;

    serde_wasm_bindgen::to_value(&meta)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Get cover art from a flo™ file
///
/// # Arguments
/// * `data` - flo™ file bytes
///
/// # Returns
/// Object with `mime_type` and `data` (Uint8Array) or null if no cover
#[wasm_bindgen]
pub fn get_cover_art(data: &[u8]) -> Result<JsValue, JsValue> {
    let reader = Reader::new();
    let file = reader.read(data).map_err(to_js_err)?;

    if file.metadata.is_empty() {
        return Ok(JsValue::NULL);
    }

    let meta = FloMetadata::from_msgpack(&file.metadata)
        .map_err(|e| JsValue::from_str(&format!("Invalid metadata: {}", e)))?;

    // try front cover first
    if let Some(pic) = meta.front_cover().or_else(|| meta.any_picture()) {
        let obj = js_sys::Object::new();
        js_sys::Reflect::set(&obj, &"mime_type".into(), &pic.mime_type.clone().into())?;
        js_sys::Reflect::set(
            &obj,
            &"data".into(),
            &js_sys::Uint8Array::from(&pic.data[..]).into(),
        )?;
        if let Some(ref desc) = pic.description {
            js_sys::Reflect::set(&obj, &"description".into(), &desc.clone().into())?;
        }
        Ok(obj.into())
    } else {
        Ok(JsValue::NULL)
    }
}

/// Set a single field in existing metadata bytes
///
/// Uses serde to dynamically set fields - field names match FloMetadata struct.
/// For complex fields (pictures, synced_lyrics, etc.) use create_metadata_from_object.
///
/// # Arguments
/// * `metadata` - Existing MessagePack metadata bytes (or empty for new)
/// * `field` - Field name (e.g., "title", "artist", "bpm")
/// * `value` - Field value (string, number, or null)
///
/// # Returns
/// Updated MessagePack metadata bytes
#[wasm_bindgen]
pub fn set_metadata_field(
    metadata: Option<Vec<u8>>,
    field: &str,
    value: JsValue,
) -> Result<Vec<u8>, JsValue> {
    // parse or create new
    let meta = match &metadata {
        Some(data) if !data.is_empty() => FloMetadata::from_msgpack(data)
            .map_err(|e| JsValue::from_str(&format!("Invalid metadata: {}", e)))?,
        _ => FloMetadata::new(),
    };

    // modify via serde
    let mut obj: serde_json::Value = serde_json::to_value(&meta)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))?;

    // jsvalue to serde
    let json_value = if value.is_null() || value.is_undefined() {
        serde_json::Value::Null
    } else if let Some(s) = value.as_string() {
        serde_json::Value::String(s)
    } else if let Some(n) = value.as_f64() {
        serde_json::json!(n)
    } else if let Some(b) = value.as_bool() {
        serde_json::Value::Bool(b)
    } else {
        // try json for complex stuff
        let js_json = js_sys::JSON::stringify(&value)
            .map_err(|_| JsValue::from_str("Cannot serialize value"))?;
        serde_json::from_str(&js_json.as_string().unwrap_or_default())
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON: {}", e)))?
    };

    // do it
    if let serde_json::Value::Object(ref mut map) = obj {
        map.insert(field.to_string(), json_value);
    } else {
        return Err(JsValue::from_str("Internal error: metadata not an object"));
    }

    // back to struct
    let updated: FloMetadata = serde_json::from_value(obj)
        .map_err(|e| JsValue::from_str(&format!("Invalid field '{}': {}", field, e)))?;

    updated
        .to_msgpack()
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get synced lyrics from a flo™ file
///
/// # Returns
/// Array of synced lyrics objects or null if none
#[wasm_bindgen]
pub fn get_synced_lyrics(data: &[u8]) -> Result<JsValue, JsValue> {
    let reader = Reader::new();
    let file = reader.read(data).map_err(to_js_err)?;

    if file.metadata.is_empty() {
        return Ok(JsValue::NULL);
    }

    let meta = FloMetadata::from_msgpack(&file.metadata)
        .map_err(|e| JsValue::from_str(&format!("Invalid metadata: {}", e)))?;

    if meta.synced_lyrics.is_empty() {
        return Ok(JsValue::NULL);
    }

    serde_wasm_bindgen::to_value(&meta.synced_lyrics)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Get waveform data from a flo™ file for instant visualization
///
/// # Returns
/// WaveformData object or null if not present
#[wasm_bindgen]
pub fn get_waveform_data(data: &[u8]) -> Result<JsValue, JsValue> {
    let reader = Reader::new();
    let file = reader.read(data).map_err(to_js_err)?;

    if file.metadata.is_empty() {
        return Ok(JsValue::NULL);
    }

    let meta = FloMetadata::from_msgpack(&file.metadata)
        .map_err(|e| JsValue::from_str(&format!("Invalid metadata: {}", e)))?;

    match meta.waveform_data {
        Some(ref waveform) => serde_wasm_bindgen::to_value(waveform)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
        None => Ok(JsValue::NULL),
    }
}

/// Get section markers from a flo™ file
///
/// # Returns
/// Array of section markers or null if none
#[wasm_bindgen]
pub fn get_section_markers(data: &[u8]) -> Result<JsValue, JsValue> {
    let reader = Reader::new();
    let file = reader.read(data).map_err(to_js_err)?;

    if file.metadata.is_empty() {
        return Ok(JsValue::NULL);
    }

    let meta = FloMetadata::from_msgpack(&file.metadata)
        .map_err(|e| JsValue::from_str(&format!("Invalid metadata: {}", e)))?;

    if meta.section_markers.is_empty() {
        return Ok(JsValue::NULL);
    }

    serde_wasm_bindgen::to_value(&meta.section_markers)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

// zero-copy metadata editing

/// update metadata without re-encoding audio
///
/// # Arguments
/// * `flo_data` - Original flo™ file bytes
/// * `new_metadata` - New MessagePack metadata bytes (use create_metadata_*)
///
/// # Returns
/// New flo™ file with updated metadata
#[wasm_bindgen]
pub fn update_metadata(flo_data: &[u8], new_metadata: &[u8]) -> Result<Vec<u8>, JsValue> {
    update_metadata_bytes(flo_data, new_metadata).map_err(to_js_err)
}

/// update metadata without re-encoding (native)
pub fn update_metadata_bytes(flo_data: &[u8], new_metadata: &[u8]) -> FloResult<Vec<u8>> {
    // basic checks
    if flo_data.len() < HEADER_SIZE as usize {
        return Err("File too small to be valid flo".to_string());
    }

    // check magic
    if flo_data[0..4] != MAGIC {
        return Err("Invalid flo file: bad magic".to_string());
    }

    // read header for chunk sizes
    let reader = Reader::new();
    let file = reader.read(flo_data)?;

    // find where metadata starts
    // layout: magic(4) + header + toc + data + extra + metadata
    let meta_offset = 4
        + file.header.header_size as usize
        + file.header.toc_size as usize
        + file.header.data_size as usize
        + file.header.extra_size as usize;

    // copy up to metadata
    let mut result = Vec::with_capacity(meta_offset + new_metadata.len());
    result.extend_from_slice(&flo_data[..meta_offset]);

    // add new metadata
    result.extend_from_slice(new_metadata);

    // fix meta_size in header
    // header layout: version(2) + flags(2) + sample_rate(4) + ...
    // meta_size is at offset: 4 + 58 = 62
    let meta_size_offset = 4 + 2 + 2 + 4 + 1 + 1 + 8 + 1 + 3 + 4 + 8 + 8 + 8 + 8;
    let new_meta_size = new_metadata.len() as u64;
    result[meta_size_offset..meta_size_offset + 8].copy_from_slice(&new_meta_size.to_le_bytes());

    Ok(result)
}

/// Replace just the metadata in a flo™ file (convenience function)
///
/// Takes a metadata object directly instead of MessagePack bytes.
///
/// # Arguments
/// * `flo_data` - Original flo™ file bytes
/// * `metadata` - JavaScript metadata object
///
/// # Returns
/// New flo™ file with updated metadata
#[wasm_bindgen]
pub fn set_metadata(flo_data: &[u8], metadata: JsValue) -> Result<Vec<u8>, JsValue> {
    let new_meta_bytes = create_metadata_from_object(metadata)?;
    update_metadata(flo_data, &new_meta_bytes)
}

/// Remove all metadata from a flo™ file
///
/// # Arguments
/// * `flo_data` - Original flo™ file bytes
///
/// # Returns
/// New flo™ file with no metadata
pub fn strip_metadata_bytes(flo_data: &[u8]) -> FloResult<Vec<u8>> {
    update_metadata_bytes(flo_data, &[])
}

/// Remove all metadata from a flo™ file
///
/// # Arguments
/// * `flo_data` - Original flo™ file bytes
///
/// # Returns
/// New flo™ file with no metadata
#[wasm_bindgen]
pub fn strip_metadata(flo_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    strip_metadata_bytes(flo_data).map_err(to_js_err)
}

/// Get just the metadata bytes from a flo™ file
///
/// # Arguments
/// * `flo_data` - flo™ file bytes
///
/// # Returns
/// Raw MessagePack metadata bytes (or empty array)
#[wasm_bindgen]
pub fn get_metadata_bytes(flo_data: &[u8]) -> Result<Vec<u8>, JsValue> {
    get_metadata_bytes_native(flo_data).map_err(to_js_err)
}

/// Get just the metadata bytes from a flo™ file
///
/// # Arguments
/// * `flo_data` - flo™ file bytes
///
/// # Returns
/// Raw MessagePack metadata bytes (or empty array)
pub fn get_metadata_bytes_native(flo_data: &[u8]) -> FloResult<Vec<u8>> {
    if flo_data.len() < HEADER_SIZE as usize {
        return Err("File too small".to_string());
    }

    // just read header for metadata location
    let reader = Reader::new();
    let file = reader.read(flo_data)?;

    Ok(file.metadata)
}

/// does the file have metadata?
#[wasm_bindgen]
pub fn has_metadata(flo_data: &[u8]) -> bool {
    if flo_data.len() < HEADER_SIZE as usize {
        return false;
    }

    // fast path, just read meta_size from header
    let meta_size_offset = 4 + 2 + 2 + 4 + 1 + 1 + 8 + 1 + 3 + 4 + 8 + 8 + 8 + 8;
    if flo_data.len() < meta_size_offset + 8 {
        return false;
    }

    let meta_size = u64::from_le_bytes(
        flo_data[meta_size_offset..meta_size_offset + 8]
            .try_into()
            .unwrap_or([0; 8]),
    );

    meta_size > 0
}

// tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_type_conversion() {
        assert_eq!(FrameType::from(0), FrameType::Silence);
        assert_eq!(FrameType::from(8), FrameType::Alpc8);
        assert_eq!(FrameType::from(254), FrameType::Raw);
        assert!(FrameType::Alpc8.is_alpc());
        assert!(!FrameType::Silence.is_alpc());
        assert_eq!(FrameType::Alpc8.lpc_order(), Some(8));
    }

    #[test]
    fn test_version() {
        assert_eq!(version(), "1.0");
    }

    #[test]
    fn test_update_metadata_preserves_audio() {
        // Create a simple flo file with metadata
        let samples: Vec<f32> = (0..4410).map(|i| (i as f32 * 0.01).sin() * 0.5).collect();
        let encoder = Encoder::new(44100, 1, 16);
        let original_meta = b"original metadata";
        let flo_data = encoder.encode(&samples, original_meta).unwrap();

        // Update metadata
        let new_meta = b"new metadata that is longer!";
        let updated = update_metadata_bytes(&flo_data, new_meta).unwrap();

        // Verify the new file is valid
        let reader = Reader::new();
        let file = reader.read(&updated).unwrap();

        // Check metadata was updated
        assert_eq!(file.metadata, new_meta);
        assert_eq!(file.header.meta_size, new_meta.len() as u64);

        // Check audio data CRC is unchanged (proves audio wasn't touched)
        let original_file = reader.read(&flo_data).unwrap();
        assert_eq!(file.header.data_crc32, original_file.header.data_crc32);

        // Decode and verify audio is identical
        let decoder = Decoder::new();
        let original_samples = decoder.decode(&flo_data).unwrap();
        let updated_samples = decoder.decode(&updated).unwrap();
        assert_eq!(original_samples, updated_samples);
    }

    #[test]
    fn test_strip_metadata() {
        let samples: Vec<f32> = vec![0.5; 1000];
        let encoder = Encoder::new(44100, 1, 16);
        let flo_with_meta = encoder.encode(&samples, b"some metadata here").unwrap();

        // Strip metadata using empty bytes (simulates strip)
        let stripped = update_metadata_bytes(&flo_with_meta, &[]).unwrap();

        // Verify
        let reader = Reader::new();
        let file = reader.read(&stripped).unwrap();
        assert!(file.metadata.is_empty());
        assert_eq!(file.header.meta_size, 0);

        // File should be smaller
        assert!(stripped.len() < flo_with_meta.len());
    }

    #[test]
    fn test_has_metadata() {
        let samples: Vec<f32> = vec![0.5; 1000];
        let encoder = Encoder::new(44100, 1, 16);

        let with_meta = encoder.encode(&samples, b"metadata").unwrap();
        let without_meta = encoder.encode(&samples, &[]).unwrap();

        assert!(has_metadata(&with_meta));
        assert!(!has_metadata(&without_meta));
    }
}
