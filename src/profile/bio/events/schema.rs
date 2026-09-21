//! Recognition of one `eventsTF` metadata object: its identity, its measure variant, and its
//! bounded text fields.

use super::super::{bounded_id, optional_text, MAX_SOURCE_ID};
use super::{Event, Key};
use serde_json::{Map, Value};

pub(super) const MAX_EVENT_TEXT_BYTES: usize = 4_096;

pub(super) fn parse(object: &Map<String, Value>) -> Option<(Key, Event)> {
    let id = object
        .get("IDEvent")
        .and_then(Value::as_u64)
        .filter(|id| bounded_id(*id))?;
    let variant = variant(object.get("IDEventType"))?;
    let name = object
        .get("Event")
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())?;
    if ["Event", "Description", "Type", "Units"]
        .into_iter()
        .any(|key| {
            object.get(key).is_some_and(|value| {
                !value.is_null()
                    && value
                        .as_str()
                        .is_none_or(|text| text.len() > MAX_EVENT_TEXT_BYTES)
            })
        })
    {
        return None;
    }
    // FieldMeasureType S/L describes source display formatting, not seconds/meters.
    Some((
        (id, variant),
        Event {
            name: name.to_owned(),
            description: optional_text(object.get("Description")),
            kind: optional_text(object.get("Type")),
            units: optional_text(object.get("Units")),
            personal: object
                .get("PersonalEvent")
                .and_then(Value::as_bool)
                .is_some_and(|value| value),
        },
    ))
}

pub(super) fn variant(value: Option<&Value>) -> Option<Option<u64>> {
    match value {
        None | Some(Value::Null) => Some(None),
        Some(value) => value
            .as_u64()
            .filter(|value| *value <= MAX_SOURCE_ID)
            .map(Some),
    }
}
