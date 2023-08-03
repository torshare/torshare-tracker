use ts_utils::hex::{decode, encode};

#[test]
fn test_encode() {
    let input: &[u8] = &[72, 101, 108, 108, 111]; // "Hello" in ASCII
    let expected_output = "48656c6c6f"; // The hexadecimal representation of "Hello"
    assert_eq!(encode(input), expected_output);
}

#[test]
fn test_decode() {
    let input = "48656c6c6f"; // Hexadecimal representation of "Hello"
    let expected_output: Vec<u8> = vec![72, 101, 108, 108, 111]; // "Hello" in ASCII
    assert_eq!(decode(input).unwrap(), expected_output);
}

#[test]
fn test_decode_error() {
    // Test decoding with an invalid hexadecimal string
    let invalid_input = "4865&6c6c6f";
    assert!(decode(invalid_input).is_err());
}
