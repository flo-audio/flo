#[cfg(test)]
mod psychoacoustic_tests {
    use libflo_audio::lossy::psychoacoustic::*;

    #[test]
    fn test_ath_curve() {
        let ath_1k = PsychoacousticModel::absolute_threshold_of_hearing(1000.0);
        let ath_100 = PsychoacousticModel::absolute_threshold_of_hearing(100.0);
        let ath_15k = PsychoacousticModel::absolute_threshold_of_hearing(15000.0);

        assert!(
            ath_1k < ath_100,
            "Should be more sensitive at 1kHz than 100Hz"
        );
        assert!(
            ath_1k < ath_15k,
            "Should be more sensitive at 1kHz than 15kHz"
        );
    }

    #[test]
    fn test_freq_to_bark() {
        let b_500 = PsychoacousticModel::freq_to_bark(500.0);
        let b_1000 = PsychoacousticModel::freq_to_bark(1000.0);
        let b_4000 = PsychoacousticModel::freq_to_bark(4000.0);

        assert!(b_500 < b_1000);
        assert!(b_1000 < b_4000);

        assert!((b_500 - 5.0).abs() < 1.0);
        assert!((b_1000 - 8.5).abs() < 1.0);
    }

    #[test]
    fn test_bark_bands() {
        let model = PsychoacousticModel::new(44100, 2048);

        assert_eq!(model.get_bark_band(0), 0);
        assert!(model.get_bark_band(model.num_coefficients() - 1) >= 20);
    }

    #[test]
    fn test_masking_threshold() {
        let mut model = PsychoacousticModel::new(44100, 2048);

        let mut coeffs = vec![0.0f32; 1024];
        let tone_bin = (1000.0 / model.frequency_resolution()).round() as usize;
        coeffs[tone_bin] = 1.0;

        let thresholds = model.calculate_masking_threshold(&coeffs);

        let near_tone = thresholds[tone_bin + 1];
        let far_from_tone = thresholds[tone_bin + 100];

        assert!(
            near_tone > far_from_tone,
            "Masking threshold should be higher near the masker"
        );
    }
}
