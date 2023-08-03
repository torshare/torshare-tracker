use serde::ser;
use serde::Serialize;
use serde::Serializer;
use std::collections::BTreeMap;
use std::error::Error as StdError;
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
pub struct BencodeSerializer {
    output: Vec<u8>,
}

impl BencodeSerializer {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            output: Vec::with_capacity(capacity),
        }
    }

    #[inline]
    fn push_slice<T: AsRef<[u8]>>(&mut self, token: T) {
        self.output.extend_from_slice(token.as_ref());
    }

    #[inline]
    fn push(&mut self, token: u8) {
        self.output.push(token);
    }

    fn encode_bytes(&mut self, v: &[u8]) {
        self.push_slice(v.len().itoa());
        self.push(TOKEN_LEN);
        self.push_slice(v);
    }
}

impl<'a> Serializer for &'a mut BencodeSerializer {
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
        self.push(TOKEN_INT);
        self.push_slice(value.itoa());
        self.push(TOKEN_END);
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
        self.push(TOKEN_INT);
        self.push_slice(value.itoa());
        self.push(TOKEN_END);
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
        self.push(TOKEN_LIST);
        value.serialize(&mut *self)?;
        self.push(TOKEN_END);
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.push(TOKEN_LIST);
        Ok(self)
    }

    fn serialize_tuple(self, size: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(size))
    }

    fn serialize_tuple_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct> {
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

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapSerializer::new(self, len.unwrap_or(0)))
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.push(TOKEN_DICT);
        variant.serialize(&mut *self)?;
        Ok(MapSerializer::new(self, len))
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

impl<'a> ser::SerializeSeq for &'a mut BencodeSerializer {
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

impl<'a> ser::SerializeTuple for &'a mut BencodeSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<Self::Ok> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut BencodeSerializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized + ser::Serialize>(&mut self, value: &T) -> Result<Self::Ok> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut BencodeSerializer {
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
pub struct MapSerializer<'a> {
    /// Bencode Serializer.
    ser: &'a mut BencodeSerializer,
    /// Bencode Dictionary.
    dict: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Current key.
    cur_key: Option<Vec<u8>>,
}

impl<'a> MapSerializer<'a> {
    pub fn new(ser: &'a mut BencodeSerializer, _len: usize) -> MapSerializer {
        MapSerializer {
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

    fn serialize<T>(&self, value: &T) -> Result<Vec<u8>>
    where
        T: ?Sized + Serialize,
    {
        let mut serializer = BencodeSerializer::new();
        value.serialize(&mut serializer)?;
        Ok(serializer.output)
    }
}

impl<'a> ser::SerializeMap for MapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        if self.cur_key.is_some() {
            return Err(Error::Custom(
                "`serialize_key` called multiple times without calling  `serialize_value`".to_string(),
            ));
        }

        let mut encoded = self.serialize(key)?;
        match encoded.first() {
            Some(b'0'..=b'9') => {}
            _ => return Err(Error::ArbitraryMapKeysUnsupported),
        }

        let colon = encoded.iter().position(|b| *b == TOKEN_LEN).unwrap();
        encoded.drain(0..colon + 1);

        self.cur_key = Some(encoded);

        Ok(())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        match self.cur_key.take() {
            Some(bytes) => {
                let encoded = self.serialize(value)?;
                if !encoded.is_empty() {
                    self.dict.insert(bytes, encoded);
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

impl<'a> ser::SerializeStruct for MapSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        ser::SerializeMap::serialize_entry(self, key, value)
    }

    fn end(mut self) -> Result<Self::Ok> {
        self.flush()
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

    fn end(mut self) -> Result<Self::Ok> {
        self.flush()?;
        self.ser.push(TOKEN_END);
        Ok(())
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
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
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
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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

/// Serializes an object into a bencode `Vec<u8>`.
pub fn encode<T: ser::Serialize>(b: &T) -> Result<Vec<u8>> {
    let mut ser = BencodeSerializer::new();
    b.serialize(&mut ser)?;
    Ok(ser.output)
}

/// Serializes an object into a bencode `Vec<u8>` with a given capacity.
pub fn encode_with_capacity<T: ser::Serialize>(b: &T, capacity: usize) -> Result<Vec<u8>> {
    let mut ser = BencodeSerializer::with_capacity(capacity);
    b.serialize(&mut ser)?;
    Ok(ser.output)
}
