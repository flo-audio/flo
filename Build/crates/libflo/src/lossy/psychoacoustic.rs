/// Number of critical bands (Bark scale, 0-24 Bark for audio up to ~20kHz)
pub const NUM_BARK_BANDS: usize = 25;

/// Critical band edges in Hz (Bark scale)
pub const BARK_BAND_EDGES: [f32; 26] = [
    0.0, 100.0, 200.0, 300.0, 400.0, 510.0, 630.0, 770.0, 920.0, 1080.0, 1270.0, 1480.0, 1720.0,
    2000.0, 2320.0, 2700.0, 3150.0, 3700.0, 4400.0, 5300.0, 6400.0, 7700.0, 9500.0, 12000.0,
    15500.0, 20500.0,
];

/// Psychoacoustic model parameters
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PsychoacousticModel {
    /// Sample rate
    pub(crate) sample_rate: u32,
    /// FFT/MDCT size (2N for MDCT)
    pub(crate) fft_size: usize,
    /// Number of MDCT coefficients (N)
    pub(crate) num_coeffs: usize,
    /// Frequency resolution (Hz per bin)
    pub(crate) freq_resolution: f32,
    /// Absolute threshold of hearing per MDCT bin (dB SPL)
    pub(crate) ath: Vec<f32>,
    /// Bark band index for each MDCT coefficient
    pub(crate) bark_band: Vec<usize>,
    /// Spreading function matrix (for masking)
    pub(crate) spreading: Vec<Vec<f32>>,
    /// Previous frame's energy (for temporal masking)
    pub(crate) prev_energy: Vec<f32>,
}

impl PsychoacousticModel {
    /// Create a new psychoacoustic model
    pub fn new(sample_rate: u32, fft_size: usize) -> Self {
        let num_coeffs = fft_size / 2;
        let freq_resolution = sample_rate as f32 / fft_size as f32;

        // Calculate ATH for each bin
        let ath: Vec<f32> = (0..num_coeffs)
            .map(|k| {
                let freq = (k as f32 + 0.5) * freq_resolution;
                Self::absolute_threshold_of_hearing(freq)
            })
            .collect();

        // Map each coefficient to its Bark band
        let bark_band: Vec<usize> = (0..num_coeffs)
            .map(|k| {
                let freq = (k as f32 + 0.5) * freq_resolution;
                Self::freq_to_bark_band(freq)
            })
            .collect();

        // Pre-compute spreading function
        let spreading = Self::compute_spreading_function();

        // Initialize previous energy
        let prev_energy = vec![0.0f32; NUM_BARK_BANDS];

        Self {
            sample_rate,
            fft_size,
            num_coeffs,
            freq_resolution,
            ath,
            bark_band,
            spreading,
            prev_energy,
        }
    }

    /// Get the Bark band for a specific coefficient index
    pub fn get_bark_band(&self, coeff_idx: usize) -> usize {
        self.bark_band.get(coeff_idx).copied().unwrap_or(0)
    }

    /// Get number of MDCT coefficients
    pub fn num_coefficients(&self) -> usize {
        self.num_coeffs
    }

    /// Get frequency resolution (Hz per bin)
    pub fn frequency_resolution(&self) -> f32 {
        self.freq_resolution
    }

    /// Absolute Threshold of Hearing (ATH) in dB SPL
    /// Based on ISO 226 / Terhardt formula
    pub fn absolute_threshold_of_hearing(freq: f32) -> f32 {
        if !(20.0..=20000.0).contains(&freq) {
            return 96.0; // Essentially inaudible
        }

        let f_khz = freq / 1000.0;

        // Terhardt's formula (simplified)
        // ATH(f) = 3.64 * (f/1000)^-0.8 - 6.5 * exp(-0.6 * (f/1000 - 3.3)^2) + 10^-3 * (f/1000)^4
        let term1 = 3.64 * f_khz.powf(-0.8);
        let term2 = 6.5 * (-0.6 * (f_khz - 3.3).powi(2)).exp();
        let term3 = 0.001 * f_khz.powi(4);

        (term1 - term2 + term3).clamp(-10.0, 96.0)
    }

    /// Convert frequency to Bark scale
    pub fn freq_to_bark(freq: f32) -> f32 {
        // Traunmüller's formula
        let bark = ((26.81 * freq) / (1960.0 + freq)) - 0.53;
        bark.clamp(0.0, 24.0)
    }

    /// Get the Bark band index for a frequency
    pub fn freq_to_bark_band(freq: f32) -> usize {
        for (i, &edge) in BARK_BAND_EDGES.iter().enumerate().skip(1) {
            if freq < edge {
                return i - 1;
            }
        }
        NUM_BARK_BANDS - 1
    }

    /// Compute the spreading function between Bark bands
    /// This models how a masker in one band affects neighboring bands
    fn compute_spreading_function() -> Vec<Vec<f32>> {
        let mut spreading = vec![vec![0.0f32; NUM_BARK_BANDS]; NUM_BARK_BANDS];

        for i in 0..NUM_BARK_BANDS {
            for j in 0..NUM_BARK_BANDS {
                let delta_bark = j as f32 - i as f32;

                // Spreading function (simplified from MPEG psychoacoustic model)
                let spread = if delta_bark >= 0.0 {
                    // Upper slope (masking above the masker)
                    -25.0 * delta_bark
                } else {
                    // Lower slope (masking below the masker)
                    -10.0 * delta_bark
                };

                // Convert dB to linear and clamp
                spreading[i][j] = (10.0f32.powf(spread / 10.0)).min(1.0);
            }
        }

        spreading
    }

    /// Calculate the masking threshold for MDCT coefficients
    /// Returns threshold in dB for each coefficient
    pub fn calculate_masking_threshold(&mut self, coeffs: &[f32]) -> Vec<f32> {
        let mut thresholds = vec![0.0f32; self.num_coeffs];

        // Step 1: Calculate energy per Bark band
        let mut band_energy = [0.0f32; NUM_BARK_BANDS];
        let mut band_count = [0usize; NUM_BARK_BANDS];

        for (k, &coeff) in coeffs.iter().enumerate() {
            let band = self.bark_band[k];
            let energy = coeff * coeff;
            band_energy[band] += energy;
            band_count[band] += 1;
        }

        // Convert to dB (average energy per band)
        let band_db: Vec<f32> = band_energy
            .iter()
            .zip(band_count.iter())
            .map(|(&e, &c)| {
                if c > 0 && e > 1e-10 {
                    10.0 * (e / c as f32).log10()
                } else {
                    -100.0
                }
            })
            .collect();

        // Step 2: Apply spreading function (simultaneous masking)
        let mut spread_threshold = vec![-100.0f32; NUM_BARK_BANDS];

        for i in 0..NUM_BARK_BANDS {
            for j in 0..NUM_BARK_BANDS {
                // Masking from band j to band i
                let masking = band_db[j] + 10.0 * self.spreading[j][i].log10();
                spread_threshold[i] = spread_threshold[i].max(masking);
            }
        }

        // Step 3: Apply masking offset (tone masking noise vs noise masking tone)
        // Simplified: use a single offset (real codecs distinguish tone/noise)
        let masking_offset = -6.0; // dB below masker
        for t in &mut spread_threshold {
            *t += masking_offset;
        }

        // Step 4: Temporal masking (post-masking)
        // Previous frame's energy can still mask current frame
        let temporal_decay = 0.7; // Decay factor per frame
        for i in 0..NUM_BARK_BANDS {
            let temporal_mask = self.prev_energy[i] * temporal_decay;
            spread_threshold[i] = spread_threshold[i].max(temporal_mask);
            self.prev_energy[i] = spread_threshold[i];
        }

        // Step 5: Combine with ATH and map back to coefficients
        for (k, threshold) in thresholds.iter_mut().enumerate() {
            let band = self.bark_band[k];
            // Final threshold is max of masking threshold and ATH
            // Subtract some headroom for safety
            *threshold = spread_threshold[band].max(self.ath[k]) - 10.0;
        }

        thresholds
    }

    /// Calculate Signal-to-Mask Ratio (SMR) for each coefficient
    /// Higher SMR = more important, needs more bits
    pub fn calculate_smr(&mut self, coeffs: &[f32]) -> Vec<f32> {
        let thresholds = self.calculate_masking_threshold(coeffs);

        coeffs
            .iter()
            .zip(thresholds.iter())
            .map(|(&c, &t)| {
                let signal_db = if c.abs() > 1e-10 {
                    20.0 * c.abs().log10()
                } else {
                    -100.0
                };
                // SMR = signal level - masking threshold
                // Positive = audible, negative = masked
                signal_db - t
            })
            .collect()
    }

    /// Calculate bits needed per band based on SMR
    /// Higher SMR needs more bits to avoid audible quantization noise
    pub fn allocate_bits(&mut self, coeffs: &[f32], total_bits: usize) -> Vec<u8> {
        let smr = self.calculate_smr(coeffs);

        // Calculate bits per band based on SMR
        let mut band_smr = [0.0f32; NUM_BARK_BANDS];
        let mut band_count = [0usize; NUM_BARK_BANDS];

        for (k, &s) in smr.iter().enumerate() {
            let band = self.bark_band[k];
            band_smr[band] = band_smr[band].max(s);
            band_count[band] += 1;
        }

        // Allocate bits proportionally to positive SMR
        let total_smr: f32 = band_smr.iter().map(|&s| s.max(0.0)).sum();

        let mut bits_per_coeff = vec![0u8; self.num_coeffs];

        if total_smr > 0.0 {
            for (k, bits) in bits_per_coeff.iter_mut().enumerate() {
                let band = self.bark_band[k];
                let band_bits = if band_smr[band] > 0.0 {
                    ((band_smr[band] / total_smr) * total_bits as f32 / band_count[band] as f32)
                        .round() as u8
                } else {
                    0
                };
                *bits = band_bits.clamp(0, 15);
            }
        }

        bits_per_coeff
    }

    /// Get the frequency for a given MDCT coefficient index
    pub fn bin_to_freq(&self, bin: usize) -> f32 {
        (bin as f32 + 0.5) * self.freq_resolution
    }

    /// Reset temporal state (for seeking/discontinuities)
    pub fn reset(&mut self) {
        self.prev_energy.fill(0.0);
    }
}
