use bytes::{BufMut, Bytes, BytesMut};
use serde::ser;
use serde::Serialize;
use std::collections::BTreeMap;
use std::error::Error as StdError;
use std::fmt;
use std::io::Error as IoError;
use std::result::Result as StdResult;

use crate::itoa::Itoa;

const TOKEN_END: u8 = b'e';
const TOKEN_DICT: u8 = b'd';
const TOKEN_LIST: u8 = b'l';
const TOKEN_INT: u8 = b'i';
const TOKEN_LEN: u8 = b':';

#[derive(Default, Debug)]
/// Bencode Serializer.
pub struct Serializer {
    pub output: BytesMut,
    pub is_sorted: bool,
}

impl Serializer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            output: BytesMut::new(),
            is_sorted: false,
        }
    }

    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            output: BytesMut::with_capacity(capacity),
            is_sorted: false,
        }
    }

    fn push_slice<T: AsRef<[u8]>>(&mut self, token: T) {
        self.output.extend_from_slice(token.as_ref());
    }

    fn push(&mut self, token: u8) {
        self.output.put_u8(token);
    }

    pub fn start_dict(&mut self) {
        self.push(TOKEN_DICT);
    }

    pub fn end_dict(&mut self) {
        self.push(TOKEN_END);
    }

    pub fn start_list(&mut self) {
        self.push(TOKEN_LIST);
    }

    pub fn end_list(&mut self) {
        self.push(TOKEN_END);
    }

    pub fn encode_bytes(&mut self, v: &[u8]) {
        self.push_slice(v.len().itoa());
        self.push(TOKEN_LEN);
        self.push_slice(v);
    }

    pub fn encode_int<T: Itoa>(&mut self, val: T) {
        self.push(TOKEN_INT);
        self.push_slice(val.itoa());
        self.push(TOKEN_END);
    }

    pub fn finalize(self) -> Bytes {
        self.output.freeze()
    }
}

impl<'a> serde::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = MapSerializer<'a>;
    type SerializeStruct = MapSerializer<'a>;
    type SerializeStructVariant = MapSerializer<'a>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        let val = if v { 1 } else { 0 };
        self.serialize_u8(val)
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, value: i64) -> Result<Self::Ok> {
        self.encode_int(value);
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, value: u64) -> Result<Self::Ok> {
        self.encode_int(value);
        Ok(())
    }

    fn serialize_f32(self, _value: f32) -> Result<Self::Ok> {
        Err(Error::Custom("Cannot serialize f32".to_string()))
    }

    fn serialize_f64(self, _value: f64) -> Result<Self::Ok> {
        Err(Error::Custom("Cannot serialize f64".to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        self.encode_bytes(v);
        Ok(())
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        variant.serialize(&mut *self)
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.push(TOKEN_LIST);
        Ok(self)
    }

    fn serialize_tuple(self, size: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(size))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.push(TOKEN_DICT);
        variant.serialize(&mut *self)?;
        self.push(TOKEN_LIST);
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapSerializer::new(self, self.is_sorted))
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.push(TOKEN_DICT);
        variant.serialize(&mut *self)?;
        Ok(MapSerializer::new(self, self.is_sorted))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        self.push(TOKEN_DICT);
        variant.serialize(&mut *self)?;
        value.serialize(&mut *self)?;
        self.push(TOKEN_END);
        Ok(())
    }
}

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<Self::Ok> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.push(TOKEN_END);
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<Self::Ok> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<Self::Ok> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<Self::Ok> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.push(TOKEN_END);
        ser::SerializeSeq::end(self)
    }
}

/// Bencode sub-serializer for maps.
pub enum MapSerializer<'a> {
    Sorted(SortedMapSerializer<'a>),
    UnSorted(UnSortedMapSerializer<'a>),
}

impl<'a> MapSerializer<'a> {
    pub fn new(ser: &'a mut Serializer, is_sorted: bool) -> Self {
        if is_sorted {
            MapSerializer::Sorted(SortedMapSerializer::new(ser))
        } else {
            MapSerializer::UnSorted(UnSortedMapSerializer::new(ser))
        }
    }
}

impl<'a> ser::SerializeMap for MapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        match self {
            MapSerializer::Sorted(s) => s.serialize_key(key),
            MapSerializer::UnSorted(u) => u.serialize_key(key),
        }
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        match self {
            MapSerializer::Sorted(s) => s.serialize_value(value),
            MapSerializer::UnSorted(u) => u.serialize_value(value),
        }
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(&mut self, key: &K, value: &V) -> Result<Self::Ok>
    where
        K: Serialize,
        V: Serialize,
    {
        match self {
            MapSerializer::Sorted(s) => s.serialize_entry(key, value),
            MapSerializer::UnSorted(u) => u.serialize_entry(key, value),
        }
    }

    fn end(self) -> Result<Self::Ok> {
        match self {
            MapSerializer::Sorted(s) => s.end(),
            MapSerializer::UnSorted(u) => u.end(),
        }
    }
}

impl<'a> ser::SerializeStruct for MapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeMap::end(self)
    }
}

impl<'a> ser::SerializeStructVariant for MapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        match self {
            MapSerializer::Sorted(mut s) => {
                s.close()?;
                s.ser.push(TOKEN_END);
            }
            MapSerializer::UnSorted(mut u) => {
                u.flush()?;
                u.ser.push(TOKEN_END);
            }
        }

        Ok(())
    }
}

/// Bencode sub-serializer for sorted maps.
pub struct SortedMapSerializer<'a> {
    /// Bencode Serializer.
    ser: &'a mut Serializer,
    /// Current key.
    cur_key: Option<Bytes>,
}

impl<'a> SortedMapSerializer<'a> {
    fn new(ser: &'a mut Serializer) -> SortedMapSerializer {
        ser.push(TOKEN_DICT);
        SortedMapSerializer { ser, cur_key: None }
    }

    fn serialize<T>(&self, value: &T) -> Result<Bytes>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = Serializer::new();
        value.serialize(&mut serializer)?;
        Ok(serializer.finalize())
    }

    fn push_entry(&mut self, key: Bytes, value: Bytes) {
        self.ser.push_slice(key);
        self.ser.push_slice(value);
    }

    fn close(&mut self) -> Result<()> {
        self.ser.push(TOKEN_END);
        Ok(())
    }
}

impl<'a> ser::SerializeMap for SortedMapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        if self.cur_key.is_some() {
            return Err(Error::Custom(
                "`serialize_key` called multiple times without calling  `serialize_value`"
                    .to_string(),
            ));
        }

        self.cur_key = Some(self.serialize(key)?);
        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        match self.cur_key.take() {
            Some(key) => {
                let value = self.serialize(value)?;
                if !value.is_empty() {
                    self.push_entry(key, value);
                }

                Ok(())
            }
            None => Err(Error::MapSerializationCallOrder),
        }
    }

    fn serialize_entry<K: ?Sized, V: ?Sized>(&mut self, key: &K, value: &V) -> Result<Self::Ok>
    where
        K: Serialize,
        V: Serialize,
    {
        if self.cur_key.is_some() {
            return Err(Error::Custom(
                "`serialize_entry` called instead of `serialize_value`".to_string(),
            ));
        }

        let value = self.serialize(value)?;
        if !value.is_empty() {
            key.serialize(&mut *self.ser)?;
            self.ser.push_slice(value);
        }

        Ok(())
    }

    fn end(mut self) -> Result<Self::Ok> {
        self.close()
    }
}

impl<'a> ser::SerializeStruct for SortedMapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeMap::end(self)
    }
}

/// Bencode sub-serializer for unsorted maps.
pub struct UnSortedMapSerializer<'a> {
    /// Bencode Serializer.
    ser: &'a mut Serializer,
    /// Bencode Dictionary.
    dict: BTreeMap<Bytes, Bytes>,
    /// Current key.
    cur_key: Option<Bytes>,
}

impl<'a> UnSortedMapSerializer<'a> {
    fn new(ser: &'a mut Serializer) -> UnSortedMapSerializer {
        UnSortedMapSerializer {
            ser,
            dict: BTreeMap::new(),
            cur_key: None,
        }
    }

    fn flush(&mut self) -> Result<()> {
        self.ser.push(TOKEN_DICT);

        for (k, v) in &self.dict {
            self.ser.encode_bytes(k);
            self.ser.push_slice(v);
        }

        self.ser.push(TOKEN_END);
        Ok(())
    }

    fn serialize<T>(&self, value: &T) -> Result<Bytes>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = Serializer::new();
        value.serialize(&mut serializer)?;
        Ok(serializer.finalize())
    }
}

impl<'a> ser::SerializeMap for UnSortedMapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        if self.cur_key.is_some() {
            return Err(Error::Custom(
                "`serialize_key` called multiple times without calling  `serialize_value`"
                    .to_string(),
            ));
        }

        let mut encoded = self.serialize(key)?;
        match encoded.first() {
            Some(b'0'..=b'9') => {}
            _ => return Err(Error::ArbitraryMapKeysUnsupported),
        }

        let colon = encoded.iter().position(|b| *b == TOKEN_LEN).unwrap();
        let encoded = encoded.split_off(colon + 1);

        self.cur_key = Some(encoded);

        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        match self.cur_key.take() {
            Some(key) => {
                let value = self.serialize(value)?;
                if !value.is_empty() {
                    self.dict.insert(key, value);
                }

                Ok(())
            }
            None => Err(Error::MapSerializationCallOrder),
        }
    }

    fn end(mut self) -> Result<Self::Ok> {
        self.flush()
    }
}

impl<'a> ser::SerializeStruct for UnSortedMapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeMap::end(self)
    }
}

/// Alias for `Result<T, bencode::Error>`.
pub type Result<T> = StdResult<T, Error>;

/// Represents all possible errors which can occur when serializing or deserializing bencode.
#[derive(Debug)]
pub enum Error {
    /// Raised when an IO error occurred.
    IoError(IoError),

    /// Error that occurs if a map with a key type which does not serialize to
    /// a byte string is encountered
    ArbitraryMapKeysUnsupported,

    /// Error that occurs if methods on MapSerializer are called out of order
    MapSerializationCallOrder,

    /// Catchall for any other kind of error.
    Custom(String),
}

impl ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Custom(msg.to_string())
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match *self {
            Error::IoError(ref error) => Some(error),
            _ => None,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let message = match *self {
            Error::IoError(ref error) => return error.fmt(f),
            Error::Custom(ref s) => s,
            Error::ArbitraryMapKeysUnsupported => {
                "Maps with key types that do not serialize to byte strings are unsupported"
            }
            Error::MapSerializationCallOrder => "Map serialization methods called out of order",
        };
        f.write_str(message)
    }
}

/// Serializes an object into a bencode `Bytes`.
pub fn encode<T: ser::Serialize>(b: &T) -> Result<Bytes> {
    _encode(b, Serializer::new())
}

/// Serializes an object into a bencode `Bytes` with a given capacity.
pub fn encode_with_capacity<T: ser::Serialize>(b: &T, capacity: usize) -> Result<Bytes> {
    _encode(b, Serializer::with_capacity(capacity))
}

fn _encode<T: ser::Serialize>(b: &T, mut ser: Serializer) -> Result<Bytes> {
    b.serialize(&mut ser)?;
    Ok(ser.finalize())
}

/// A trait for types that can be serialized into the Bencode format.
/// Implementors of this trait must also implement the standard `Serialize` trait
/// provided by the `serde` crate.
pub trait Bencode: Serialize {
    /// Determines whether the data needs to be sorted before serialization.
    /// By default, this method returns `true`.
    fn requires_sort(&self) -> bool {
        true
    }

    /// Estimates the capacity needed for serializing the data.
    fn capacity(&self) -> usize {
        0
    }

    /// Serializes the implementor into Bencode format.
    ///
    /// Returns:
    /// A `Result` containing the serialized data as `Bytes`, or an error if
    /// serialization fails.
    fn bencode(&self) -> Result<Bytes> {
        let mut serializer = Serializer::with_capacity(self.capacity() * 2);
        serializer.is_sorted = self.requires_sort() == false;
        self.serialize(&mut serializer)?;
        Ok(serializer.finalize())
    }
}

#[macro_export]
macro_rules! bencode_int {
    ($serializer:expr, $value:expr) => {
        $serializer.encode_int($value);
    };
}

#[macro_export]
macro_rules! bencode_str {
    ($serializer:expr, $value:expr) => {
        $serializer.encode_bytes($value.as_bytes());
    };
}

#[macro_export]
macro_rules! bencode_dict {
    ($serializer:expr, $($key:expr => $value:expr),*) => {
        $serializer.start_dict();
        $(
            $serializer.encode_bytes($key.as_bytes());
            $value;
        )*
        $serializer.end_dict();
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default, PartialEq, Serialize)]
    struct Response {
        complete: i64,
        incomplete: i64,
        interval: i64,
        #[serde(rename = "min interval")]
        min_interval: i64,
        peers: Option<Vec<u8>>,
        peers6: Option<Vec<u8>>,
    }

    #[test]
    fn test_encode() {
        let expected_output = "d8:completei0e10:incompletei0e8:intervali0e12:min intervali0ee";
        let response = Response::default();
        let encoded = encode(&response).unwrap();
        assert_eq!(encoded, expected_output);
    }

    #[test]
    fn test_encode_with_capacity() {
        let expected_output = "d8:completei0e10:incompletei0e8:intervali0e12:min intervali0ee";
        let response = Response::default();
        let encoded = encode_with_capacity(&response, 100).unwrap();
        assert_eq!(encoded, expected_output);
    }
}
