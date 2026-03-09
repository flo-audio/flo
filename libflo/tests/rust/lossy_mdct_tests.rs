#[cfg(test)]
mod mdct_tests {
    use libflo_audio::lossy::mdct::*;
    use std::f32::consts::PI;

    #[test]
    fn test_window_functions() {
        // Sine window should be symmetric
        let sine = Mdct::sine_window(256);
        assert_eq!(sine.len(), 256);
        for i in 0..128 {
            assert!((sine[i] - sine[255 - i]).abs() < 1e-6);
        }

        // Window values should be in [0, 1]
        for &w in &sine {
            assert!((0.0..=1.0).contains(&w));
        }

        // Vorbis window
        let vorbis = Mdct::vorbis_window(256);
        assert_eq!(vorbis.len(), 256);
        for &w in &vorbis {
            assert!((0.0..=1.0).contains(&w));
        }
    }

    #[test]
    fn test_mdct_inverse_basic() {
        let mdct = Mdct::new(1, WindowType::Sine);

        // Create a simple test signal
        let samples: Vec<f32> = (0..2048).map(|i| ((i as f32) * 0.01).sin()).collect();

        // Forward MDCT
        let coeffs = mdct.forward(&samples, BlockSize::Long);
        assert_eq!(coeffs.len(), 1024);

        // Inverse MDCT
        let reconstructed = mdct.inverse(&coeffs, BlockSize::Long);
        assert_eq!(reconstructed.len(), 2048);
    }

    #[test]
    fn test_mdct_perfect_reconstruction() {
        let n = 128; // Number of MDCT coefficients
        let n2 = 2 * n; // Window/block size (2N)

        // Sine window: w[k] = sin(π(k + 0.5) / 2N)
        let window: Vec<f32> = (0..n2)
            .map(|k| ((PI * ((k as f32) + 0.5)) / (n2 as f32)).sin())
            .collect();

        // Verify Princen-Bradley condition
        for i in 0..n {
            let sum = window[i].powi(2) + window[i + n].powi(2);
            assert!((sum - 1.0).abs() < 0.0001, "P-B violated at {}: {}", i, sum);
        }

        // Create test signal
        let freq = 440.0;
        let sample_rate = 44100.0;
        let total_samples = n2 * 4;
        let signal: Vec<f32> = (0..total_samples)
            .map(|i| ((2.0 * PI * freq * (i as f32)) / sample_rate).sin())
            .collect();

        // Forward MDCT
        let mdct_forward = |samples: &[f32]| -> Vec<f32> {
            let mut coeffs = vec![0.0f32; n];
            for (k, coeff) in coeffs.iter_mut().enumerate().take(n) {
                let mut sum = 0.0f32;
                for i in 0..n2 {
                    let angle = (PI / (n as f32))
                        * ((i as f32) + 0.5 + (n as f32) / 2.0)
                        * ((k as f32) + 0.5);
                    sum += samples[i] * window[i] * angle.cos();
                }
                *coeff = sum;
            }
            coeffs
        };

        // Inverse MDCT
        let mdct_inverse = |coeffs: &[f32]| -> Vec<f32> {
            let mut samples = vec![0.0f32; n2];
            let scale = 2.0 / (n as f32);
            for i in 0..n2 {
                let mut sum = 0.0f32;
                for (k, &coeff) in coeffs.iter().enumerate().take(n) {
                    let angle = (PI / (n as f32))
                        * ((i as f32) + 0.5 + (n as f32) / 2.0)
                        * ((k as f32) + 0.5);
                    sum += coeff * angle.cos();
                }
                samples[i] = sum * scale * window[i];
            }
            samples
        };

        // Process with overlap-add
        let mut output = vec![0.0f32; total_samples];

        for start in (0..=total_samples - n2).step_by(n) {
            let frame = &signal[start..start + n2];
            let coeffs = mdct_forward(frame);
            let reconstructed = mdct_inverse(&coeffs);

            for i in 0..n2 {
                if start + i < output.len() {
                    output[start + i] += reconstructed[i];
                }
            }
        }

        // Check middle region
        let check_start = n2;
        let check_end = total_samples - n2;

        let mut max_error = 0.0f32;
        for i in check_start..check_end {
            let err = (signal[i] - output[i]).abs();
            max_error = max_error.max(err);
        }

        assert!(
            max_error < 0.001,
            "Max reconstruction error: {} (expected < 0.001)",
            max_error
        );
    }

    #[test]
    fn test_short_blocks() {
        let mdct = Mdct::new(1, WindowType::Sine);

        let samples: Vec<f32> = (0..256).map(|i| ((i as f32) * 0.1).sin()).collect();

        let coeffs = mdct.forward(&samples, BlockSize::Short);
        assert_eq!(coeffs.len(), 128);

        let reconstructed = mdct.inverse(&coeffs, BlockSize::Short);
        assert_eq!(reconstructed.len(), 256);
    }

    #[test]
    fn test_multichannel_analyze_synthesize() {
        let mut mdct = Mdct::new(2, WindowType::Sine);

        let samples: Vec<f32> = (0..4096)
            .map(|i| {
                if i % 2 == 0 {
                    ((i as f32) * 0.01).sin()
                } else {
                    ((i as f32) * 0.02).cos()
                }
            })
            .collect();

        let coeffs = mdct.analyze(&samples, BlockSize::Long);
        assert_eq!(coeffs.len(), 2);
        assert_eq!(coeffs[0].len(), 1024);
        assert_eq!(coeffs[1].len(), 1024);
    }

    #[test]
    fn test_fft_speedup() {
        use std::time::Instant;

        let mdct = Mdct::new(1, WindowType::Sine);
        let input: Vec<f32> = (0..2048).map(|i| i as f32 / 2048.0).collect();

        // Time the FFT-based implementation
        let start = Instant::now();
        for _ in 0..100 {
            let _ = mdct.forward(&input, BlockSize::Long);
        }
        let fft_time = start.elapsed();

        // Should complete quickly with FFT - O(N log N) instead of O(N²)
        assert!(
            fft_time.as_millis() < 1000,
            "FFT MDCT should be fast: took {:?}",
            fft_time
        );
    }

    #[test]
    fn test_fft_perfect_reconstruction() {
        let mdct = Mdct::new(1, WindowType::Sine);

        // Create a test signal: two overlapping frames
        let input1: Vec<f32> = (0..2048)
            .map(|i| (2.0 * PI * i as f32 / 64.0).sin())
            .collect();
        let input2: Vec<f32> = (1024..3072)
            .map(|i| (2.0 * PI * i as f32 / 64.0).sin())
            .collect();

        // Forward MDCT
        let coeffs1 = mdct.forward(&input1, BlockSize::Long);
        let coeffs2 = mdct.forward(&input2, BlockSize::Long);

        // Inverse MDCT
        let r1 = mdct.inverse(&coeffs1, BlockSize::Long);
        let r2 = mdct.inverse(&coeffs2, BlockSize::Long);

        // Overlap-add: middle section should reconstruct original
        let mut reconstructed = vec![0.0f32; 1024];
        for i in 0..1024 {
            reconstructed[i] = r1[1024 + i] + r2[i];
        }

        // Compare with original middle section
        let original: Vec<f32> = (1024..2048)
            .map(|i| (2.0 * PI * i as f32 / 64.0).sin())
            .collect();

        let mut mse = 0.0;
        for i in 0..1024 {
            let diff = reconstructed[i] - original[i];
            mse += diff * diff;
        }
        mse /= 1024.0;

        assert!(
            mse < 1e-10,
            "FFT perfect reconstruction failed: MSE = {}",
            mse
        );
    }
}
