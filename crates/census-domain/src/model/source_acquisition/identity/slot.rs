use super::super::scalar::{bounded, try_text, VersionOne};
use super::super::{AcquisitionSlotId, CaptureDomainError, PublicSourceResourceV1};
use crate::model::{serialized_digest, StoreIdentity};
use crate::UsJurisdiction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Stage {
    Results,
    SourceDiscovery,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Source {
    Milesplit,
    PublicWeb,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResourceRole {
    StructuredWholeMeet,
    PublicResource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SlotWire")]
pub struct AcquisitionSlotV1 {
    version: VersionOne,
    store_identity: StoreIdentity,
    owner_key: String,
    stage: Stage,
    #[serde(skip_serializing_if = "Option::is_none")]
    historical_year: Option<u16>,
    source: Source,
    source_site: UsJurisdiction,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_meet_id: Option<String>,
    resource_role: ResourceRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    public_resource: Option<Box<PublicSourceResourceV1>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SlotWire {
    version: VersionOne,
    store_identity: StoreIdentity,
    owner_key: String,
    stage: Stage,
    historical_year: Option<u16>,
    source: Source,
    source_site: UsJurisdiction,
    source_meet_id: Option<String>,
    resource_role: ResourceRole,
    public_resource: Option<PublicSourceResourceV1>,
}
impl AcquisitionSlotV1 {
    pub fn new(
        store_identity: StoreIdentity,
        owner_key: &str,
        historical_year: u16,
        source_site: UsJurisdiction,
        source_meet_id: &str,
    ) -> Result<Self, CaptureDomainError> {
        owner(owner_key)?;
        if !(2020..=2027).contains(&historical_year) {
            return Err(CaptureDomainError::Invalid {
                field: "slot scope",
            });
        }
        validate_meet_id(source_meet_id)?;
        Ok(Self {
            version: VersionOne,
            store_identity,
            owner_key: copy(owner_key)?,
            stage: Stage::Results,
            historical_year: Some(historical_year),
            source: Source::Milesplit,
            source_site,
            source_meet_id: Some(copy(source_meet_id)?),
            resource_role: ResourceRole::StructuredWholeMeet,
            public_resource: None,
        })
    }
    pub fn for_public_resource(
        store_identity: StoreIdentity,
        owner_key: &str,
        source_site: UsJurisdiction,
        resource: PublicSourceResourceV1,
    ) -> Result<Self, CaptureDomainError> {
        owner(owner_key)?;
        Ok(Self {
            version: VersionOne,
            store_identity,
            owner_key: copy(owner_key)?,
            stage: Stage::SourceDiscovery,
            historical_year: None,
            source: Source::PublicWeb,
            source_site,
            source_meet_id: None,
            resource_role: ResourceRole::PublicResource,
            public_resource: Some(Box::new(resource)),
        })
    }
    pub fn id(&self) -> Result<AcquisitionSlotId, CaptureDomainError> {
        let digest = serialized_digest(&("source-acquisition-slot-v1", self))
            .map_err(|e| CaptureDomainError::Encoding(e.to_string()))?;
        AcquisitionSlotId::parse(&digest)
    }
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store_identity
    }
    pub fn owner_key(&self) -> &str {
        &self.owner_key
    }
    pub fn historical_year(&self) -> Option<u16> {
        self.historical_year
    }
    pub fn source_site(&self) -> UsJurisdiction {
        self.source_site
    }
    pub fn source_meet_id(&self) -> Option<&str> {
        self.source_meet_id.as_deref()
    }
    pub fn public_resource(&self) -> Option<&PublicSourceResourceV1> {
        self.public_resource.as_deref()
    }
}
impl TryFrom<SlotWire> for AcquisitionSlotV1 {
    type Error = CaptureDomainError;
    fn try_from(w: SlotWire) -> Result<Self, Self::Error> {
        let VersionOne = w.version;
        match (
            w.stage,
            w.source,
            w.resource_role,
            w.historical_year,
            w.source_meet_id,
            w.public_resource,
        ) {
            (
                Stage::Results,
                Source::Milesplit,
                ResourceRole::StructuredWholeMeet,
                Some(year),
                Some(meet),
                None,
            ) => Self::new(w.store_identity, &w.owner_key, year, w.source_site, &meet),
            (
                Stage::SourceDiscovery,
                Source::PublicWeb,
                ResourceRole::PublicResource,
                None,
                None,
                Some(resource),
            ) => Self::for_public_resource(w.store_identity, &w.owner_key, w.source_site, resource),
            _ => Err(CaptureDomainError::Invalid {
                field: "slot resource role",
            }),
        }
    }
}
fn owner(value: &str) -> Result<(), CaptureDomainError> {
    bounded(value, "owner key", 64)?;
    if value.trim() != value {
        return Err(CaptureDomainError::Invalid {
            field: "slot scope",
        });
    }
    Ok(())
}
fn copy(value: &str) -> Result<String, CaptureDomainError> {
    try_text(value).map_err(|_| CaptureDomainError::Invalid {
        field: "slot allocation",
    })
}
pub(in crate::model::source_acquisition) fn validate_meet_id(
    value: &str,
) -> Result<(), CaptureDomainError> {
    if value.len() > 20
        || value.is_empty()
        || value.starts_with('0')
        || !value.bytes().all(|b| b.is_ascii_digit())
        || value.parse::<u64>().is_err()
    {
        return Err(CaptureDomainError::Invalid {
            field: "source meet id",
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
