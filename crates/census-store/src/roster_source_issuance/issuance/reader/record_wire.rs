use base64::display::Base64Display;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{
    de::{DeserializeSeed, SeqAccess, Visitor},
    ser::{SerializeSeq, SerializeTuple},
    Deserializer, Serialize, Serializer,
};

type Record = (Vec<u8>, Vec<u8>);

pub(super) fn serialize<S: Serializer>(rows: &[Record], serializer: S) -> Result<S::Ok, S::Error> {
    let mut seq = serializer.serialize_seq(Some(rows.len()))?;
    for (key, value) in rows {
        let row = RowEncoder { key, value };
        seq.serialize_element(&row)?;
    }
    seq.end()
}

struct RowEncoder<'a> {
    key: &'a [u8],
    value: &'a [u8],
}

impl<'a> Serialize for RowEncoder<'a> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut row = serializer.serialize_tuple(2)?;
        let key_display = Base64Display::new(self.key, &STANDARD);
        row.serialize_element(&DisplayWrapper(&key_display))?;
        let value_display = Base64Display::new(self.value, &STANDARD);
        row.serialize_element(&DisplayWrapper(&value_display))?;
        row.end()
    }
}

struct DisplayWrapper<'a, T>(&'a T);

impl<'a, T: std::fmt::Display> Serialize for DisplayWrapper<'a, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&self.0)
    }
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Record>, D::Error> {
    deserializer.deserialize_seq(RowsVisitor {
        max_rows: 4096,
        max_blob: 4 * 1024 * 1024,
    })
}

struct RowsVisitor {
    max_rows: usize,
    max_blob: usize,
}

impl<'de> Visitor<'de> for RowsVisitor {
    type Value = Vec<Record>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("sequence of record pairs")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut rows = Vec::new();

        loop {
            if rows.len() == self.max_rows {
                if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                    return Err(serde::de::Error::custom(
                        "record wire: row count exceeds 4096 bound",
                    ));
                }
                break;
            }

            match seq.next_element_seed(RowSeed {
                max_blob: self.max_blob,
            })? {
                Some(row) => {
                    rows.try_reserve(1)
                        .map_err(|_| serde::de::Error::custom("record wire: allocation"))?;
                    rows.push(row);
                }
                None => break,
            }
        }

        Ok(rows)
    }
}

struct RowSeed {
    max_blob: usize,
}

impl<'de> DeserializeSeed<'de> for RowSeed {
    type Value = Record;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let row = deserializer.deserialize_tuple(
            2,
            RowVisitor {
                max_blob: self.max_blob,
            },
        )?;
        Ok(row)
    }
}

struct RowVisitor {
    max_blob: usize,
}

impl<'de> Visitor<'de> for RowVisitor {
    type Value = Record;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("two-element record row")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let key = seq
            .next_element_seed(DecodedBlobSeed {
                max_bytes: self.max_blob,
            })?
            .ok_or_else(|| serde::de::Error::custom("record wire: missing key"))?;

        let value = seq
            .next_element_seed(DecodedBlobSeed {
                max_bytes: self.max_blob,
            })?
            .ok_or_else(|| serde::de::Error::custom("record wire: missing value"))?;

        if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
            return Err(serde::de::Error::custom(
                "record wire: extra element in row",
            ));
        }

        Ok((key, value))
    }
}

struct DecodedBlobSeed {
    max_bytes: usize,
}

impl<'de> DeserializeSeed<'de> for DecodedBlobSeed {
    type Value = Vec<u8>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_str(DecodedBlobVisitor {
            max_bytes: self.max_bytes,
        })
    }
}

struct DecodedBlobVisitor {
    max_bytes: usize,
}

impl<'de> Visitor<'de> for DecodedBlobVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("base64 blob")
    }

    fn visit_str<E: serde::de::Error>(self, input: &str) -> Result<Self::Value, E> {
        let encoded_limit = self
            .max_bytes
            .checked_add(2)
            .map(|bytes| bytes / 3)
            .and_then(|blocks| blocks.checked_mul(4))
            .ok_or_else(|| E::custom("record wire: encoded size overflow"))?;
        if input.len() > encoded_limit {
            return Err(serde::de::Error::custom(
                "record wire: base64 input exceeds encoded bound",
            ));
        }

        if !input.is_empty() && !input.len().is_multiple_of(4) {
            return Err(serde::de::Error::custom(
                "record wire: base64 length not multiple of 4",
            ));
        }

        let padding = if input.ends_with("==") {
            2
        } else if input.ends_with("=") {
            1
        } else {
            0
        };

        let quotient = input.len() / 4;
        let estimated = quotient
            .checked_mul(3)
            .ok_or_else(|| serde::de::Error::custom("record wire: size overflow"))?;
        let exact = estimated
            .checked_sub(padding)
            .ok_or_else(|| serde::de::Error::custom("record wire: size underflow"))?;

        if exact > self.max_bytes {
            return Err(serde::de::Error::custom(
                "record wire: decoded blob exceeds 4MiB bound",
            ));
        }

        let mut buf = Vec::new();
        buf.try_reserve_exact(exact)
            .map_err(|_| serde::de::Error::custom("record wire: allocation"))?;
        buf.resize(exact, 0);

        let decoded = STANDARD
            .decode_slice(input, &mut buf)
            .map_err(|_| serde::de::Error::custom("record wire: base64 decode"))?;

        if decoded != exact {
            return Err(serde::de::Error::custom(
                "record wire: base64 decoded size mismatch",
            ));
        }

        Ok(buf)
    }
}
