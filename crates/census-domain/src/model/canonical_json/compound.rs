use super::quoted;
use super::write;
use super::CanonicalJsonError;
use super::CanonicalSerializer;
use serde::ser::SerializeMap;
use serde::ser::SerializeSeq;
use serde::ser::SerializeStruct;
use serde::ser::SerializeStructVariant;
use serde::ser::SerializeTuple;
use serde::ser::SerializeTupleStruct;
use serde::ser::SerializeTupleVariant;
use serde::Serialize;
use std::io;

pub struct Array<'w, W: io::Write> {
    writer: &'w mut W,
    first: bool,
    close: &'static [u8],
}

impl<'w, W: io::Write> Array<'w, W> {
    pub(super) fn open(writer: &'w mut W, close: &'static [u8]) -> Self {
        Self {
            writer,
            first: true,
            close,
        }
    }

    pub(super) fn push<T: Serialize + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<(), CanonicalJsonError> {
        self.separate()?;
        value.serialize(CanonicalSerializer {
            writer: self.writer,
        })
    }

    fn separate(&mut self) -> Result<(), CanonicalJsonError> {
        if self.first {
            self.first = false;
            Ok(())
        } else {
            write(self.writer, b",")
        }
    }

    pub(super) fn finish(self) -> Result<(), CanonicalJsonError> {
        write(self.writer, self.close)
    }
}

impl<W: io::Write> SerializeSeq for Array<'_, W> {
    type Ok = ();
    type Error = CanonicalJsonError;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.push(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<W: io::Write> SerializeTuple for Array<'_, W> {
    type Ok = ();
    type Error = CanonicalJsonError;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.push(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<W: io::Write> SerializeTupleStruct for Array<'_, W> {
    type Ok = ();
    type Error = CanonicalJsonError;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.push(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<W: io::Write> SerializeTupleVariant for Array<'_, W> {
    type Ok = ();
    type Error = CanonicalJsonError;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.push(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

pub struct Object<'w, W: io::Write> {
    writer: &'w mut W,
    first: bool,
    close: &'static [u8],
}

impl<'w, W: io::Write> Object<'w, W> {
    pub(super) fn open(writer: &'w mut W, close: &'static [u8]) -> Self {
        Self {
            writer,
            first: true,
            close,
        }
    }

    pub(super) fn field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), CanonicalJsonError> {
        self.separate()?;
        quoted(self.writer, key)?;
        write(self.writer, b":")?;
        value.serialize(CanonicalSerializer {
            writer: self.writer,
        })
    }

    fn key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), CanonicalJsonError> {
        let mut buffer = Vec::new();
        key.serialize(CanonicalSerializer {
            writer: &mut buffer,
        })?;
        match buffer.first() {
            None => Err(CanonicalJsonError::Unsupported("empty key".to_string())),
            Some(b'"') => write(self.writer, &buffer),
            Some(b'0'..=b'9') | Some(b'-') | Some(b't') | Some(b'f') => {
                quoted(self.writer, &String::from_utf8_lossy(&buffer))
            }
            Some(other) => Err(CanonicalJsonError::Unsupported(format!(
                "json object key starting with {:?}",
                char::from(*other)
            ))),
        }
    }

    fn separate(&mut self) -> Result<(), CanonicalJsonError> {
        if self.first {
            self.first = false;
            Ok(())
        } else {
            write(self.writer, b",")
        }
    }

    fn finish(self) -> Result<(), CanonicalJsonError> {
        write(self.writer, self.close)
    }
}

impl<W: io::Write> SerializeMap for Object<'_, W> {
    type Ok = ();
    type Error = CanonicalJsonError;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Self::Error> {
        self.separate()?;
        self.key(key)?;
        write(self.writer, b":")
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error> {
        value.serialize(CanonicalSerializer {
            writer: self.writer,
        })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<W: io::Write> SerializeStruct for Object<'_, W> {
    type Ok = ();
    type Error = CanonicalJsonError;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.field(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}

impl<W: io::Write> SerializeStructVariant for Object<'_, W> {
    type Ok = ();
    type Error = CanonicalJsonError;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.field(key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish()
    }
}
