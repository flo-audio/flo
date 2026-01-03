mod crc32_tests {
    use libflo::core::crc32::compute;

    #[test]
    fn test_crc32_empty() {
        assert_eq!(compute(&[]), 0x00000000);
    }

    #[test]
    fn test_crc32_known() {
        // "123456789" should produce 0xCBF43926
        let data = b"123456789";
        assert_eq!(compute(data), 0xCBF43926);
    }
}
