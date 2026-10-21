use super::CaptureDomainError;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "HeaderWire")]
pub struct ObservedHttpHeaderV1 {
    name: String,
    value: Vec<u8>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeaderWire {
    name: String,
    #[serde(deserialize_with = "deserialize_value")]
    value: Vec<u8>,
}

impl ObservedHttpHeaderV1 {
    pub fn new(name: &str, value: Vec<u8>) -> Result<Self, CaptureDomainError> {
        if name.is_empty() || !name.bytes().all(is_token) {
            return Err(CaptureDomainError::Invalid {
                field: "observed header name",
            });
        }
        if name.len() > 128 {
            return Err(CaptureDomainError::Bound {
                field: "observed header name",
                max: 128,
            });
        }
        if value.len() > 2048 {
            return Err(CaptureDomainError::Bound {
                field: "observed header value",
                max: 2048,
            });
        }
        if !valid_header_value(&value) {
            return Err(CaptureDomainError::Invalid {
                field: "observed header value",
            });
        }
        Ok(Self {
            name: name.to_owned(),
            value,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &[u8] {
        &self.value
    }
    pub fn charge_chain_bytes(
        headers: &[Self],
        initial: usize,
    ) -> Result<usize, CaptureDomainError> {
        if initial > 16 * 1024 {
            return Err(CaptureDomainError::Bound {
                field: "observed chain header bytes",
                max: 16 * 1024,
            });
        }
        headers.iter().try_fold(initial, |count, header| {
            count
                .checked_add(header.name.len())
                .and_then(|count| count.checked_add(header.value.len()))
                .filter(|count| *count <= 16 * 1024)
                .ok_or(CaptureDomainError::Bound {
                    field: "observed chain header bytes",
                    max: 16 * 1024,
                })
        })
    }
}

pub(in crate::model::source_acquisition) fn valid_header_value(value: &[u8]) -> bool {
    value
        .iter()
        .all(|byte| *byte == b'\t' || (*byte >= b' ' && *byte != 0x7f))
}

impl TryFrom<HeaderWire> for ObservedHttpHeaderV1 {
    type Error = CaptureDomainError;

    fn try_from(wire: HeaderWire) -> Result<Self, Self::Error> {
        Self::new(&wire.name, wire.value)
    }
}

fn is_token(byte: u8) -> bool {
    byte.is_ascii_lowercase()
        || byte.is_ascii_digit()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

fn deserialize_value<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    crate::model::serde_bounds::sequence::<D, u8, 2048>(deserializer)
}
