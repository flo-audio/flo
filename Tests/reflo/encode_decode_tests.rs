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

    #[test]
    fn test_decode_fixture_lossless_mono() {
        const FIXTURE: &[u8] = include_bytes!("../fixtures/sine_440hz_mono.flo");

        let (samples, sample_rate, channels) = decode_to_samples(FIXTURE).unwrap();

        assert_eq!(sample_rate, 44_100);
        assert_eq!(channels, 1);
        // 2 seconds of mono audio interleaved => 2 * 44100 samples.
        assert_eq!(samples.len(), 2 * 44_100);
    }

    #[test]
    fn test_decode_fixture_lossless_stereo() {
        const FIXTURE: &[u8] = include_bytes!("../fixtures/chord_cmajor_stereo.flo");

        let (samples, sample_rate, channels) = decode_to_samples(FIXTURE).unwrap();

        assert_eq!(sample_rate, 44_100);
        assert_eq!(channels, 2);
        // 2 seconds of stereo audio interleaved => 2 * 44100 * 2 samples.
        assert_eq!(samples.len(), 2 * 44_100 * 2);
    }

    #[test]
    fn test_decode_fixture_lossy() {
        const FIXTURE: &[u8] = include_bytes!("../fixtures/lossy_chord_high.flo");

        let (samples, sample_rate, channels) = decode_to_samples(FIXTURE).unwrap();

        assert_eq!(sample_rate, 44_100);
        assert_eq!(channels, 2);
        // Lossy encoding adds/pre-rolls MDCT frames; allow a tolerance.
        let expected = 2 * 44_100 * 2;
        assert!(
            samples.len().abs_diff(expected) <= 4096,
            "decoded {} samples, expected ~{expected}",
            samples.len()
        );
    }
}
