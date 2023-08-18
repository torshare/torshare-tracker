/// The `Itoa` trait represents a trait that defines a method `itoa` which converts
/// a value to its ASCII representation as a vector of bytes (`Vec<u8>`).
/// The name "Itoa" is an abbreviation for "integer to ASCII".
pub trait Itoa {
    /// Converts the value to its ASCII representation as a vector of bytes.
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` containing the ASCII representation of the value.
    fn itoa(&self) -> Vec<u8>;
}

macro_rules! itoa_impl_for_integer {
    ($int_type:ty) => {
        impl Itoa for $int_type {
            fn itoa(&self) -> Vec<u8> {
                let mut buffer = itoa::Buffer::new();
                buffer.format(*self).into()
            }
        }
    };
}

itoa_impl_for_integer!(i64);
itoa_impl_for_integer!(i32);
itoa_impl_for_integer!(i16);
itoa_impl_for_integer!(i8);
itoa_impl_for_integer!(u64);
itoa_impl_for_integer!(u32);
itoa_impl_for_integer!(u16);
itoa_impl_for_integer!(u8);
itoa_impl_for_integer!(usize);
itoa_impl_for_integer!(isize);

#[cfg(test)]
mod tests {
    use super::*;

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
}
