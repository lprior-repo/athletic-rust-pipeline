use super::quoted;
use super::write;
use super::Array;
use super::CanonicalJsonError;
use super::Object;
use serde::Serialize;
use serde::Serializer as SerdeSerializer;
use std::io;

pub struct Serializer<'w, W: io::Write> {
    pub writer: &'w mut W,
}

fn number<W: io::Write, N: std::fmt::Display>(
    writer: &mut W,
    value: N,
) -> Result<(), CanonicalJsonError> {
    write!(writer, "{value}").map_err(CanonicalJsonError::from)
}

impl<'w, W: io::Write> SerdeSerializer for Serializer<'w, W> {
    type Ok = ();
    type Error = CanonicalJsonError;
    type SerializeSeq = Array<'w, W>;
    type SerializeTuple = Array<'w, W>;
    type SerializeTupleStruct = Array<'w, W>;
    type SerializeTupleVariant = Array<'w, W>;
    type SerializeMap = Object<'w, W>;
    type SerializeStruct = Object<'w, W>;
    type SerializeStructVariant = Object<'w, W>;

    fn serialize_bool(self, value: bool) -> Result<(), CanonicalJsonError> {
        write(self.writer, if value { b"true" } else { b"false" })
    }

    fn serialize_i8(self, value: i8) -> Result<(), CanonicalJsonError> {
        number(self.writer, value)
    }

    fn serialize_i16(self, value: i16) -> Result<(), CanonicalJsonError> {
        number(self.writer, value)
    }

    fn serialize_i32(self, value: i32) -> Result<(), CanonicalJsonError> {
        number(self.writer, value)
    }

    fn serialize_i64(self, value: i64) -> Result<(), CanonicalJsonError> {
        number(self.writer, value)
    }

    fn serialize_u8(self, value: u8) -> Result<(), CanonicalJsonError> {
        number(self.writer, value)
    }

    fn serialize_u16(self, value: u16) -> Result<(), CanonicalJsonError> {
        number(self.writer, value)
    }

    fn serialize_u32(self, value: u32) -> Result<(), CanonicalJsonError> {
        number(self.writer, value)
    }

    fn serialize_u64(self, value: u64) -> Result<(), CanonicalJsonError> {
        number(self.writer, value)
    }

    fn serialize_i128(self, value: i128) -> Result<(), CanonicalJsonError> {
        number(self.writer, value)
    }

    fn serialize_u128(self, value: u128) -> Result<(), CanonicalJsonError> {
        number(self.writer, value)
    }

    fn serialize_f32(self, value: f32) -> Result<(), CanonicalJsonError> {
        if value.is_finite() {
            number(self.writer, zmij::Buffer::new().format_finite(value))
        } else {
            write(self.writer, b"null")
        }
    }

    fn serialize_f64(self, value: f64) -> Result<(), CanonicalJsonError> {
        if value.is_finite() {
            number(self.writer, zmij::Buffer::new().format_finite(value))
        } else {
            write(self.writer, b"null")
        }
    }

    fn serialize_char(self, value: char) -> Result<(), CanonicalJsonError> {
        let mut buffer = [0u8; 4];
        quoted(self.writer, value.encode_utf8(&mut buffer))
    }

    fn serialize_str(self, value: &str) -> Result<(), CanonicalJsonError> {
        quoted(self.writer, value)
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<(), CanonicalJsonError> {
        write(self.writer, b"[")?;
        let mut array = Array::open(self.writer, b"]");
        for byte in value {
            array.push(byte)?;
        }
        array.finish()
    }

    fn serialize_none(self) -> Result<(), CanonicalJsonError> {
        write(self.writer, b"null")
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), CanonicalJsonError> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), CanonicalJsonError> {
        write(self.writer, b"null")
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<(), CanonicalJsonError> {
        write(self.writer, b"null")
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<(), CanonicalJsonError> {
        quoted(self.writer, variant)
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<(), CanonicalJsonError> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), CanonicalJsonError> {
        write(self.writer, b"{")?;
        quoted(self.writer, variant)?;
        write(self.writer, b":")?;
        value.serialize(Serializer {
            writer: self.writer,
        })?;
        write(self.writer, b"}")
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, CanonicalJsonError> {
        write(self.writer, b"[")?;
        Ok(Array::open(self.writer, b"]"))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, CanonicalJsonError> {
        write(self.writer, b"[")?;
        Ok(Array::open(self.writer, b"]"))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, CanonicalJsonError> {
        write(self.writer, b"[")?;
        Ok(Array::open(self.writer, b"]"))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, CanonicalJsonError> {
        write(self.writer, b"{")?;
        quoted(self.writer, variant)?;
        write(self.writer, b":[")?;
        Ok(Array::open(self.writer, b"]}"))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, CanonicalJsonError> {
        write(self.writer, b"{")?;
        Ok(Object::open(self.writer, b"}"))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, CanonicalJsonError> {
        write(self.writer, b"{")?;
        Ok(Object::open(self.writer, b"}"))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, CanonicalJsonError> {
        write(self.writer, b"{")?;
        quoted(self.writer, variant)?;
        write(self.writer, b":{")?;
        Ok(Object::open(self.writer, b"}}"))
    }

    fn is_human_readable(&self) -> bool {
        true
    }

    fn collect_str<T: std::fmt::Display + ?Sized>(
        self,
        value: &T,
    ) -> Result<(), CanonicalJsonError> {
        quoted(self.writer, &value.to_string())
    }
}
