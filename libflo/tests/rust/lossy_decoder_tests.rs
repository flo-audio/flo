#[cfg(test)]
mod decoder_tests {
    use libflo_audio::lossy::decoder::deserialize_sparse;
    use libflo_audio::lossy::encoder::serialize_sparse;

    #[test]
    fn test_sparse_roundtrip() {
        let coeffs = vec![0i16, 0, 0, 100, 0, 0, 0, 0, -50, 25, 0, 0];
        let encoded = serialize_sparse(&coeffs);
        let decoded = deserialize_sparse(&encoded, coeffs.len());

        assert_eq!(coeffs, decoded);
    }

    #[test]
    fn test_sparse_all_zeros() {
        let coeffs = vec![0i16; 1024];
        let encoded = serialize_sparse(&coeffs);
        let decoded = deserialize_sparse(&encoded, coeffs.len());

        assert_eq!(coeffs, decoded);
        // All zeros should compress very well
        assert!(encoded.len() < 20);
    }
}
