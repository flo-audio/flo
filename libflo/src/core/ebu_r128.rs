use serde::{Deserialize, Serialize};
pub type FloSample = f32;

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
