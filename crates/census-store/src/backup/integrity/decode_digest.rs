use serde_json::Value;

use super::{CorruptionClass, Decode};

pub(super) const OWNED_DIGEST_PHASES: [&str; 4] = [
    "milesplit_owned_meet_v1",
    "milesplit_owned_meet_v2",
    "milesplit_owned_meet_v3",
    "milesplit_owned_meet_v4",
];

pub(super) fn digest(
    decode: &mut Decode,
    phase: &str,
    locator: &str,
    payload: &Value,
    subject: String,
) {
    if !OWNED_DIGEST_PHASES.contains(&phase) || !locator.starts_with("row/") {
        return;
    }
    let Some((_, recorded)) = locator.rsplit_once('/') else {
        return;
    };
    if recorded.len() != 64 || !recorded.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return;
    }
    let actual = match census_domain::model::serialized_digest(payload) {
        Ok(actual) => actual,
        Err(error) => {
            decode.note(
                CorruptionClass::Undecodable,
                subject,
                format!(
                    "the payload cannot be hashed for the {recorded} digest its key records: \
                     {error}"
                ),
            );
            return;
        }
    };
    if actual != recorded {
        decode.note(
            CorruptionClass::DigestMismatch,
            subject,
            format!("the key records {recorded} and the payload hashes to {actual}"),
        );
    }
}

pub(super) fn digest_chunked(
    decode: &mut Decode,
    phase: &str,
    locator: &str,
    collected: Vec<u8>,
    subject: String,
) {
    let entry: Value = match serde_json::from_slice(&collected) {
        Ok(entry) => entry,
        Err(source) => {
            decode.note(CorruptionClass::Undecodable, subject, source.to_string());
            return;
        }
    };
    if let Some(payload) = entry.get("payload") {
        digest(decode, phase, locator, payload, subject);
    } else {
        decode.note(
            CorruptionClass::Undecodable,
            subject,
            "the journal entry carries no payload".to_string(),
        );
    }
}
