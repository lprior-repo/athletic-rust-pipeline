use census_domain::model::StoreIdentity;
use serde::{Deserialize, Deserializer};

use crate::{StoreError, StoreResult};

use super::{StoreLineageError, KEY, PHASE};

#[derive(Default)]
struct Representation(bool);

impl<'de> Deserialize<'de> for Representation {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        serde::de::IgnoredAny::deserialize(deserializer)?;
        Ok(Self(true))
    }
}

#[derive(Deserialize)]
struct Envelope {
    key: Option<String>,
    payload: Option<serde_json::Value>,
    #[serde(default, rename = "type")]
    representation: Representation,
}

struct InlineEnvelope(Envelope);

impl<'de> Deserialize<'de> for InlineEnvelope {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct MapOnly;

        impl<'de> serde::de::Visitor<'de> for MapOnly {
            type Value = InlineEnvelope;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an inline store identity object")
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                map: A,
            ) -> Result<Self::Value, A::Error> {
                Envelope::deserialize(serde::de::value::MapAccessDeserializer::new(map))
                    .map(InlineEnvelope)
            }
        }

        deserializer.deserialize_map(MapOnly)
    }
}

pub(super) fn read(
    raw: &[u8],
    meter: &mut crate::acquisition::admission::ReadMeter<'_>,
) -> StoreResult<StoreIdentity> {
    meter.decode(raw)?;
    drop(
        serde_json::from_slice::<serde_json::Value>(raw).map_err(|source| StoreError::Decode {
            key: format!("{PHASE}/{KEY}"),
            source,
        })?,
    );
    meter.decode(raw)?;
    let InlineEnvelope(envelope) = serde_json::from_slice(raw).map_err(|source| {
        if source.is_data() {
            StoreLineageError::MalformedEnvelope.into()
        } else {
            StoreError::Decode {
                key: format!("{PHASE}/{KEY}"),
                source,
            }
        }
    })?;
    if envelope.representation.0 {
        return Err(StoreLineageError::UnsupportedRepresentation.into());
    }
    match envelope.key.as_deref() {
        Some(KEY) => {}
        Some(_) => return Err(StoreLineageError::UnexpectedKey.into()),
        None => return Err(StoreLineageError::MalformedEnvelope.into()),
    }
    let value = envelope
        .payload
        .as_ref()
        .and_then(serde_json::Value::as_str)
        .ok_or(StoreLineageError::MalformedEnvelope)?;
    Ok(StoreIdentity::parse(value)?)
}
