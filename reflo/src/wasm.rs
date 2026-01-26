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
        Ok(samples) => serde_wasm_bindgen::to_value(&samples)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
        Err(e) => Err(JsValue::from_str(&e.to_string())),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn get_flo_info(flo_bytes: &[u8]) -> Result<JsValue, JsValue> {
    match crate::get_flo_info(flo_bytes) {
        Ok(info) => serde_wasm_bindgen::to_value(&info)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
        Err(e) => Err(JsValue::from_str(&e.to_string())),
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn read_flo_metadata(flo_bytes: &[u8]) -> Result<JsValue, JsValue> {
    match crate::get_metadata(flo_bytes) {
        Ok(metadata) => serde_wasm_bindgen::to_value(&metadata)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
        Err(e) => Err(JsValue::from_str(&format!("{}", e))),
    }
}

/// Check if a flo™ file has metadata
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn has_flo_metadata(flo_bytes: &[u8]) -> bool {
    crate::has_metadata(flo_bytes)
}

/// Compute EBU R128 loudness metrics from audio samples
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn compute_loudness_metrics_reflo(
    samples: &[f32],
    channels: u8,
    sample_rate: u32,
) -> Result<JsValue, JsValue> {
    use libflo_audio::core::analysis::compute_ebu_r128_loudness;
    let metrics = compute_ebu_r128_loudness(samples, channels, sample_rate);
    serde_wasm_bindgen::to_value(&metrics)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Extract spectral fingerprint from audio samples
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn extract_spectral_fingerprint_reflo(
    samples: &[f32],
    channels: u8,
    sample_rate: u32,
    fft_size: Option<usize>,
    hop_size: Option<usize>,
) -> Result<JsValue, JsValue> {
    use libflo_audio::core::analysis::extract_spectral_fingerprint;
    let fingerprint =
        extract_spectral_fingerprint(samples, channels, sample_rate, fft_size, hop_size);
    serde_wasm_bindgen::to_value(&fingerprint)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Extract dominant frequencies from spectral fingerprint
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn extract_dominant_frequencies_reflo(
    fingerprint_js: JsValue,
    num_frequencies: usize,
) -> Result<JsValue, JsValue> {
    use libflo_audio::core::analysis::{extract_dominant_frequencies, SpectralFingerprint};

    let fingerprint: SpectralFingerprint = serde_wasm_bindgen::from_value(fingerprint_js)
        .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;

    let dominant_freqs = extract_dominant_frequencies(&fingerprint, num_frequencies);
    serde_wasm_bindgen::to_value(&dominant_freqs)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Extract waveform peaks from audio samples
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn extract_waveform_peaks_reflo(
    samples: &[f32],
    channels: u8,
    sample_rate: u32,
    peaks_per_second: u32,
) -> Result<JsValue, JsValue> {
    use libflo_audio::core::analysis::extract_waveform_peaks;
    let waveform = extract_waveform_peaks(samples, channels, sample_rate, peaks_per_second);
    serde_wasm_bindgen::to_value(&waveform)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
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
