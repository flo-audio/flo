mod rice_tests {
    use libflo_audio::core::rice::*;

    #[test]
    fn test_rice_roundtrip_float() {
        // Use values that quantize cleanly to 16-bit integers
        let residuals: Vec<f32> = vec![0.5, -0.5, 0.25, -0.25, 0.0, 0.125];
        let k = 10; // Appropriate for 16-bit values
        let encoded = encode(&residuals, k);
        let decoded = decode(&encoded, k, residuals.len());

        // With 16-bit quantization, expect ~1/32767 precision
        for (orig, dec) in residuals.iter().zip(decoded.iter()) {
            assert!((orig - dec).abs() < 0.001, "orig={}, dec={}", orig, dec);
        }
    }

    #[test]
    fn test_rice_roundtrip_i32() {
        let residuals: Vec<i32> = vec![100, -200, 50, -10, 0, 150, -300];
        let k = estimate_rice_parameter_i32(&residuals);
        let encoded = encode_i32(&residuals, k);
        let decoded = decode_i32(&encoded, k, residuals.len());

        assert_eq!(residuals, decoded);
    }

    #[test]
    fn test_zigzag() {
        // Test zigzag encoding/decoding
        let values = vec![0, 1, -1, 2, -2, 100, -100];
        for &v in &values {
            let unsigned = if v >= 0 {
                (v as u32) << 1
            } else {
                ((-v as u32) << 1) - 1
            };
            let back = if (unsigned & 1) == 0 {
                (unsigned >> 1) as i32
            } else {
                -(((unsigned >> 1) + 1) as i32)
            };
            assert_eq!(v, back);
        }
    }

    #[test]
    fn test_bit_writer_reader() {
        let mut writer = BitWriter::new();
        writer.write_bits(0b10110, 5);
        writer.write_bits(0b001, 3);
        let bytes = writer.into_bytes();

        let mut reader = BitReader::new(&bytes);
        assert_eq!(reader.read_bits(5), 0b10110);
        assert_eq!(reader.read_bits(3), 0b001);
    }
}
