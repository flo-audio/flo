//! Audio analysis functions for flo™ codec

use crate::core::metadata::WaveformData;
use serde::{Deserialize, Serialize};
pub type FloSample = f32;
use rustfft::num_complex::Complex;
use rustfft::FftDirection;

/// EBU R128 loudness metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoudnessMetrics {
    /// Integrated loudness in LUFS (LKFS)
    pub integrated_lufs: f64,
    /// Loudness range in LU (LRA)
    pub loudness_range_lu: f64,
    /// True peak in dBTP
    pub true_peak_dbtp: f64,
    /// Sample peak in dBFS (for reference)
    pub sample_peak_dbfs: f64,
}

/// Compute EBU R128 loudness metrics from audio samples
///
/// # Arguments
/// * `samples` - Audio samples (interleaved if multi-channel)
/// * `channels` - Number of audio channels
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// `LoudnessMetrics` struct with EBU R128 measurements
pub fn compute_ebu_r128_loudness(
    samples: &[FloSample],
    channels: u8,
    sample_rate: u32,
) -> LoudnessMetrics {
    if samples.is_empty() {
        return LoudnessMetrics {
            integrated_lufs: -23.0,
            loudness_range_lu: 0.0,
            true_peak_dbtp: -150.0,
            sample_peak_dbfs: -150.0,
        };
    }

    // Constants per EBU R128 spec
    let gating_threshold = -70.0; // LUFS threshold for gating
    let _relative_threshold = -10.0; // LU below gated loudness
    let _min_ms_for_integration = 400; // Minimum duration for valid measurement
    let block_size = 0.4; // 400ms block size for loudness measurement

    // Calculate samples per block
    let samples_per_block = (sample_rate as f64 * block_size) as usize;

    // De-interleave samples by channel
    let samples_per_channel = samples.len() / channels as usize;
    let mut channel_samples: Vec<Vec<f32>> = Vec::with_capacity(channels as usize);
    for ch in 0..channels {
        let mut ch_data = Vec::with_capacity(samples_per_channel);
        for i in 0..samples_per_channel {
            let sample_idx = i * channels as usize + ch as usize;
            if sample_idx < samples.len() {
                ch_data.push(samples[sample_idx]);
            }
        }
        channel_samples.push(ch_data);
    }

    // Process each channel
    let mut channel_loudness = Vec::with_capacity(channels as usize);

    for ch_samples in &channel_samples {
        let mut block_loudness = Vec::new();

        // Process in blocks
        let mut pos = 0;
        while pos + samples_per_block <= ch_samples.len() {
            let block = &ch_samples[pos..pos + samples_per_block];

            // Compute mean square for block
            let mean_square: f64 =
                block.iter().map(|&x| x as f64 * x as f64).sum::<f64>() / block.len() as f64;

            // Convert to LUFS using EBU R128 weighting
            let loudness_lufs = if mean_square > 0.0 {
                -0.691 + 10.0 * (mean_square).log10()
            } else {
                -150.0 // Very low level
            };

            block_loudness.push(loudness_lufs);
            pos += samples_per_block;
        }

        // Find absolute peak for this channel
        let abs_peak = ch_samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

        let peak_dbfs = if abs_peak > 0.0 {
            20.0 * (abs_peak as f64).log10()
        } else {
            -150.0
        };

        channel_loudness.push((block_loudness, peak_dbfs));
    }

    // Gating: find blocks above threshold
    let mut gated_blocks = Vec::<f64>::new();
    for (ch_loudness, _) in &channel_loudness {
        for &block_lufs in ch_loudness {
            if block_lufs > gating_threshold {
                gated_blocks.push(block_lufs);
            }
        }
    }

    // Calculate integrated loudness
    let integrated_lufs = if gated_blocks.is_empty() {
        -23.0 // Default value
    } else {
        let gated_mean = gated_blocks.iter().sum::<f64>() / gated_blocks.len() as f64;
        gated_mean
    };

    // Calculate loudness range
    let loudness_range_lu = if gated_blocks.len() < 2 {
        0.0
    } else {
        let mut sorted_blocks = gated_blocks.clone();
        sorted_blocks.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let lower_percentile = sorted_blocks[(sorted_blocks.len() as f64 * 0.10) as usize];
        let upper_percentile = sorted_blocks[(sorted_blocks.len() as f64 * 0.95) as usize];
        upper_percentile - lower_percentile
    };

    // Find true peak across all channels
    let true_peak_abs = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

    let true_peak_dbtp = if true_peak_abs > 0.0 {
        20.0 * (true_peak_abs as f64).log10()
    } else {
        -150.0
    };

    LoudnessMetrics {
        integrated_lufs,
        loudness_range_lu,
        true_peak_dbtp,
        sample_peak_dbfs: channel_loudness
            .iter()
            .map(|(_, peak)| *peak)
            .fold(-150.0f64, f64::max),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpectralFingerprint {
    /// FFT window size used for analysis
    pub fft_size: usize,
    /// Number of frequency bins (half of FFT size for real signals)
    pub frequency_bins: usize,
    /// Frequency resolution in Hz per bin
    pub frequency_resolution: f64,
    /// Spectral data (frequency bins x time frames)
    /// For each frame: magnitudes of frequency components
    pub spectral_data: Vec<Vec<f32>>,
    /// Number of audio channels analyzed
    pub channels: u8,
    /// Sample rate of the original audio
    pub sample_rate: u32,
    /// Hop size between consecutive frames (in samples)
    pub hop_size: usize,
}

/// Extract waveform peaks from audio samples
///
/// # Arguments
/// * `samples` - Audio samples (interleaved if stereo)
/// * `channels` - Number of audio channels (1 or 2)
/// * `sample_rate` - Sample rate in Hz
/// * `peaks_per_second` - Desired number of peak values per second
///
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
    fft_size: Option<usize>,
    hop_size: Option<usize>,
) -> SpectralFingerprint {
    if samples.is_empty() {
        return SpectralFingerprint {
            fft_size: 0,
            frequency_bins: 0,
            frequency_resolution: 0.0,
            spectral_data: Vec::new(),
            channels,
            sample_rate,
            hop_size: 0,
        };
    }

    let fft_size = fft_size.unwrap_or(2048);
    let hop_size = hop_size.unwrap_or(fft_size / 2);

    // Ensure FFT size is power of 2
    let fft_size = fft_size.next_power_of_two();
    let frequency_bins = fft_size / 2 + 1; // Only positive frequencies for real signals
    let frequency_resolution = sample_rate as f64 / fft_size as f64;

    // Initialize FFT planner and buffer
    let mut planner = rustfft::FftPlanner::<f32>::new();
    let fft = planner.plan_fft(fft_size, FftDirection::Forward);

    // Pre-allocate buffer for complex samples
    let mut fft_buffer = vec![Complex { re: 0.0, im: 0.0 }; fft_size];

    // Create Hann window for better spectral analysis
    let mut window = vec![0.0; fft_size];
    for i in 0..fft_size {
        window[i] =
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (fft_size - 1) as f32).cos());
    }

    let samples_per_channel = samples.len() / channels as usize;
    let num_frames = if samples_per_channel >= fft_size {
        (samples_per_channel - fft_size) / hop_size + 1
    } else {
        1 // At least one frame if we have any samples
    };

    let mut spectral_data = Vec::with_capacity(num_frames);

    // Process each channel separately
    match channels {
        1 => {
            // Mono processing
            for frame_idx in 0..num_frames {
                let start_sample = frame_idx * hop_size;
                let end_sample = (start_sample + fft_size).min(samples_per_channel);

                // Clear buffer and apply windowing
                fft_buffer.fill(Complex { re: 0.0, im: 0.0 });
                for i in 0..(end_sample - start_sample) {
                    fft_buffer[i] = Complex {
                        re: samples[start_sample + i] * window[i],
                        im: 0.0,
                    };
                }

                // Apply FFT
                fft.process(&mut fft_buffer);

                // Convert to magnitude spectrum (only positive frequencies)
                let mut spectrum = Vec::with_capacity(frequency_bins);
                for i in 0..frequency_bins {
                    let magnitude = (fft_buffer[i].re * fft_buffer[i].re
                        + fft_buffer[i].im * fft_buffer[i].im)
                        .sqrt();
                    spectrum.push(magnitude);
                }
                spectral_data.push(spectrum);
            }
        }
        2 => {
            // Stereo processing - analyze left channel primarily
            for frame_idx in 0..num_frames {
                let start_sample = frame_idx * hop_size;
                let end_sample = (start_sample + fft_size).min(samples_per_channel);

                fft_buffer.fill(Complex { re: 0.0, im: 0.0 });

                for i in 0..(end_sample - start_sample) {
                    let sample_idx = (start_sample + i) * 2; // Left channel index
                    if sample_idx < samples.len() {
                        fft_buffer[i] = Complex {
                            re: samples[sample_idx] * window[i],
                            im: 0.0,
                        };
                    }
                }

                // Apply FFT
                fft.process(&mut fft_buffer);

                // Convert to magnitude spectrum
                let mut spectrum = Vec::with_capacity(frequency_bins);
                for i in 0..frequency_bins {
                    let magnitude = (fft_buffer[i].re * fft_buffer[i].re
                        + fft_buffer[i].im * fft_buffer[i].im)
                        .sqrt();
                    spectrum.push(magnitude);
                }
                spectral_data.push(spectrum);
            }
        }
        _ => {
            // Multi-channel: mix down to mono
            for frame_idx in 0..num_frames {
                let start_sample = frame_idx * hop_size;
                let end_sample = (start_sample + fft_size).min(samples_per_channel);

                fft_buffer.fill(Complex { re: 0.0, im: 0.0 });

                for i in 0..(end_sample - start_sample) {
                    // Mix down all channels
                    let mut mixed_sample = 0.0;
                    for ch in 0..channels {
                        let sample_idx = (start_sample + i) * channels as usize + ch as usize;
                        if sample_idx < samples.len() {
                            mixed_sample += samples[sample_idx];
                        }
                    }
                    mixed_sample /= channels as f32;
                    fft_buffer[i] = Complex {
                        re: mixed_sample * window[i],
                        im: 0.0,
                    };
                }

                // Apply FFT
                fft.process(&mut fft_buffer);

                // Convert to magnitude spectrum
                let mut spectrum = Vec::with_capacity(frequency_bins);
                for i in 0..frequency_bins {
                    let magnitude = (fft_buffer[i].re * fft_buffer[i].re
                        + fft_buffer[i].im * fft_buffer[i].im)
                        .sqrt();
                    spectrum.push(magnitude);
                }
                spectral_data.push(spectrum);
            }
        }
    }

    SpectralFingerprint {
        fft_size,
        frequency_bins,
        frequency_resolution,
        spectral_data,
        channels,
        sample_rate,
        hop_size,
    }
}

/// Extract dominant frequencies from spectral fingerprint
///
/// # Arguments
/// * `fingerprint` - Spectral fingerprint from `extract_spectral_fingerprint`
/// * `num_frequencies` - Number of dominant frequencies to extract per frame
///
/// # Returns
/// Vector of vectors containing dominant frequencies (Hz) for each frame
pub fn extract_dominant_frequencies(
    fingerprint: &SpectralFingerprint,
    num_frequencies: usize,
) -> Vec<Vec<f64>> {
    let mut dominant_freqs = Vec::with_capacity(fingerprint.spectral_data.len());

    for frame_spectrum in &fingerprint.spectral_data {
        let mut freq_magnitude_pairs: Vec<(usize, f32)> = frame_spectrum
            .iter()
            .enumerate()
            .map(|(bin_idx, &magnitude)| (bin_idx, magnitude))
            .collect();

        // Sort by magnitude (descending)
        freq_magnitude_pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Extract top frequencies
        let frame_dominants: Vec<f64> = freq_magnitude_pairs
            .iter()
            .take(num_frequencies)
            .map(|(bin_idx, _)| *bin_idx as f64 * fingerprint.frequency_resolution)
            .collect();

        dominant_freqs.push(frame_dominants);
    }

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
    if fingerprint1.frequency_bins != fingerprint2.frequency_bins {
        return 0.0; // Incompatible fingerprints
    }

    let min_frames = fingerprint1
        .spectral_data
        .len()
        .min(fingerprint2.spectral_data.len());
    if min_frames == 0 {
        return 0.0;
    }

    let mut total_similarity = 0.0;

    for i in 0..min_frames {
        let spectrum1 = &fingerprint1.spectral_data[i];
        let spectrum2 = &fingerprint2.spectral_data[i];

        // Compute cosine similarity
        let dot_product: f32 = spectrum1
            .iter()
            .zip(spectrum2.iter())
            .map(|(a, b)| a * b)
            .sum();
        let norm1: f32 = spectrum1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = spectrum2.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm1 > 0.0 && norm2 > 0.0 {
            total_similarity += dot_product / (norm1 * norm2);
        }
    }

    total_similarity / min_frames as f32
}
