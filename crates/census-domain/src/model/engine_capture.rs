use super::engine_valid::{
    valid_date, valid_digest, valid_hex64, valid_url, MAX_CAPTURE_BYTES, MAX_LOGICAL_ID_BYTES,
};
use super::*;
use crate::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceCapture {
    capture_id: String,
    subject_id: String,
    body_hash: String,
    byte_len: u64,
    captured_at: String,
    source_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCapture {
    capture_id: String,
    subject_id: String,
    body_hash: String,
    byte_len: u64,
    captured_at: String,
    source_url: Option<String>,
}

impl SourceCapture {
    pub fn new(
        subject: &SourceSubject,
        body_hash: String,
        byte_len: u64,
        captured_at: String,
        source_url: Option<String>,
    ) -> Result<Self, DomainError> {
        if !valid_hex64(&body_hash) {
            return Err(DomainError::Unsupported { field: "body_hash" });
        }
        if byte_len == 0 || byte_len > MAX_CAPTURE_BYTES {
            return Err(DomainError::OutOfRange { field: "byte_len" });
        }
        if !valid_date(&captured_at) {
            return Err(DomainError::Unsupported {
                field: "captured_at",
            });
        }
        if let Some(url) = &source_url {
            if !valid_url(url) {
                return Err(DomainError::Unsupported {
                    field: "source_url",
                });
            }
        }
        let capture_id = format!("capture:{}:{body_hash}", subject.logical_id());
        if capture_id.len() > MAX_LOGICAL_ID_BYTES {
            return Err(DomainError::OutOfRange {
                field: "capture_id",
            });
        }
        Ok(Self {
            capture_id,
            subject_id: subject.logical_id().to_string(),
            body_hash,
            byte_len,
            captured_at,
            source_url,
        })
    }
    pub fn capture_id(&self) -> &str {
        &self.capture_id
    }
    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }
    pub fn body_hash(&self) -> &str {
        &self.body_hash
    }
    pub fn byte_len(&self) -> u64 {
        self.byte_len
    }
    pub fn captured_at(&self) -> &str {
        &self.captured_at
    }
    pub fn source_url(&self) -> Option<&str> {
        self.source_url.as_deref()
    }
}

impl<'de> Deserialize<'de> for SourceCapture {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawCapture::deserialize(deserializer)?;
        if !valid_hex64(&raw.body_hash) {
            return Err(serde::de::Error::custom(
                "body_hash is not 64-char lowercase hex",
            ));
        }
        if raw.byte_len == 0 || raw.byte_len > MAX_CAPTURE_BYTES {
            return Err(serde::de::Error::custom(
                "byte_len is outside its permitted range",
            ));
        }
        if !valid_date(&raw.captured_at) {
            return Err(serde::de::Error::custom("captured_at is not YYYY-MM-DD"));
        }
        if let Some(url) = &raw.source_url {
            if !valid_url(url) {
                return Err(serde::de::Error::custom("source_url is not supported"));
            }
        }
        let expected = format!("capture:{}:{}", raw.subject_id, raw.body_hash);
        if expected != raw.capture_id {
            return Err(serde::de::Error::custom(
                "capture_id does not match subject and hash",
            ));
        }
        if raw.capture_id.len() > MAX_LOGICAL_ID_BYTES {
            return Err(serde::de::Error::custom(
                "capture_id is outside its permitted range",
            ));
        }
        if !valid_digest(&raw.subject_id) || raw.subject_id.len() > MAX_LOGICAL_ID_BYTES {
            return Err(serde::de::Error::custom(
                "subject_id is outside its permitted range",
            ));
        }
        Ok(Self {
            capture_id: raw.capture_id,
            subject_id: raw.subject_id,
            body_hash: raw.body_hash,
            byte_len: raw.byte_len,
            captured_at: raw.captured_at,
            source_url: raw.source_url,
        })
    }
}
