use super::engine_valid::{valid_date, valid_hex64, MAX_LOGICAL_ID_BYTES};
use super::*;
use crate::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceClaim {
    claim_id: String,
    subject_id: String,
    capture_digest: String,
    observed_on: String,
    payload_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawClaim {
    claim_id: String,
    subject_id: String,
    capture_digest: String,
    observed_on: String,
    payload_digest: String,
}

impl SourceClaim {
    pub fn new(
        subject: &SourceSubject,
        capture: &SourceCapture,
        observed_on: String,
        payload_digest: String,
    ) -> Result<Self, DomainError> {
        if capture.subject_id() != subject.logical_id() {
            return Err(DomainError::Unsupported {
                field: "capture_digest",
            });
        }
        if !valid_date(&observed_on) {
            return Err(DomainError::Unsupported {
                field: "observed_on",
            });
        }
        if !valid_hex64(&payload_digest) {
            return Err(DomainError::Unsupported {
                field: "payload_digest",
            });
        }
        let claim_id = format!("claim:{}:{payload_digest}", subject.logical_id());
        if claim_id.len() > MAX_LOGICAL_ID_BYTES {
            return Err(DomainError::OutOfRange { field: "claim_id" });
        }
        Ok(Self {
            claim_id,
            subject_id: subject.logical_id().to_string(),
            capture_digest: capture.body_hash().to_string(),
            observed_on,
            payload_digest,
        })
    }
    pub fn claim_id(&self) -> &str {
        &self.claim_id
    }
    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }
    pub fn capture_digest(&self) -> &str {
        &self.capture_digest
    }
    pub fn observed_on(&self) -> &str {
        &self.observed_on
    }
    pub fn payload_digest(&self) -> &str {
        &self.payload_digest
    }
}

impl<'de> Deserialize<'de> for SourceClaim {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawClaim::deserialize(deserializer)?;
        if !valid_hex64(&raw.capture_digest) {
            return Err(serde::de::Error::custom(
                "capture_digest is not 64-char hex",
            ));
        }
        if !valid_hex64(&raw.payload_digest) {
            return Err(serde::de::Error::custom(
                "payload_digest is not 64-char hex",
            ));
        }
        if !valid_date(&raw.observed_on) {
            return Err(serde::de::Error::custom("observed_on is not YYYY-MM-DD"));
        }
        let expected = format!("claim:{}:{}", raw.subject_id, raw.payload_digest);
        if expected != raw.claim_id {
            return Err(serde::de::Error::custom(
                "claim_id does not match subject and digest",
            ));
        }
        if raw.claim_id.len() > MAX_LOGICAL_ID_BYTES {
            return Err(serde::de::Error::custom(
                "claim_id is outside its permitted range",
            ));
        }
        Ok(Self {
            claim_id: raw.claim_id,
            subject_id: raw.subject_id,
            capture_digest: raw.capture_digest,
            observed_on: raw.observed_on,
            payload_digest: raw.payload_digest,
        })
    }
}
