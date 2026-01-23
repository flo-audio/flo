//! Audio analysis functions for flo™ codec

use crate::core::metadata::WaveformData;
use serde::{Serialize, Deserialize};
pub type FloSample = f32;
use rustfft::num_complex::Complex;
use rustfft::FftDirection;

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
                let rms = (window_samples
                    .iter()
                    .map(|&s| (s * s) as f64)
                    .sum::<f64>() / window_samples.len() as f64)
                    .sqrt() as f32;
                peaks.push(rms);
            }
            2 => {
                // Stereo RMS
                let (left_sum, right_sum, count) = window_samples
                    .chunks_exact(2)
                    .fold((0.0f64, 0.0f64, 0usize), |(l_sum, r_sum, count), chunk| {
                        (l_sum + (chunk[0] * chunk[0]) as f64,
                         r_sum + (chunk[1] * chunk[1]) as f64,
                         count + 1)
                    });
                
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
                    .sum::<f64>() / (window_samples.len() / channels as usize) as f64)
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
        window[i] = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (fft_size - 1) as f32).cos());
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
                        im: 0.0 
                    };
                }

                // Apply FFT
                fft.process(&mut fft_buffer);

                // Convert to magnitude spectrum (only positive frequencies)
                let mut spectrum = Vec::with_capacity(frequency_bins);
                for i in 0..frequency_bins {
                    let magnitude = (fft_buffer[i].re * fft_buffer[i].re + fft_buffer[i].im * fft_buffer[i].im).sqrt();
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
                            im: 0.0 
                        };
                    }
                }

                // Apply FFT
                fft.process(&mut fft_buffer);

                // Convert to magnitude spectrum
                let mut spectrum = Vec::with_capacity(frequency_bins);
                for i in 0..frequency_bins {
                    let magnitude = (fft_buffer[i].re * fft_buffer[i].re + fft_buffer[i].im * fft_buffer[i].im).sqrt();
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
                        im: 0.0 
                    };
                }

                // Apply FFT
                fft.process(&mut fft_buffer);

                // Convert to magnitude spectrum
                let mut spectrum = Vec::with_capacity(frequency_bins);
                for i in 0..frequency_bins {
                    let magnitude = (fft_buffer[i].re * fft_buffer[i].re + fft_buffer[i].im * fft_buffer[i].im).sqrt();
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
pub fn spectral_similarity(fingerprint1: &SpectralFingerprint, fingerprint2: &SpectralFingerprint) -> f32 {
    if fingerprint1.frequency_bins != fingerprint2.frequency_bins {
        return 0.0; // Incompatible fingerprints
    }

    let min_frames = fingerprint1.spectral_data.len().min(fingerprint2.spectral_data.len());
    if min_frames == 0 {
        return 0.0;
    }

    let mut total_similarity = 0.0;

    for i in 0..min_frames {
        let spectrum1 = &fingerprint1.spectral_data[i];
        let spectrum2 = &fingerprint2.spectral_data[i];

        // Compute cosine similarity
        let dot_product: f32 = spectrum1.iter().zip(spectrum2.iter()).map(|(a, b)| a * b).sum();
        let norm1: f32 = spectrum1.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm2: f32 = spectrum2.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm1 > 0.0 && norm2 > 0.0 {
            total_similarity += dot_product / (norm1 * norm2);
        }
    }

    total_similarity / min_frames as f32
}
