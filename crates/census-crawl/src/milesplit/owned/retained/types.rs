use super::CaptureWitnessError;
use crate::milesplit::owned::MAX_OWNED_BODY_BYTES;
use crate::net::FetchOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedAcquisitionRef {
    manifest_key: String,
    meet_id: u64,
}

impl OwnedAcquisitionRef {
    pub fn from_capture(meet_id: u64, capture: &FetchOutcome) -> Result<Self, CaptureWitnessError> {
        let key = super::acquisition_manifest_key(meet_id, capture).map_err(|error| {
            super::error::corrupt("owned acquisition reference", error.to_string())
        })?;
        Self::parse(&key)
    }

    pub fn parse(key: &str) -> Result<Self, CaptureWitnessError> {
        if key.len() > 106 {
            return Err(CaptureWitnessError::Unsupported {
                key: "oversized acquisition reference".into(),
                field: "reference",
                detail: format!(
                    "reference is {} bytes, past the 106-byte ceiling",
                    key.len()
                ),
            });
        }
        let mut parts = key.split('/');
        let valid = parts.next() == Some("acquisition");
        let meet = parts.next().filter(|value| positive_id(value));
        let digest = parts.next().filter(|value| hex_digest(value));
        if !valid
            || meet.is_none()
            || digest.is_none()
            || parts.next() != Some("manifest")
            || parts.next().is_some()
        {
            return Err(CaptureWitnessError::Unsupported {
                key: key.into(),
                field: "reference",
                detail: "expected acquisition/{positive meet ID}/{64 lowercase hex}/manifest"
                    .into(),
            });
        }
        let meet_id = meet
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or_else(|| CaptureWitnessError::Unsupported {
                key: key.into(),
                field: "reference",
                detail: "invalid positive meet ID".into(),
            })?;
        let mut manifest_key = String::new();
        manifest_key
            .try_reserve_exact(key.len())
            .map_err(|source| CaptureWitnessError::Allocation {
                key: key.into(),
                needed: key.len(),
                source,
            })?;
        manifest_key.push_str(key);
        Ok(Self {
            manifest_key,
            meet_id,
        })
    }

    pub fn manifest_key(&self) -> &str {
        &self.manifest_key
    }

    pub const fn meet_id(&self) -> u64 {
        self.meet_id
    }
}

impl std::str::FromStr for OwnedAcquisitionRef {
    type Err = CaptureWitnessError;

    fn from_str(key: &str) -> Result<Self, Self::Err> {
        Self::parse(key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptureReadBudget {
    max_body_bytes: usize,
}

impl CaptureReadBudget {
    pub fn new(max_body_bytes: usize) -> Result<Self, CaptureWitnessError> {
        if max_body_bytes == 0 || max_body_bytes > MAX_OWNED_BODY_BYTES {
            return Err(CaptureWitnessError::Budget {
                key: "capture read budget".into(),
                what: "body ceiling",
                needed: max_body_bytes,
                max: MAX_OWNED_BODY_BYTES,
            });
        }
        Ok(Self { max_body_bytes })
    }

    pub const fn max_body_bytes(self) -> usize {
        self.max_body_bytes
    }
}

#[derive(Debug)]
pub struct VerifiedOwnedAcquisition {
    reference: OwnedAcquisitionRef,
    capture: FetchOutcome,
}

impl VerifiedOwnedAcquisition {
    pub(super) fn new(reference: OwnedAcquisitionRef, capture: FetchOutcome) -> Self {
        Self { reference, capture }
    }

    pub fn reference(&self) -> &OwnedAcquisitionRef {
        &self.reference
    }

    pub fn capture(&self) -> &FetchOutcome {
        &self.capture
    }

    pub fn body(&self) -> &[u8] {
        &self.capture.body
    }
}

pub(super) fn hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn positive_id(value: &str) -> bool {
    !value.starts_with('0')
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value.parse::<u64>().is_ok_and(|id| id > 0)
}
