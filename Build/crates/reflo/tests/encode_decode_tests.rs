#[cfg(test)]
mod tests {
    use reflo::audio::AudioMetadata;
    use reflo::{decode_to_samples, encode_from_samples, EncodeOptions};

    #[test]
    fn test_encode_decode_round_trip() {
        // Create test samples
        let sample_rate = 44100;
        let channels = 2;
        let duration = 1.0; // 1 second
        let num_samples = (sample_rate as f32 * duration * channels as f32) as usize;

        // Generate sine wave
        let mut samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples / channels {
            let t = i as f32 / sample_rate as f32;
            let sample = (t * 440.0 * 2.0 * std::f32::consts::PI).sin() * 0.5;
            for _ in 0..channels {
                samples.push(sample);
            }
        }

        // Encode
        let flo_bytes = encode_from_samples(
            &samples,
            sample_rate,
            channels,
            AudioMetadata::default(),
            EncodeOptions::lossless(),
        )
        .unwrap();

        // Decode
        let (decoded_samples, decoded_sr, decoded_ch) = decode_to_samples(&flo_bytes).unwrap();

        assert_eq!(decoded_sr, sample_rate);
        assert_eq!(decoded_ch, channels);
        assert_eq!(decoded_samples.len(), samples.len());

        // Check samples are close (allowing for compression artifacts)
        for (original, decoded) in samples.iter().zip(decoded_samples.iter()) {
            assert!((original - decoded).abs() < 0.01);
        }
    }
}
