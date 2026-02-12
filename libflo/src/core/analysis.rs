//! Audio analysis functions for flo™ codec

use crate::core::metadata::WaveformData;
use rustfft::num_complex::Complex;
use rustfft::FftDirection;
use serde::{Deserialize, Serialize};

pub type FloSample = f32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralFingerprint {
    /// BLAKE3 hash of the audio content
    pub hash: [u8; 32],
    /// Duration in milliseconds
    pub duration_ms: u32,
    /// Sample rate
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u8,
    /// Peak frequency ranges (8 key frequency bands)
    pub frequency_peaks: [u8; 8],
    /// Energy distribution across frequency bands (16 bands)
    pub energy_profile: [u8; 16],
    /// Average loudness (LUFS scaled to u8)
    pub avg_loudness: u8,
}

/// Extract waveform peaks from audio samples
///
/// # Arguments
/// * `samples` - Audio samples (interleaved if stereo)
/// * `channels` - Number of audio channels (1 or 2)
/// * `sample_rate` - Sample rate in Hz
/// * `peaks_per_second` - Number of peak values per second
///
/// # Returns
/// `WaveformData` struct containing extracted peaks
pub fn extract_waveform_peaks(
    samples: &[FloSample],
    channels: u8,
    sample_rate: u32,
    peaks_per_second: u32,
) -> WaveformData {
    if samples.is_empty() {
        return WaveformData {
            peaks_per_second,
            peaks: Vec::new(),
            channels,
        };
    }

    let samples_per_peak = sample_rate as f64 / peaks_per_second as f64;
    let total_peaks = (samples.len() as f64 / (samples_per_peak * channels as f64)).ceil() as usize;

    let mut peaks = Vec::with_capacity(total_peaks);

    for peak_idx in 0..total_peaks {
        let start_sample = (peak_idx as f64 * samples_per_peak) as usize;
        let end_sample = ((peak_idx as f64 + 1.0) * samples_per_peak) as usize;

        let start_sample = start_sample * channels as usize;
        let end_sample = (end_sample * channels as usize).min(samples.len());

        if start_sample >= samples.len() {
            break;
        }

        let window_samples = &samples[start_sample..end_sample];

        match channels {
            1 => {
                // Mono: find peak in window
                let peak = window_samples
                    .iter()
                    .map(|&s| s.abs())
                    .fold(0.0f32, f32::max);
                peaks.push(peak);
            }
            2 => {
                // Stereo: find peaks for each channel
                let (left_peak, right_peak) = window_samples
                    .chunks_exact(2)
                    .map(|chunk| (chunk[0].abs(), chunk[1].abs()))
                    .fold((0.0f32, 0.0f32), |(l_max, r_max), (l, r)| {
                        (l_max.max(l), r_max.max(r))
                    });

                // Combine stereo peaks (average)
                peaks.push((left_peak + right_peak) / 2.0);
            }
            _ => {
                // Unsupported channel count: treat as mono by averaging
                let peak = window_samples
                    .chunks(channels as usize)
                    .map(|chunk| chunk.iter().copied().sum::<f32>() / chunk.len() as f32)
                    .fold(0.0f32, f32::max);
                peaks.push(peak);
            }
        }
    }

    // Normalize peaks to 0.0-1.0 range
    let max_peak = peaks.iter().fold(0.0f32, |max, &peak| max.max(peak));
    if max_peak > 0.0 {
        for peak in &mut peaks {
            *peak /= max_peak;
        }
    }

    WaveformData {
        peaks_per_second,
        peaks,
        channels,
    }
}

/// Extract waveform peaks using RMS method for smoother visualization
///
/// Similar to `extract_waveform_peaks` but uses RMS instead of peak values
/// for a smoother visual representation
pub fn extract_waveform_rms(
    samples: &[FloSample],
    channels: u8,
    sample_rate: u32,
    peaks_per_second: u32,
) -> WaveformData {
    if samples.is_empty() {
        return WaveformData {
            peaks_per_second,
            peaks: Vec::new(),
            channels,
        };
    }

    let samples_per_peak = sample_rate as f64 / peaks_per_second as f64;
    let total_peaks = (samples.len() as f64 / (samples_per_peak * channels as f64)).ceil() as usize;

    let mut peaks = Vec::with_capacity(total_peaks);

    for peak_idx in 0..total_peaks {
        let start_sample = (peak_idx as f64 * samples_per_peak) as usize;
        let end_sample = ((peak_idx as f64 + 1.0) * samples_per_peak) as usize;

        let start_sample = start_sample * channels as usize;
        let end_sample = (end_sample * channels as usize).min(samples.len());

        if start_sample >= samples.len() {
            break;
        }

        let window_samples = &samples[start_sample..end_sample];

        match channels {
            1 => {
                // Mono RMS
                let rms = (window_samples.iter().map(|&s| (s * s) as f64).sum::<f64>()
                    / window_samples.len() as f64)
                    .sqrt() as f32;
                peaks.push(rms);
            }
            2 => {
                // Stereo RMS
                let (left_sum, right_sum, count) = window_samples.chunks_exact(2).fold(
                    (0.0f64, 0.0f64, 0usize),
                    |(l_sum, r_sum, count), chunk| {
                        (
                            l_sum + (chunk[0] * chunk[0]) as f64,
                            r_sum + (chunk[1] * chunk[1]) as f64,
                            count + 1,
                        )
                    },
                );

                let count = count.max(1); // Avoid division by zero
                let left_rms = (left_sum / count as f64).sqrt() as f32;
                let right_rms = (right_sum / count as f64).sqrt() as f32;

                // Combine stereo RMS
                peaks.push((left_rms + right_rms) / 2.0);
            }
            _ => {
                // Unsupported channel count: treat as mono
                let rms = (window_samples
                    .chunks(channels as usize)
                    .map(|chunk| {
                        let avg = chunk.iter().copied().sum::<f32>() / chunk.len() as f32;
                        (avg * avg) as f64
                    })
                    .sum::<f64>()
                    / (window_samples.len() / channels as usize) as f64)
                    .sqrt() as f32;
                peaks.push(rms);
            }
        }
    }

    // Normalize RMS peaks to 0.0-1.0 range
    let max_peak = peaks.iter().fold(0.0f32, |max, &peak| max.max(peak));
    if max_peak > 0.0 {
        for peak in &mut peaks {
            *peak /= max_peak;
        }
    }

    WaveformData {
        peaks_per_second,
        peaks,
        channels,
    }
}

/// Extract spectral fingerprint from audio samples
///
/// # Arguments
/// * `samples` - Audio samples (interleaved if stereo)
/// * `channels` - Number of audio channels (1 or 2)
/// * `sample_rate` - Sample rate in Hz
/// * `fft_size` - FFT window size (must be power of 2, default: 2048)
/// * `hop_size` - Hop size between frames (default: fft_size/2 for 50% overlap)
///
/// # Returns
/// `SpectralFingerprint` struct containing spectral analysis
pub fn extract_spectral_fingerprint(
    samples: &[FloSample],
    channels: u8,
    sample_rate: u32,
    _fft_size: Option<usize>,
    _hop_size: Option<usize>,
) -> SpectralFingerprint {
    if samples.is_empty() {
        return SpectralFingerprint {
            hash: [0; 32],
            duration_ms: 0,
            sample_rate,
            channels,
            frequency_peaks: [0; 8],
            energy_profile: [0; 16],
            avg_loudness: 0,
        };
    }

    // Calculate duration - ensure at least 1ms for any non-zero samples
    let samples_per_channel = samples.len() / channels as usize;
    let duration_ms = ((samples_per_channel as f64 / sample_rate as f64 * 1000.0) as u32).max(1);

    // Create BLAKE3 hash of audio content + format info
    use blake3::Hasher;
    let mut hasher = Hasher::new();

    // Include format information in hash
    hasher.update(&channels.to_le_bytes());
    hasher.update(&sample_rate.to_le_bytes());
    hasher.update(&(samples.len() as u32).to_le_bytes());

    // Hash samples in chunks to avoid memory issues
    for chunk in samples.chunks(1024) {
        let chunk_bytes = unsafe {
            std::slice::from_raw_parts(chunk.as_ptr() as *const u8, std::mem::size_of_val(chunk))
        };
        hasher.update(chunk_bytes);
    }
    let hash = hasher.finalize().into();

    // Compact spectral analysis using small FFT
    let fft_size = 256; // Much smaller than before
    let mut planner = rustfft::FftPlanner::<f32>::new();
    let fft = planner.plan_fft(fft_size, FftDirection::Forward);
    let mut fft_buffer = vec![Complex { re: 0.0, im: 0.0 }; fft_size];

    // Take first and middle sections for analysis (quick sampling)
    let analysis_points = [
        samples_per_channel / 4,
        samples_per_channel / 2,
        samples_per_channel * 3 / 4,
    ];
    let mut frequency_bands = [0.0f32; 16];
    let mut peak_bands = [0u8; 8];

    for &sample_idx in &analysis_points {
        if sample_idx + fft_size < samples_per_channel {
            // Extract mono samples for this section
            for i in 0..fft_size {
                let mut sample = 0.0;
                for ch in 0..channels {
                    let idx = (sample_idx + i) * channels as usize + ch as usize;
                    if idx < samples.len() {
                        sample += samples[idx];
                    }
                }
                sample /= channels as f32;
                fft_buffer[i] = Complex {
                    re: sample,
                    im: 0.0,
                };
            }

            // Apply FFT
            fft.process(&mut fft_buffer);

            // Calculate energy in frequency bands (16 bands)
            for band in 0..16 {
                let start_bin = band * fft_size / 32;
                let end_bin = ((band + 1) * fft_size / 32).min(fft_size / 2);
                let mut energy = 0.0;
                for bin in start_bin..end_bin {
                    energy += fft_buffer[bin].re * fft_buffer[bin].re
                        + fft_buffer[bin].im * fft_buffer[bin].im;
                }
                frequency_bands[band] += energy.sqrt();
            }

            // Track peak frequencies (8 bands)
            for band in 0..8 {
                let start_bin = band * fft_size / 16;
                let end_bin = ((band + 1) * fft_size / 16).min(fft_size / 2);

                let (peak_bin, _) = (start_bin..end_bin)
                    .map(|bin| {
                        (
                            bin,
                            (fft_buffer[bin].re * fft_buffer[bin].re
                                + fft_buffer[bin].im * fft_buffer[bin].im)
                                .sqrt(),
                        )
                    })
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap_or((0, 0.0));

                // Convert to scaled u8 (log scale for better distribution)
                let peak_value = (peak_bin as f32 / fft_size as f32 * 255.0) as u8;
                peak_bands[band] = peak_bands[band].max(peak_value);
            }
        }
    }

    // Normalize frequency bands to u8
    let max_energy = frequency_bands.iter().cloned().fold(0.0f32, f32::max);
    let energy_profile = if max_energy > 0.0 {
        frequency_bands.map(|e| (e / max_energy * 255.0) as u8)
    } else {
        [0; 16]
    };

    // Compute average loudness (simplified RMS to LUFS conversion)
    let rms: f32 = samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32;
    let avg_loudness = ((-20.0 * (rms + 1e-10).log10()).clamp(-60.0, 0.0) + 60.0) as u8;

    SpectralFingerprint {
        hash,
        duration_ms,
        sample_rate,
        channels,
        frequency_peaks: peak_bands,
        energy_profile,
        avg_loudness,
    }
}

/// Extract dominant frequencies from spectral fingerprint
///
/// # Arguments
/// * `fingerprint` - Spectral fingerprint from `extract_spectral_fingerprint`
/// * `num_frequencies` - Number of dominant frequencies to extract per frame
///
/// # Returns
/// Vector of dominant frequencies (Hz) based on peak frequency bands
pub fn extract_dominant_frequencies(
    fingerprint: &SpectralFingerprint,
    num_frequencies: usize,
) -> Vec<Vec<f64>> {
    let num_frequencies = num_frequencies.min(8); // Max 8 bands available
    let mut dominant_freqs = Vec::with_capacity(1);
    let mut frame_dominants = Vec::with_capacity(num_frequencies);

    // Convert frequency peaks back to actual frequencies
    for i in 0..num_frequencies {
        // Map u8 back to frequency range (0-255 maps to 0Hz to Nyquist)
        let normalized_freq = fingerprint.frequency_peaks[i] as f64 / 255.0;
        let frequency = normalized_freq * (fingerprint.sample_rate as f64 / 2.0);
        frame_dominants.push(frequency);
    }

    dominant_freqs.push(frame_dominants);
    dominant_freqs
}

/// Compute spectral similarity between two fingerprints
///
/// # Arguments
/// * `fingerprint1` - First spectral fingerprint
/// * `fingerprint2` - Second spectral fingerprint
///
/// # Returns
/// Similarity score between 0.0 (completely different) and 1.0 (identical)
pub fn spectral_similarity(
    fingerprint1: &SpectralFingerprint,
    fingerprint2: &SpectralFingerprint,
) -> f32 {
    // If hashes match, it's the same content
    let hash_match = fingerprint1.hash == fingerprint2.hash;

    if hash_match {
        return 1.0;
    }

    // Basic format compatibility check
    if fingerprint1.sample_rate != fingerprint2.sample_rate
        || fingerprint1.channels != fingerprint2.channels
    {
        return 0.0;
    }

    // Compare energy profiles (16 bands)
    let energy_similarity: f32 = fingerprint1
        .energy_profile
        .iter()
        .zip(fingerprint2.energy_profile.iter())
        .map(|(a, b)| 1.0 - (*a as f32 - *b as f32).abs() / 255.0)
        .sum::<f32>()
        / 16.0;

    // Compare frequency peaks (8 bands)
    let peak_similarity: f32 = fingerprint1
        .frequency_peaks
        .iter()
        .zip(fingerprint2.frequency_peaks.iter())
        .map(|(a, b)| 1.0 - (*a as f32 - *b as f32).abs() / 255.0)
        .sum::<f32>()
        / 8.0;

    // Compare loudness
    let loudness_similarity =
        1.0 - (fingerprint1.avg_loudness as f32 - fingerprint2.avg_loudness as f32).abs() / 255.0;

    // Weighted average (energy is most important for similarity)
    energy_similarity * 0.5 + peak_similarity * 0.3 + loudness_similarity * 0.2
}
