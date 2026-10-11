use super::engine_valid::{MAX_LOGICAL_ID_BYTES, MAX_PROVIDER_ID_BYTES};
use super::*;
use crate::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceSubject {
    namespace: SourceNamespace,
    provider_id: String,
    logical_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSubject {
    namespace: SourceNamespace,
    provider_id: String,
    logical_id: String,
}

fn valid_provider_id(value: &str) -> bool {
    if value.trim() != value || value.is_empty() {
        return false;
    }
    if value.len() > MAX_PROVIDER_ID_BYTES {
        return false;
    }
    !value.bytes().any(|b| b == b':' || b == 0)
}

impl SourceSubject {
    pub fn new(namespace: SourceNamespace, provider_id: String) -> Result<Self, DomainError> {
        if !valid_provider_id(&provider_id) {
            return Err(DomainError::Unsupported {
                field: "provider_id",
            });
        }
        let logical_id = format!("{namespace}:{provider_id}");
        if logical_id.len() > MAX_LOGICAL_ID_BYTES {
            return Err(DomainError::OutOfRange {
                field: "logical_id",
            });
        }
        Ok(Self {
            namespace,
            provider_id,
            logical_id,
        })
    }
    pub fn namespace(&self) -> &SourceNamespace {
        &self.namespace
    }
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }
    pub fn logical_id(&self) -> &str {
        &self.logical_id
    }
    pub fn source_identity(&self) -> SourceIdentity {
        SourceIdentity::new(self.namespace.clone(), self.provider_id.clone())
    }
}

impl<'de> Deserialize<'de> for SourceSubject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawSubject::deserialize(deserializer)?;
        let rebuilt =
            Self::new(raw.namespace, raw.provider_id).map_err(serde::de::Error::custom)?;
        if rebuilt.logical_id != raw.logical_id {
            return Err(serde::de::Error::custom(
                "logical_id does not match namespace:id",
            ));
        }
        Ok(rebuilt)
    }
}
