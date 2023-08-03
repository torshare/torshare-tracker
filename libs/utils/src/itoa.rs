pub trait Itoa {
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
