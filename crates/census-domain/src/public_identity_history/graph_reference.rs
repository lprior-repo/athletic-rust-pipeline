use super::{record::is_digest, HistoryError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Wire")]
pub struct GraphAuthorityReferenceV1 {
    pub version: u32,
    pub capture_digest: String,
    pub seal_root: String,
}

impl GraphAuthorityReferenceV1 {
    pub fn new(capture_digest: String, seal_root: String) -> Result<Self, HistoryError> {
        let value = Self {
            version: 1,
            capture_digest,
            seal_root,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), HistoryError> {
        if self.version != 1 || !is_digest(&self.capture_digest) || !is_digest(&self.seal_root) {
            return Err(HistoryError::Invalid("graph authority reference"));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    version: u32,
    capture_digest: String,
    seal_root: String,
}

impl TryFrom<Wire> for GraphAuthorityReferenceV1 {
    type Error = HistoryError;

    fn try_from(value: Wire) -> Result<Self, Self::Error> {
        let reference = Self {
            version: value.version,
            capture_digest: value.capture_digest,
            seal_root: value.seal_root,
        };
        reference.validate()?;
        Ok(reference)
    }
}

#[cfg(test)]
mod tests;
