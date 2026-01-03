#[cfg(test)]
mod quality_tests {
    use libflo::lossy::QualityPreset;

    #[test]
    fn test_quality_preset_roundtrip() {
        for preset in [
            QualityPreset::Low,
            QualityPreset::Medium,
            QualityPreset::High,
            QualityPreset::VeryHigh,
            QualityPreset::Transparent,
        ] {
            let v: u8 = preset.into();
            let back: QualityPreset = v.into();
            assert_eq!(preset, back);
        }
    }

    #[test]
    fn test_quality_values() {
        assert_eq!(QualityPreset::Low.as_f32(), 0.0);
        assert_eq!(QualityPreset::Transparent.as_f32(), 1.0);

        // Medium should be between low and transparent
        let medium = QualityPreset::Medium.as_f32();
        assert!(medium > 0.0 && medium < 1.0);
    }
}
