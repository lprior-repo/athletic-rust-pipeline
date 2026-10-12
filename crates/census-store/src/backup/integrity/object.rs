use super::{count_at, length, CorruptionClass, Decode, StoreError, StoreResult};
use crate::journal::{object_digest, OBJECT_CHUNK_BYTES};
use serde_json::Value;

impl Decode<'_> {
    pub(super) fn object_alias(
        &mut self,
        phase: &str,
        locator: &str,
        alias: &Value,
        subject: String,
    ) -> StoreResult<()> {
        let Some(object) = alias
            .get("object")
            .and_then(Value::as_str)
            .filter(|object| crate::journal::is_object_id(object))
        else {
            self.note(
                CorruptionClass::Undecodable,
                subject,
                "invalid object alias digest".into(),
            );
            return Ok(());
        };
        let key = crate::journal::object_manifest_key(object)?;
        let Some(raw) = self
            .store
            .journal
            .get(key)
            .map_err(|source| StoreError::Read { source })?
        else {
            self.note(
                CorruptionClass::Truncated,
                subject,
                "the ready alias has no object manifest".into(),
            );
            return Ok(());
        };
        if !self.charge(1, length(raw.len())) {
            return Ok(());
        }
        let manifest: Value = match serde_json::from_slice(&raw) {
            Ok(manifest) => manifest,
            Err(error) => {
                self.note(CorruptionClass::Undecodable, subject, error.to_string());
                return Ok(());
            }
        };
        let Some((records, bytes)) = object_decode_charge(&manifest) else {
            self.note(
                CorruptionClass::Undecodable,
                subject,
                "object manifest omits record or byte counts".into(),
            );
            return Ok(());
        };
        if !self.charge(records, bytes) {
            return Ok(());
        }
        self.decode_object_records(phase, locator, subject)
    }

    fn decode_object_records(
        &mut self,
        phase: &str,
        locator: &str,
        subject: String,
    ) -> StoreResult<()> {
        let object = match self.store.journal_object(phase, locator) {
            Ok(Some(object)) => object,
            Ok(None) => {
                self.note(
                    CorruptionClass::Truncated,
                    subject,
                    "object alias vanished during integrity checking".into(),
                );
                return Ok(());
            }
            Err(error) => {
                self.note(CorruptionClass::Undecodable, subject, error.to_string());
                return Ok(());
            }
        };
        for record in object.records() {
            if let Err(error) = record {
                self.note(CorruptionClass::Undecodable, subject, error.to_string());
                break;
            }
        }
        Ok(())
    }

    pub(super) fn object_metadata(&mut self, subject: String, parsed: &Value, fields: &[&str]) {
        let missing = fields.iter().find(|field| parsed.get(**field).is_none());
        if let Some(field) = missing {
            self.note(
                CorruptionClass::Undecodable,
                subject,
                format!("the object journal record carries no {field}"),
            );
        }
    }

    pub(super) fn object_chunk(&mut self, mut subject: String, parsed: &Value) {
        let (Some(index), Some(declared)) = (count_at(parsed, "index"), count_at(parsed, "length"))
        else {
            self.note(
                CorruptionClass::Undecodable,
                subject,
                "the object journal chunk carries no index or length".to_string(),
            );
            return;
        };
        let Some(recorded) = parsed.get("digest").and_then(Value::as_str) else {
            self.note(
                CorruptionClass::Undecodable,
                subject,
                "the object journal chunk carries no digest".to_string(),
            );
            return;
        };
        let Some(data) = parsed.get("data").and_then(Value::as_str) else {
            self.note(
                CorruptionClass::Undecodable,
                subject,
                "the object journal chunk carries no data string".to_string(),
            );
            return;
        };
        let Some(decoded) = self.chunk_bytes(&subject, index, data, OBJECT_CHUNK_BYTES) else {
            return;
        };
        let size_check = self.validate_object_chunk_size(&mut subject, index, &decoded, declared);
        if !size_check {
            return;
        }
        let actual = object_digest(&decoded);
        if actual != recorded {
            self.note(
                CorruptionClass::DigestMismatch,
                subject,
                format!(
                    "the record declares {recorded} and object chunk {index} hashes to {actual}"
                ),
            );
        }
    }

    fn validate_object_chunk_size(
        &mut self,
        subject: &mut String,
        index: usize,
        decoded: &[u8],
        declared: usize,
    ) -> bool {
        if decoded.len() < declared {
            self.note(
                CorruptionClass::Truncated,
                std::mem::take(subject),
                format!(
                    "object chunk {index} decodes to {} bytes against the {declared} the record \
                     declares",
                    decoded.len()
                ),
            );
            return false;
        }
        if decoded.len() > declared {
            self.note(
                CorruptionClass::Oversized,
                std::mem::take(subject),
                format!(
                    "object chunk {index} decodes to {} bytes against the {declared} the record \
                     declares",
                    decoded.len()
                ),
            );
            return false;
        }
        true
    }

    pub(super) fn object_manifest(&mut self, subject: String, parsed: &Value) {
        let (Some(chunks), Some(total_bytes)) =
            (count_at(parsed, "chunks"), count_at(parsed, "total_bytes"))
        else {
            self.note(
                CorruptionClass::Undecodable,
                subject,
                "the object manifest records no chunk count or payload length".to_string(),
            );
            return;
        };
        let Some(digests) = parsed.get("chunk_digests").and_then(Value::as_array) else {
            self.note(
                CorruptionClass::Undecodable,
                subject,
                "the object manifest lists no chunk digests".to_string(),
            );
            return;
        };
        let ceiling = chunks.saturating_mul(OBJECT_CHUNK_BYTES);
        if total_bytes > ceiling {
            self.note(
                CorruptionClass::Oversized,
                subject,
                format!(
                    "the object manifest records a {total_bytes}-byte payload for {chunks} chunks, \
                     past the {ceiling}-byte ceiling those chunks carry"
                ),
            );
            return;
        }
        if digests.len() < chunks {
            self.note(
                CorruptionClass::Truncated,
                subject,
                format!(
                    "the object manifest lists {} chunk digests for {chunks} chunks",
                    digests.len()
                ),
            );
            return;
        }
        if digests.len() > chunks {
            self.note(
                CorruptionClass::Oversized,
                subject,
                format!(
                    "the object manifest lists {} chunk digests for {chunks} chunks",
                    digests.len()
                ),
            );
            return;
        }
        self.charge(0, length(total_bytes));
    }
}

fn object_decode_charge(manifest: &Value) -> Option<(u64, u64)> {
    let records = count_at(manifest, "records")?;
    let bytes = count_at(manifest, "total_bytes")?;
    let allocation = bytes.checked_mul(4).and_then(|bytes| {
        records
            .checked_mul(128)
            .and_then(|ranges| bytes.checked_add(ranges))
    });
    Some((length(records), allocation.map_or(u64::MAX, length)))
}
