use super::{legacy_wire, owned, with_admitted_envelope, Field, JournalPath, LegacyEntry};
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
        let at = if envelope.get(Field::Type).is_none() {
            let raw = envelope
                .get(Field::At)
                .ok_or_else(|| path.refusal("journal envelope timestamp is missing"))?;
            legacy_wire::limit(path, "encoded timestamp", raw.get().len(), 128)?;
            Some(legacy_wire::record::<String>(
                path,
                what,
                raw.get().as_bytes(),
            )?)
        } else {
            None
        };
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
