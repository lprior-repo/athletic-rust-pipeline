use super::{
    legacy_wire, owned, with_admitted_envelope, Field, JournalPath, LegacyEntry, RawEnvelope,
};
use crate::{JournalReadBudget, StoreResult};
use serde_json::value::RawValue;

pub(crate) struct JournalRawEnvelope {
    at: String,
    payload: Box<RawValue>,
}

impl JournalRawEnvelope {
    pub(crate) fn at(&self) -> &str {
        &self.at
    }

    pub(crate) fn payload(&self) -> &RawValue {
        &self.payload
    }
}

pub(in crate::read::view) fn entry(
    path: JournalPath<'_>,
    what: &str,
    raw: &[u8],
    budget: JournalReadBudget,
) -> StoreResult<LegacyEntry<JournalRawEnvelope>> {
    with_admitted_envelope(path, what, raw, budget, |envelope| {
        let at = timestamp(path, what, &envelope)?;
        let decoded = owned(path, what, envelope)?;
        let payload = match decoded.payload {
            Some(payload) => Some(JournalRawEnvelope {
                at: at.ok_or_else(|| path.refusal("journal envelope timestamp is missing"))?,
                payload,
            }),
            None => None,
        };
        Ok(LegacyEntry {
            header: decoded.header,
            payload,
        })
    })
}

pub(in crate::read::view) fn projected_entry<T>(
    path: JournalPath<'_>,
    what: &str,
    raw: &[u8],
    budget: JournalReadBudget,
    project: &impl Fn(&RawValue) -> StoreResult<T>,
) -> StoreResult<LegacyEntry<T>> {
    with_admitted_envelope(path, what, raw, budget, |envelope| {
        drop(timestamp(path, what, &envelope)?);
        super::projected(path, what, envelope, project)
    })
}

fn timestamp(
    path: JournalPath<'_>,
    what: &str,
    envelope: &RawEnvelope<'_>,
) -> StoreResult<Option<String>> {
    if envelope.get(Field::Type).is_some() {
        return Ok(None);
    }
    let raw = envelope
        .get(Field::At)
        .ok_or_else(|| path.refusal("journal envelope timestamp is missing"))?;
    legacy_wire::limit(path, "encoded timestamp", raw.get().len(), 128)?;
    legacy_wire::record(path, what, raw.get().as_bytes()).map(Some)
}
