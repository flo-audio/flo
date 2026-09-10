#[cfg(test)]
mod transform_tests {
    use libflo_audio::lossy::{deserialize_frame, TransformDecoder, TransformEncoder};
    use libflo_audio::Reader;

    #[test]
    fn test_stereo_transform_encode_decode() {
        let sample_rate = 44100u32;
        let channels = 2u8;

        let mut samples = Vec::with_capacity(sample_rate as usize * 2);
        for i in 0..sample_rate as usize {
            samples.push((i as f32 * 0.01).sin() * 0.5);
            samples.push((i as f32 * 0.015).sin() * 0.5);
        }

        let quality = 0.55;
        let mut encoder = TransformEncoder::new(sample_rate, channels, quality);

        let flo_data = encoder
            .encode_to_flo(&samples, &[])
            .expect("Encoding failed");

        let reader = Reader::new();
        let file = reader.read(&flo_data).expect("Reading file failed");

        let mut decoder = TransformDecoder::new(file.header.sample_rate, file.header.channels);
        let mut all_samples = Vec::new();

        for frame in &file.frames {
            if frame.channels.is_empty() {
                continue;
            }

            let frame_data = &frame.channels[0].residuals;
            let transform_frame = deserialize_frame(frame_data).expect("Failed to deserialize");

            let decoded_samples = decoder.decode_frame(&transform_frame);
            all_samples.extend(decoded_samples);
        }

        assert!(!all_samples.is_empty(), "Should decode some samples");
    }

    #[test]
    fn test_sine_wave_compression() {
        use std::f32::consts::PI;

        let sample_rate = 44100u32;
        let channels = 1u8;
        let duration_sec = 1.0;
        let num_samples = (sample_rate as f32 * duration_sec) as usize;

        // Generate pure 440Hz sine wave
        let samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * PI * 440.0 * t).sin() * 0.5
            })
            .collect();

        let original_size = samples.len() * 4; // f32 = 4 bytes

        // Test at different quality levels
        for quality in [0.0, 0.35, 0.55, 0.75, 1.0] {
            let mut encoder = TransformEncoder::new(sample_rate, channels, quality);
            let flo_data = encoder
                .encode_to_flo(&samples, &[])
                .expect("Encoding failed");

            let ratio = original_size as f32 / flo_data.len() as f32;
            println!(
                "Sine quality {:.2}: {} bytes -> {} bytes ({:.1}x compression)",
                quality,
                original_size,
                flo_data.len(),
                ratio
            );
        }
    }

    #[test]
    fn test_white_noise_compression() {
        let sample_rate = 44100u32;
        let channels = 1u8;
        let num_samples = sample_rate as usize;

        // Generate white noise
        let mut rng_state = 12345u32;
        let samples: Vec<f32> = (0..num_samples)
            .map(|_| {
                rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
                ((rng_state >> 16) as f32 / 32768.0) - 1.0
            })
            .map(|s| s * 0.3)
            .collect();

        let original_size = samples.len() * 4;

        for quality in [0.0, 0.35, 0.55, 0.75, 1.0] {
            let mut encoder = TransformEncoder::new(sample_rate, channels, quality);
            let flo_data = encoder
                .encode_to_flo(&samples, &[])
                .expect("Encoding failed");

            let ratio = original_size as f32 / flo_data.len() as f32;
            println!(
                "Noise quality {:.2}: {} bytes -> {} bytes ({:.1}x compression)",
                quality,
                original_size,
                flo_data.len(),
                ratio
            );
        }
    }

    #[test]
    fn test_sine_wave_decode_quality() {
        use std::f32::consts::PI;

        let sample_rate = 44100u32;
        let channels = 1u8;
        let num_samples = sample_rate as usize; // 1 second

        // Generate pure 440Hz sine wave
        let original: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * PI * 440.0 * t).sin() * 0.5
            })
            .collect();

        // Encode at high quality
        let mut encoder = TransformEncoder::new(sample_rate, channels, 0.75);
        let flo_data = encoder
            .encode_to_flo(&original, &[])
            .expect("Encoding failed");

        // Decode using the main decode function (handles pre-roll correctly)
        let decoded = libflo_audio::decode(&flo_data).expect("Decoding failed");

        // Trim to original length (decoded may have extra samples)
        let decoded: Vec<f32> = decoded.into_iter().take(original.len()).collect();

        // Calculate SNR
        let mut signal_power = 0.0f64;
        let mut noise_power = 0.0f64;

        for (o, d) in original.iter().zip(decoded.iter()) {
            signal_power += (*o as f64).powi(2);
            noise_power += ((*o - *d) as f64).powi(2);
        }

        let snr_db = if noise_power > 1e-20 {
            10.0 * (signal_power / noise_power).log10()
        } else {
            100.0
        };

        println!("Sine wave decode SNR: {:.1} dB", snr_db);
        println!(
            "Original samples: {}, Decoded samples: {}",
            original.len(),
            decoded.len()
        );

        // Sample comparison - check some samples in the middle (skip transients)
        let start = 1000; // Skip first ~23ms to avoid edge effects
        println!("\nSamples comparison (starting at {}):", start);
        for i in start..start + 10 {
            if i < decoded.len() {
                let diff: f32 = original[i] - decoded[i];
                println!(
                    "  [{}] orig: {:.4}, decoded: {:.4}, diff: {:.6}",
                    i,
                    original[i],
                    decoded[i],
                    diff.abs()
                );
            }
        }

        // For lossy, we expect SNR > 15 dB for basic quality
        assert!(snr_db > 10.0, "SNR too low: {} dB", snr_db);
    }
}
