use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt;
use std::io;

mod compound;
mod serializer;
#[cfg(test)]
#[path = "canonical_json/tests.rs"]
mod tests;

pub(crate) use compound::{Array, Object};
pub(crate) use serializer::Serializer as CanonicalSerializer;

#[derive(Debug)]
pub enum CanonicalJsonError {
    Unsupported(String),
    Write(io::Error),
}

impl fmt::Display for CanonicalJsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(message) => write!(formatter, "unsupported json value: {message}"),
            Self::Write(error) => write!(formatter, "json write failed: {error}"),
        }
    }
}

impl std::error::Error for CanonicalJsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Write(error) => Some(error),
            Self::Unsupported(_) => None,
        }
    }
}

impl serde::ser::Error for CanonicalJsonError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::Unsupported(message.to_string())
    }
}

impl From<io::Error> for CanonicalJsonError {
    fn from(error: io::Error) -> Self {
        Self::Write(error)
    }
}

pub fn serialized_digest<T: Serialize + ?Sized>(value: &T) -> Result<String, CanonicalJsonError> {
    let mut writer = DigestWriter(Sha256::new());
    serialize_into(&mut writer, value)?;
    Ok(format!("{:x}", writer.0.finalize()))
}

#[cfg(test)]
pub(crate) fn serialized_bytes<T: Serialize + ?Sized>(
    value: &T,
) -> Result<Vec<u8>, CanonicalJsonError> {
    let mut bytes = Vec::new();
    serialize_into(&mut bytes, value)?;
    Ok(bytes)
}

fn serialize_into<W: io::Write, T: Serialize + ?Sized>(
    writer: &mut W,
    value: &T,
) -> Result<(), CanonicalJsonError> {
    value.serialize(CanonicalSerializer { writer })
}

pub(super) fn write(writer: &mut impl io::Write, bytes: &[u8]) -> Result<(), CanonicalJsonError> {
    writer.write_all(bytes)?;
    Ok(())
}

pub(super) fn quoted(writer: &mut impl io::Write, value: &str) -> Result<(), CanonicalJsonError> {
    write(writer, b"\"")?;
    for character in value.chars() {
        match character {
            '"' => write(writer, br#"\""#)?,
            '\\' => write(writer, br"\\")?,
            '\u{8}' => write(writer, br"\b")?,
            '\u{c}' => write(writer, br"\f")?,
            '\n' => write(writer, br"\n")?,
            '\r' => write(writer, br"\r")?,
            '\t' => write(writer, br"\t")?,
            control if control < '\u{20}' => {
                let escaped = format!("\\u{:04x}", u32::from(control));
                write(writer, escaped.as_bytes())?;
            }
            other => {
                let mut buffer = [0u8; 4];
                write(writer, other.encode_utf8(&mut buffer).as_bytes())?;
            }
        }
    }
    write(writer, b"\"")
}

struct DigestWriter(Sha256);

impl io::Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
