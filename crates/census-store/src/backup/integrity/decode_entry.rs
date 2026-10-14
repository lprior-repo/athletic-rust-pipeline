use super::{
    decode_chunks, decode_digest, CorruptionClass, Decode, StoreResult, MANIFEST_TYPE,
    OBJECT_ALIAS_TYPE, OBJECT_CHUNK_TYPE, OBJECT_INDEX_TYPE, OBJECT_MANIFEST_TYPE,
};
use serde_json::Value;

pub(super) fn entry(
    decode: &mut Decode,
    phase: &str,
    locator: &str,
    row_key: &[u8],
    raw: &[u8],
) -> StoreResult<()> {
    let subject = format!("journal/{phase}/{locator}");
    let parsed: Value = match serde_json::from_slice(raw) {
        Ok(parsed) => parsed,
        Err(source) => {
            decode.note(CorruptionClass::Undecodable, subject, source.to_string());
            return Ok(());
        }
    };
    match parsed.get("type").and_then(Value::as_str) {
        Some(MANIFEST_TYPE) => {
            decode_chunks::chunked(decode, phase, locator, row_key, &parsed, subject)
        }
        Some(OBJECT_CHUNK_TYPE) => {
            decode.object_chunk(subject, &parsed);
            Ok(())
        }
        Some(OBJECT_MANIFEST_TYPE) => {
            decode.object_manifest(subject, &parsed);
            Ok(())
        }
        Some(OBJECT_INDEX_TYPE) => {
            decode.object_metadata(subject, &parsed, &["entries"]);
            Ok(())
        }
        Some(OBJECT_ALIAS_TYPE) => decode.object_alias(phase, locator, &parsed, subject),
        _ => {
            if let Some(payload) = parsed.get("payload") {
                decode_digest::digest(decode, phase, locator, payload, subject);
            } else {
                decode.note(
                    CorruptionClass::Undecodable,
                    subject,
                    "the journal entry carries no payload".to_string(),
                );
            }
            Ok(())
        }
    }
}
