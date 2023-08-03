use ts_utils::itoa::Itoa;

macro_rules! itoa_test {
    ($test_name:ident, $int_type:ty, $value:expr, $expected:expr) => {
        #[test]
        fn $test_name() {
            let value: $int_type = $value;
            assert_eq!(value.itoa(), $expected.to_vec());
        }
    };
}

itoa_test!(test_itoa_i64, i64, 12345, b"12345");
itoa_test!(test_itoa_i16, i16, -42, b"-42");
itoa_test!(test_itoa_i8, i8, 127, b"127");
itoa_test!(test_itoa_i32, i32, -987654321, b"-987654321");
itoa_test!(test_itoa_isize, isize, -1234567890, b"-1234567890");
itoa_test!(test_itoa_u64, u64, 999999999999, b"999999999999");
itoa_test!(test_itoa_u16, u16, 65535, b"65535");
itoa_test!(test_itoa_u8, u8, 255, b"255");
itoa_test!(test_itoa_u32, u32, 987654321, b"987654321");
itoa_test!(test_itoa_usize, usize, 1234567890, b"1234567890");
