use std::marker::PhantomData;

use census_domain::model::{ReviewVerdict, ReviewVerdictKind, VerdictBatch};
use serde::de::value::{MapAccessDeserializer, StrDeserializer};
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use super::ModelError;

pub(super) fn message_content(body: &Value) -> Option<String> {
    let message = body.get("choices")?.get(0)?.get("message")?;
    let content = content_text(message)?;
    let trimmed = content.trim().to_string();
    (!trimmed.is_empty()).then_some(trimmed)
}

pub(super) fn content_text(message: &Value) -> Option<String> {
    match message.get("content")? {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let joined: String = parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("");
            Some(joined)
        }
        _ => None,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireBatch {
    subject_id: String,
    verdicts: Vec<Object<WireVerdict>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireVerdict {
    case_id: String,
    #[serde(deserialize_with = "deserialize_kind")]
    kind: ReviewVerdictKind,
    field: String,
    value: String,
    confidence: u8,
    rationale: String,
}

struct Object<T>(T);

struct ObjectVisitor<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(ObjectVisitor(PhantomData))
    }
}

impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
    type Value = Object<T>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON object")
    }

    fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
        T::deserialize(MapAccessDeserializer::new(map)).map(Object)
    }
}

fn deserialize_kind<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<ReviewVerdictKind, D::Error> {
    struct KindVisitor;

    impl<'de> Visitor<'de> for KindVisitor {
        type Value = ReviewVerdictKind;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a verdict kind string")
        }

        fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
            ReviewVerdictKind::deserialize(StrDeserializer::<E>::new(value))
        }
    }

    deserializer.deserialize_str(KindVisitor)
}

pub fn parse_batch(content: &str) -> Result<VerdictBatch, ModelError> {
    let Object(wire): Object<WireBatch> =
        serde_json::from_str(content).map_err(|_| ModelError::Content {
            reason: "not a verdict batch",
        })?;
    if wire
        .verdicts
        .iter()
        .any(|Object(verdict)| verdict.confidence > 100)
    {
        return Err(ModelError::Content {
            reason: "confidence exceeds 100",
        });
    }
    Ok(VerdictBatch {
        subject_id: wire.subject_id,
        verdicts: wire
            .verdicts
            .into_iter()
            .map(|Object(verdict)| ReviewVerdict {
                case_id: verdict.case_id,
                kind: verdict.kind,
                field: Some(verdict.field),
                value: Some(verdict.value),
                confidence: verdict.confidence,
                rationale: verdict.rationale,
            })
            .collect(),
    })
}
