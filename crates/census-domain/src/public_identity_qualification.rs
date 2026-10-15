use crate::model::StoreIdentity;
use serde::{Deserialize, Serialize};

mod codec;
#[cfg(test)]
mod tests;

pub const PUBLIC_IDENTITY_QUALIFICATION_LIFECYCLE: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PublicIdentityQualificationV1(Declaration);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Declaration {
    version: VersionOne,
    lifecycle: QualificationLifecycle,
    #[serde(deserialize_with = "codec::store_identity")]
    store_identity: StoreIdentity,
    protocol: IdentityProtocol,
    history_version: VersionOne,
    acceptance: IdentityAcceptance,
}

impl PublicIdentityQualificationV1 {
    pub fn new(store_identity: StoreIdentity) -> Self {
        Self(Declaration {
            version: VersionOne,
            lifecycle: QualificationLifecycle::FreshQualification,
            store_identity,
            protocol: IdentityProtocol::PublicIdentityLedgerV1,
            history_version: VersionOne,
            acceptance: IdentityAcceptance::SourceBoundEstablishmentV1,
        })
    }

    pub fn store_identity(&self) -> &StoreIdentity {
        &self.0.store_identity
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum QualificationLifecycle {
    FreshQualification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum IdentityProtocol {
    PublicIdentityLedgerV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum IdentityAcceptance {
    SourceBoundEstablishmentV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
struct VersionOne;

impl TryFrom<u8> for VersionOne {
    type Error = UnsupportedVersion;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self),
            found => Err(UnsupportedVersion(found)),
        }
    }
}

impl From<VersionOne> for u8 {
    fn from(_: VersionOne) -> Self {
        1
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unsupported public identity qualification version {0}")]
struct UnsupportedVersion(u8);
