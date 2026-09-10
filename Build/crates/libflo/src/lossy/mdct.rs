// Full disclosure, this code is inspired by Symphonia's MDCT implementation,
// and part's of ffmpeg's as well.

use rustfft::{num_complex::Complex, FftPlanner};
use std::f32::consts::PI;
use std::sync::Arc;

/// Window types for MDCT
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowType {
    /// Sine window - simple, good for most content
    Sine,
    /// Kaiser-Bessel Derived - better frequency selectivity
    KaiserBesselDerived,
    /// Vorbis window - optimized for audio
    Vorbis,
}

/// MDCT block sizes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockSize {
    /// Long block (2048 samples) - good frequency resolution for stationary signals
    Long,
    /// Short block (256 samples) - good time resolution for transients
    Short,
    /// Start block - transition from long to short
    Start,
    /// Stop block - transition from short to long
    Stop,
}

impl BlockSize {
    /// Get the number of samples for this block size
    pub fn samples(self) -> usize {
        match self {
            BlockSize::Long | BlockSize::Start | BlockSize::Stop => 2048,
            BlockSize::Short => 256,
        }
    }

    /// Get the number of MDCT coefficients (N/2)
    pub fn coefficients(self) -> usize {
        self.samples() / 2
    }
}

/// FFT-based MDCT transform for a specific window size
struct MdctTransform {
    /// Window size (N)
    n: usize,
    /// Number of coefficients (N/2)
    n2: usize,
    /// FFT size (N/4)
    n4: usize,
    /// Window function
    window: Vec<f32>,
    /// Forward FFT
    fft: Arc<dyn rustfft::Fft<f32>>,
    /// Twiddle factors: e^(i*π/n2 * (k + 1/8))
    twiddle: Vec<Complex<f32>>,
}

impl MdctTransform {
    fn new(window_size: usize, window_type: WindowType) -> Self {
        let n = window_size;
        let n2 = n / 2;
        let n4 = n / 4;

        // Create window
        let window = match window_type {
            WindowType::Sine => Self::sine_window(n),
            WindowType::KaiserBesselDerived => Self::kbd_window(n, 4.0),
            WindowType::Vorbis => Self::vorbis_window(n),
        };

        // Create FFT planner
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(n4);

        // Pre-compute twiddle factors
        let twiddle: Vec<Complex<f32>> = (0..n4)
            .map(|k| {
                let theta = PI / n2 as f32 * (k as f32 + 0.125);
                Complex::new(theta.cos(), theta.sin())
            })
            .collect();

        Self {
            n,
            n2,
            n4,
            window,
            fft,
            twiddle,
        }
    }

    /// Sine window: w[n] = sin(π(n+0.5)/N)
    fn sine_window(n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (PI * (i as f32 + 0.5) / n as f32).sin())
            .collect()
    }

    /// Vorbis window: sin(π/2 * sin²(π(n+0.5)/N))
    fn vorbis_window(n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| {
                let x = (PI * (i as f32 + 0.5) / n as f32).sin();
                (PI / 2.0 * x * x).sin()
            })
            .collect()
    }

    /// Kaiser-Bessel Derived window
    fn kbd_window(n: usize, alpha: f32) -> Vec<f32> {
        let half = n / 2;

        // Compute Kaiser window for first half
        let kaiser: Vec<f32> = (0..=half)
            .map(|i| {
                Self::bessel_i0(
                    PI * alpha * (1.0 - (2.0 * i as f32 / half as f32 - 1.0).powi(2)).sqrt(),
                )
            })
            .collect();

        // Cumulative sum
        let mut cumsum = vec![0.0f32; half + 1];
        cumsum[0] = kaiser[0];
        for i in 1..=half {
            cumsum[i] = cumsum[i - 1] + kaiser[i];
        }
        let total = cumsum[half];

        // Build KBD window
        let mut window = vec![0.0f32; n];
        for i in 0..half {
            window[i] = (cumsum[i] / total).sqrt();
            window[n - 1 - i] = window[i];
        }

        window
    }

    /// Modified Bessel function I0 (for KBD window)
    fn bessel_i0(x: f32) -> f32 {
        let mut sum = 1.0f32;
        let mut term = 1.0f32;
        let x_sq = x * x / 4.0;

        for k in 1..20 {
            term *= x_sq / (k * k) as f32;
            sum += term;
            if term < 1e-10 {
                break;
            }
        }

        sum
    }

    /// Forward MDCT using FFT - O(N log N)
    ///
    /// Based on FFmpeg's ff_mdct_calc_c algorithm.
    fn forward(&self, samples: &[f32]) -> Vec<f32> {
        let n = self.n;
        let n2 = self.n2;
        let n4 = self.n4;
        let n8 = n4 / 2;
        let n3 = 3 * n4;

        // Apply window
        let x: Vec<f32> = samples
            .iter()
            .zip(self.window.iter())
            .map(|(&s, &w)| s * w)
            .collect();

        // Pre-rotation: fold N windowed samples into N/4 complex FFT inputs
        let mut z: Vec<Complex<f32>> = vec![Complex::new(0.0, 0.0); n4];

        for i in 0..n8 {
            // First butterfly
            let re = -x[2 * i + n3] - x[n3 - 1 - 2 * i];
            let im = -x[n4 + 2 * i] + x[n4 - 1 - 2 * i];

            let w = &self.twiddle[i];
            z[i] = Complex::new(-re * w.re - im * w.im, re * w.im - im * w.re);

            // Second butterfly
            let re2 = x[2 * i] - x[n2 - 1 - 2 * i];
            let im2 = -x[n2 + 2 * i] - x[n - 1 - 2 * i];

            let w2 = &self.twiddle[n8 + i];
            z[n8 + i] = Complex::new(-re2 * w2.re - im2 * w2.im, re2 * w2.im - im2 * w2.re);
        }

        // Forward FFT
        self.fft.process(&mut z);

        // Post-rotation: extract N/2 real coefficients
        let mut output = vec![0.0; n2];

        for i in 0..n8 {
            let idx1 = n8 - i - 1;
            let idx2 = n8 + i;

            let w1 = &self.twiddle[idx1];
            let z1 = z[idx1];
            let i1 = -z1.re * w1.im + z1.im * w1.re;
            let r0 = -z1.re * w1.re - z1.im * w1.im;

            let w2 = &self.twiddle[idx2];
            let z2 = z[idx2];
            let i0 = -z2.re * w2.im + z2.im * w2.re;
            let r1 = -z2.re * w2.re - z2.im * w2.im;

            output[2 * idx1] = r0;
            output[2 * idx1 + 1] = i0;
            output[2 * idx2] = r1;
            output[2 * idx2 + 1] = i1;
        }

        output
    }

    /// Inverse MDCT using FFT - O(N log N)
    ///
    /// Based on Symphonia's IMDCT implementation.
    fn inverse(&self, spec: &[f32]) -> Vec<f32> {
        let n = self.n;
        let n2 = self.n2;
        let n4 = self.n4;
        let n8 = n4 / 2;

        // Pre-FFT twiddling
        let mut z: Vec<Complex<f32>> = Vec::with_capacity(n4);

        for i in 0..n4 {
            let even = spec[i * 2];
            let odd = -spec[n2 - 1 - i * 2];

            let w = &self.twiddle[i];
            z.push(Complex::new(
                odd * w.im - even * w.re,
                odd * w.re + even * w.im,
            ));
        }

        // Apply forward FFT
        self.fft.process(&mut z);

        // Post-FFT twiddling and unfolding
        let mut output = vec![0.0; n];
        let scale = 2.0 / n2 as f32;

        // First half of FFT output
        for i in 0..n8 {
            let w = &self.twiddle[i];
            let val_re = w.re * z[i].re + w.im * z[i].im;
            let val_im = w.im * z[i].re - w.re * z[i].im;

            let fi = 2 * i;
            let ri = n4 - 1 - 2 * i;

            output[ri] = -val_im * scale * self.window[ri];
            output[n4 + fi] = val_im * scale * self.window[n4 + fi];
            output[n2 + ri] = val_re * scale * self.window[n2 + ri];
            output[n2 + n4 + fi] = val_re * scale * self.window[n2 + n4 + fi];
        }

        // Second half of FFT output
        for i in 0..n8 {
            let idx = n8 + i;
            let w = &self.twiddle[idx];
            let val_re = w.re * z[idx].re + w.im * z[idx].im;
            let val_im = w.im * z[idx].re - w.re * z[idx].im;

            let fi = 2 * i;
            let ri = n4 - 1 - 2 * i;

            output[fi] = -val_re * scale * self.window[fi];
            output[n4 + ri] = val_re * scale * self.window[n4 + ri];
            output[n2 + fi] = val_im * scale * self.window[n2 + fi];
            output[n2 + n4 + ri] = val_im * scale * self.window[n2 + n4 + ri];
        }

        output
    }
}

/// MDCT processor with pre-computed windows and FFT plans
///
/// Provides O(N log N) MDCT/IMDCT transforms using FFT acceleration.
pub struct Mdct {
    /// Long block transform (2048 samples)
    long_transform: MdctTransform,
    /// Short block transform (256 samples)
    short_transform: MdctTransform,
    /// Previous frame's windowed samples for overlap-add (per channel)
    overlap_buffer: Vec<Vec<f32>>,
    /// Number of channels
    channels: usize,
}

impl Mdct {
    /// Create a new MDCT processor
    pub fn new(channels: usize, window_type: WindowType) -> Self {
        let long_transform = MdctTransform::new(2048, window_type);
        let short_transform = MdctTransform::new(256, window_type);

        // Initialize overlap buffers (N/2 samples per channel for long blocks)
        let overlap_buffer = vec![vec![0.0f32; 1024]; channels];

        Self {
            long_transform,
            short_transform,
            overlap_buffer,
            channels,
        }
    }

    /// Sine window: w[n] = sin(π(n+0.5)/N)
    pub fn sine_window(n: usize) -> Vec<f32> {
        MdctTransform::sine_window(n)
    }

    /// Vorbis window: sin(π/2 * sin²(π(n+0.5)/N))
    pub fn vorbis_window(n: usize) -> Vec<f32> {
        MdctTransform::vorbis_window(n)
    }

    /// Forward MDCT: N time samples → N/2 frequency coefficients
    ///
    /// X[k] = Σ x[n] * w[n] * cos(π/N * (n + 0.5 + N/2) * (k + 0.5))
    pub fn forward(&self, samples: &[f32], block_size: BlockSize) -> Vec<f32> {
        let n = block_size.samples();
        assert!(samples.len() >= n, "Not enough samples for MDCT");

        let transform = match block_size {
            BlockSize::Long | BlockSize::Start | BlockSize::Stop => &self.long_transform,
            BlockSize::Short => &self.short_transform,
        };

        transform.forward(&samples[..n])
    }

    /// Inverse MDCT: N/2 frequency coefficients → N time samples
    ///
    /// y[n] = 2/N * Σ(k=0 to N-1) X[k] * cos(π/N * (n + 0.5 + N/2) * (k + 0.5))
    pub fn inverse(&self, coeffs: &[f32], block_size: BlockSize) -> Vec<f32> {
        let n2 = block_size.coefficients();
        assert!(coeffs.len() >= n2, "Not enough coefficients for IMDCT");

        let transform = match block_size {
            BlockSize::Long | BlockSize::Start | BlockSize::Stop => &self.long_transform,
            BlockSize::Short => &self.short_transform,
        };

        transform.inverse(&coeffs[..n2])
    }

    /// Process a frame with overlap-add for perfect reconstruction
    /// Returns N/2 output samples (the middle half after overlap-add)
    pub fn process_frame(
        &mut self,
        samples: &[f32],
        channel: usize,
        block_size: BlockSize,
    ) -> (Vec<f32>, Vec<f32>) {
        let n = block_size.samples();
        let n2 = n / 2;

        // Forward MDCT
        let coeffs = self.forward(samples, block_size);

        // Inverse MDCT (for testing/verification)
        let reconstructed = self.inverse(&coeffs, block_size);

        // Overlap-add with previous frame
        let mut output = vec![0.0f32; n2];
        for i in 0..n2 {
            output[i] = reconstructed[i] + self.overlap_buffer[channel][i];
        }

        // Store second half for next frame's overlap
        self.overlap_buffer[channel].copy_from_slice(&reconstructed[n2..n2 + n2]);

        (coeffs, output)
    }

    /// Reset overlap buffers (e.g., for seeking)
    pub fn reset(&mut self) {
        for buf in &mut self.overlap_buffer {
            buf.fill(0.0);
        }
    }

    /// Encode samples to MDCT coefficients for all channels
    /// Input: interleaved samples [L, R, L, R, ...]
    /// Output: MDCT coefficients per channel
    pub fn analyze(&mut self, samples: &[f32], block_size: BlockSize) -> Vec<Vec<f32>> {
        let n = block_size.samples();
        let samples_per_channel = samples.len() / self.channels;

        // Deinterleave
        let mut channel_data: Vec<Vec<f32>> = (0..self.channels)
            .map(|_| Vec::with_capacity(samples_per_channel))
            .collect();

        for (i, &s) in samples.iter().enumerate() {
            channel_data[i % self.channels].push(s);
        }

        // MDCT each channel
        let mut all_coeffs = Vec::with_capacity(self.channels);
        for data in &channel_data {
            if data.len() >= n {
                let coeffs = self.forward(data, block_size);
                all_coeffs.push(coeffs);
            } else {
                // Pad with zeros if not enough samples
                let mut padded = data.clone();
                padded.resize(n, 0.0);
                let coeffs = self.forward(&padded, block_size);
                all_coeffs.push(coeffs);
            }
        }

        all_coeffs
    }

    /// Synthesize samples from MDCT coefficients with overlap-add
    /// Input: MDCT coefficients per channel
    /// Output: interleaved samples
    pub fn synthesize(&mut self, coeffs: &[Vec<f32>], block_size: BlockSize) -> Vec<f32> {
        let n = block_size.samples();
        let n2 = n / 2;

        // IMDCT + overlap-add for each channel
        let mut channel_outputs: Vec<Vec<f32>> = Vec::with_capacity(self.channels);

        for (ch, ch_coeffs) in coeffs.iter().enumerate() {
            let reconstructed = self.inverse(ch_coeffs, block_size);

            // Overlap-add
            let mut output = vec![0.0f32; n2];
            for i in 0..n2 {
                output[i] = reconstructed[i] + self.overlap_buffer[ch][i];
            }

            // Store for next frame
            self.overlap_buffer[ch].copy_from_slice(&reconstructed[n2..n2 + n2]);

            channel_outputs.push(output);
        }

        // Interleave
        let mut output = Vec::with_capacity(n2 * self.channels);
        for i in 0..n2 {
            for ch in 0..self.channels {
                output.push(channel_outputs[ch][i]);
            }
        }

        output
    }
}
