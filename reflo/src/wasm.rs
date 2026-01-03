#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn encode_audio_to_flo(
    audio_bytes: &[u8],
    lossy: bool,
    quality: f32,
    level: u8,
) -> Result<Vec<u8>, JsValue> {
    let options = if lossy {
        crate::EncodeOptions::lossy(quality).with_level(level)
    } else {
        crate::EncodeOptions::lossless().with_level(level)
    };

    crate::encode_from_audio(audio_bytes, options).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn decode_flo_to_wav(flo_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    crate::decode_to_wav(flo_bytes).map_err(|e| JsValue::from_str(&e.to_string()))
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn decode_flo_to_samples(flo_bytes: &[u8]) -> Result<JsValue, JsValue> {
    match crate::decode_to_samples(flo_bytes) {
        Ok((samples, sample_rate, channels)) => {
            // Create a JS object with the results
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(
                &obj,
                &JsValue::from_str("samples"),
                &js_sys::Float32Array::from(&samples[..]).into(),
            )?;
            js_sys::Reflect::set(
                &obj,
                &JsValue::from_str("sampleRate"),
                &JsValue::from_f64(sample_rate as f64),
            )?;
            js_sys::Reflect::set(
                &obj,
                &JsValue::from_str("channels"),
                &JsValue::from_f64(channels as f64),
            )?;
            Ok(obj.into())
        }
        Err(e) => Err(JsValue::from_str(&e.to_string())),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct FloInfo {
    version: String,
    sample_rate: u32,
    channels: u8,
    bit_depth: u8,
    duration_secs: f32,
    total_frames: u64,
    file_size: usize,
    compression_ratio: f32,
    crc_valid: bool,
    is_lossy: bool,
    lossy_quality: u8,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl FloInfo {
    #[wasm_bindgen(getter)]
    pub fn version(&self) -> String {
        self.version.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[wasm_bindgen(getter)]
    pub fn channels(&self) -> u8 {
        self.channels
    }

    #[wasm_bindgen(getter)]
    pub fn bit_depth(&self) -> u8 {
        self.bit_depth
    }

    #[wasm_bindgen(getter)]
    pub fn duration_secs(&self) -> f32 {
        self.duration_secs
    }

    #[wasm_bindgen(getter)]
    pub fn total_frames(&self) -> u64 {
        self.total_frames
    }

    #[wasm_bindgen(getter)]
    pub fn file_size(&self) -> usize {
        self.file_size
    }

    #[wasm_bindgen(getter)]
    pub fn compression_ratio(&self) -> f32 {
        self.compression_ratio
    }

    #[wasm_bindgen(getter)]
    pub fn crc_valid(&self) -> bool {
        self.crc_valid
    }

    #[wasm_bindgen(getter)]
    pub fn is_lossy(&self) -> bool {
        self.is_lossy
    }

    #[wasm_bindgen(getter)]
    pub fn lossy_quality(&self) -> u8 {
        self.lossy_quality
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_flo_file_info(flo_bytes: &[u8]) -> Result<FloInfo, JsValue> {
    let info = crate::get_flo_info(flo_bytes).map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(FloInfo {
        version: info.version,
        sample_rate: info.sample_rate,
        channels: info.channels,
        bit_depth: info.bit_depth,
        duration_secs: info.duration_secs as f32,
        total_frames: info.total_frames,
        file_size: info.file_size,
        compression_ratio: info.compression_ratio as f32,
        crc_valid: info.crc_valid,
        is_lossy: info.is_lossy,
        lossy_quality: info.lossy_quality,
    })
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_audio_file_info(audio_bytes: &[u8]) -> Result<JsValue, JsValue> {
    match crate::get_audio_info(audio_bytes) {
        Ok(info) => {
            let obj = js_sys::Object::new();
            js_sys::Reflect::set(
                &obj,
                &JsValue::from_str("sampleRate"),
                &JsValue::from_f64(info.sample_rate as f64),
            )?;
            js_sys::Reflect::set(
                &obj,
                &JsValue::from_str("channels"),
                &JsValue::from_f64(info.channels as f64),
            )?;
            js_sys::Reflect::set(
                &obj,
                &JsValue::from_str("durationSecs"),
                &JsValue::from_f64(info.duration_secs as f64),
            )?;
            Ok(obj.into())
        }
        Err(e) => Err(JsValue::from_str(&e.to_string())),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_flo_metadata_json(flo_bytes: &[u8]) -> Result<String, JsValue> {
    match crate::get_metadata(flo_bytes) {
        Ok(Some(meta)) => {
            serde_json::to_string(&meta).map_err(|e| JsValue::from_str(&e.to_string()))
        }
        Ok(None) => Ok("null".to_string()),
        Err(e) => Err(JsValue::from_str(&e.to_string())),
    }
}

/// Update metadata in a flo™ file WITHOUT re-encoding audio!
/// This is instant because flo™ stores metadata in a separate chunk.
///
/// # Arguments
/// * `flo_bytes` - Original flo™ file bytes
/// * `metadata` - JavaScript object with metadata fields
///
/// # Returns
/// New flo™ file bytes with updated metadata
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn update_flo_metadata(flo_bytes: &[u8], metadata: JsValue) -> Result<Vec<u8>, JsValue> {
    crate::update_metadata_no_reencode(flo_bytes, metadata)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Strip all metadata from a flo™ file WITHOUT re-encoding audio!
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn strip_flo_metadata(flo_bytes: &[u8]) -> Result<Vec<u8>, JsValue> {
    crate::strip_metadata_no_reencode(flo_bytes).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Check if a flo™ file has metadata
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn has_flo_metadata(flo_bytes: &[u8]) -> bool {
    crate::has_metadata(flo_bytes)
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn validate_flo_file(flo_bytes: &[u8]) -> Result<bool, JsValue> {
    crate::validate_flo(flo_bytes).map_err(|e| JsValue::from_str(&e.to_string()))
}

// Initialize wasm-bindgen panic hook for better error messages
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}
