/// Tries to convert the first 4 bytes of a byte slice to a little-endian u32 value.
///
/// # Arguments
///
/// * `bytes`: A slice of bytes to convert.
///
/// # Returns
///
/// * `Some(u32_value)`: If the input slice contains at least 4 bytes, it returns the
/// little-endian u32 value formed by the first 4 bytes of the slice.
/// * `None`: If the input slice has fewer than 4 bytes, it returns None.
pub fn try_convert_bytes_to_u32(bytes: &[u8]) -> Option<u32> {
    if bytes.len() >= 4 {
        Some(u32::from_le_bytes(bytes[0..4].try_into().unwrap()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_bytes_to_u32() {
        // Test with valid 4-byte input
        let bytes_4 = [0x12, 0x34, 0x56, 0x78];
        assert_eq!(try_convert_bytes_to_u32(&bytes_4), Some(0x78563412));

        // Test with valid 8-byte input (only the first 4 bytes will be considered)
        let bytes_8 = [0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD, 0xEF, 0x00];
        assert_eq!(try_convert_bytes_to_u32(&bytes_8), Some(0x78563412));

        // Test with valid 0-byte input (empty slice)
        let bytes_empty: [u8; 0] = [];
        assert_eq!(try_convert_bytes_to_u32(&bytes_empty), None);
    }
}
