pub(crate) mod envelope;

use serde::de::{Error, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{value::RawValue, Map, Value};

use super::legacy_wire::{self, LegacyEntry};
use super::object_wire::JournalPath;
use crate::{JournalReadBudget, StoreError, StoreResult};

#[derive(Clone, Copy, Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum Field {
    Type,
    Version,
    Key,
    At,
    Payload,
    Chunks,
    TotalBytes,
    Index,
    Total,
    Data,
}

impl Field {
    fn index(self) -> usize {
        match self {
            Self::Type => 0,
            Self::Version => 1,
            Self::Key => 2,
            Self::At => 3,
            Self::Payload => 4,
            Self::Chunks => 5,
            Self::TotalBytes => 6,
            Self::Index => 7,
            Self::Total => 8,
            Self::Data => 9,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Version => "version",
            Self::Key => "key",
            Self::At => "at",
            Self::Payload => "payload",
            Self::Chunks => "chunks",
            Self::TotalBytes => "total_bytes",
            Self::Index => "index",
            Self::Total => "total",
            Self::Data => "data",
        }
    }
}

struct RawEnvelope<'de>([Option<&'de RawValue>; 10]);

impl RawEnvelope<'_> {
    fn get(&self, field: Field) -> Option<&RawValue> {
        self.0.get(field.index()).copied().flatten()
    }
}

impl<'de> Deserialize<'de> for RawEnvelope<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(EnvelopeVisitor)
    }
}

struct EnvelopeVisitor;

impl<'de> Visitor<'de> for EnvelopeVisitor {
    type Value = RawEnvelope<'de>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a legacy journal envelope with unique known fields")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut fields = [None; 10];
        std::iter::from_fn(|| envelope_field(&mut map, &mut fields).transpose())
            .take(11)
            .try_for_each(std::convert::identity)?;
        Ok(RawEnvelope(fields))
    }
}

fn envelope_field<'de, A: MapAccess<'de>>(
    map: &mut A,
    fields: &mut [Option<&'de RawValue>; 10],
) -> Result<Option<()>, A::Error> {
    let Some(field) = map.next_key::<Field>()? else {
        return Ok(None);
    };
    let slot = fields
        .get_mut(field.index())
        .ok_or_else(|| A::Error::custom("legacy envelope field index is invalid"))?;
    if slot.is_some() {
        return Err(A::Error::custom(format!(
            "duplicate legacy envelope field {}",
            field.name()
        )));
    }
    *slot = Some(map.next_value::<&RawValue>()?);
    Ok(Some(()))
}

fn with_admitted_envelope<T>(
    path: JournalPath<'_>,
    what: &str,
    raw: &[u8],
    budget: JournalReadBudget,
    own: impl FnOnce(RawEnvelope<'_>) -> StoreResult<T>,
) -> StoreResult<T> {
    let envelope: RawEnvelope<'_> = legacy_wire::record(path, what, raw)?;
    if envelope.get(Field::Type).is_none() {
        legacy_wire::limit(
            path,
            "encoded envelope",
            raw.len(),
            budget.max_encoded_bytes(),
        )?;
    }
    own(envelope)
}

fn projected<T>(
    path: JournalPath<'_>,
    what: &str,
    envelope: RawEnvelope<'_>,
    project: impl FnOnce(&RawValue) -> StoreResult<T>,
) -> StoreResult<LegacyEntry<T>> {
    let mut header = Map::new();
    [Field::Type, Field::Key, Field::Data]
        .into_iter()
        .try_for_each(|field| -> StoreResult<()> {
            if let Some(raw) = envelope.get(field) {
                let value: String = legacy_wire::record(path, what, raw.get().as_bytes())?;
                header.insert(field.name().to_owned(), Value::String(value));
            }
            Ok(())
        })?;
    [
        Field::Version,
        Field::Chunks,
        Field::TotalBytes,
        Field::Index,
        Field::Total,
    ]
    .into_iter()
    .try_for_each(|field| -> StoreResult<()> {
        if let Some(raw) = envelope.get(field) {
            let value: u64 = legacy_wire::record(path, what, raw.get().as_bytes())?;
            header.insert(field.name().to_owned(), Value::from(value));
        }
        Ok(())
    })?;
    let payload = match envelope.get(Field::Payload) {
        Some(raw) if envelope.get(Field::Type).is_none() => Some(project(raw)?),
        Some(_) | None => None,
    };
    Ok(LegacyEntry {
        header: Value::Object(header),
        payload,
    })
}

fn owned(
    path: JournalPath<'_>,
    what: &str,
    envelope: RawEnvelope<'_>,
) -> StoreResult<LegacyEntry<Box<RawValue>>> {
    projected(path, what, envelope, |raw| owned_payload(path, raw))
}

fn owned_payload(path: JournalPath<'_>, raw: &RawValue) -> StoreResult<Box<RawValue>> {
    let mut payload = String::new();
    payload
        .try_reserve_exact(raw.get().len())
        .map_err(|source| StoreError::Refused {
            detail: format!(
                "legacy journal {}/{} cannot reserve {} payload bytes: {source}",
                path.phase,
                path.key,
                raw.get().len()
            ),
        })?;
    payload.push_str(raw.get());
    RawValue::from_string(payload).map_err(|source| StoreError::Decode {
        key: format!("legacy journal payload for {}/{}", path.phase, path.key),
        source,
    })
}

pub(super) fn entry(
    path: JournalPath<'_>,
    what: &str,
    raw: &[u8],
    budget: JournalReadBudget,
) -> StoreResult<LegacyEntry<Box<RawValue>>> {
    with_admitted_envelope(path, what, raw, budget, |envelope| {
        owned(path, what, envelope)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_boundary_checks_all_ten_fields_and_eleventh_key(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let fields = r#""type":null,"version":null,"key":null,"at":null,"payload":null,"chunks":null,"total_bytes":null,"index":null,"total":null,"data":null"#;
        let raw = format!("{{{fields}}}");
        let path = JournalPath {
            phase: "p",
            key: "k",
        };
        let envelope: RawEnvelope<'_> = legacy_wire::record(path, "root", raw.as_bytes())?;
        check!(eq; envelope.0.iter().flatten().count(), 10);
        for suffix in [r#""\u006bey":"#, r#""unknown":"#] {
            let raw = format!("{{{fields},{suffix}}}");
            match legacy_wire::record::<RawEnvelope<'_>>(path, "root", raw.as_bytes()) {
                Err(StoreError::Decode { source, .. }) => {
                    check!(eq; source.classify(), serde_json::error::Category::Data);
                }
                Err(error) => return Err(error.into()),
                Ok(_) => return Err("eleventh envelope field was accepted".into()),
            }
        }
        Ok(())
    }

    #[test]
    fn oversized_inline_never_enters_owned_decoder() -> Result<(), Box<dyn std::error::Error>> {
        let raw = br#"{"key":"k","payload":{"x":null,"x":1}}"#;
        let entered = std::cell::Cell::new(false);
        let result = with_admitted_envelope(
            JournalPath {
                phase: "p",
                key: "k",
            },
            "root",
            raw,
            JournalReadBudget::new(1)?,
            |_| {
                entered.set(true);
                Ok(())
            },
        );
        check!(!entered.get());
        check!(matches!(result, Err(StoreError::JournalReadLimit { .. })));
        Ok(())
    }

    #[test]
    fn unknown_nested_metadata_never_enters_owned_decoder() -> Result<(), Box<dyn std::error::Error>>
    {
        let raw = br#"{"key":"k","unknown":{"a":[{"b":[null,null]}]},"payload":true}"#;
        let entered = std::cell::Cell::new(false);
        let result = with_admitted_envelope(
            JournalPath {
                phase: "p",
                key: "k",
            },
            "root",
            raw,
            JournalReadBudget::new(raw.len())?,
            |_| {
                entered.set(true);
                Ok(())
            },
        );
        check!(!entered.get());
        check!(matches!(result, Err(StoreError::Decode { .. })));
        Ok(())
    }
}
