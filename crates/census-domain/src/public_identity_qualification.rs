use crate::model::StoreIdentity;
use serde::{Deserialize, Serialize};

mod codec;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_v2;
#[cfg(test)]
mod tests_v3;

pub const PUBLIC_IDENTITY_QUALIFICATION_LIFECYCLE: u32 = 5;
pub const PUBLIC_IDENTITY_QUALIFICATION_V2_LIFECYCLE: u32 = 6;
pub const PUBLIC_IDENTITY_QUALIFICATION_V3_LIFECYCLE: u32 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PublicIdentityQualificationVersion {
    SourceBoundV1,
    RetainedAuthorityContextV2,
    RetainedAuthorityGraphV3,
}

impl PublicIdentityQualificationVersion {
    pub fn lifecycle_revision(self) -> u32 {
        match self {
            Self::SourceBoundV1 => PUBLIC_IDENTITY_QUALIFICATION_LIFECYCLE,
            Self::RetainedAuthorityContextV2 => PUBLIC_IDENTITY_QUALIFICATION_V2_LIFECYCLE,
            Self::RetainedAuthorityGraphV3 => PUBLIC_IDENTITY_QUALIFICATION_V3_LIFECYCLE,
        }
    }

    pub fn witness_version(self) -> u32 {
        match self {
            Self::SourceBoundV1 => 1,
            Self::RetainedAuthorityContextV2 => 2,
            Self::RetainedAuthorityGraphV3 => 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum PublicIdentityQualification {
    V1(PublicIdentityQualificationV1),
    V2(PublicIdentityQualificationV2),
    V3(PublicIdentityQualificationV3),
}

impl PublicIdentityQualification {
    pub fn store_identity(&self) -> &StoreIdentity {
        match self {
            Self::V1(declaration) => declaration.store_identity(),
            Self::V2(declaration) => declaration.store_identity(),
            Self::V3(declaration) => declaration.store_identity(),
        }
    }

    pub fn version(&self) -> PublicIdentityQualificationVersion {
        match self {
            Self::V1(_) => PublicIdentityQualificationVersion::SourceBoundV1,
            Self::V2(_) => PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
            Self::V3(_) => PublicIdentityQualificationVersion::RetainedAuthorityGraphV3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PublicIdentityQualificationV1(Declaration<1, IdentityAcceptance>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PublicIdentityQualificationV2(Declaration<2, RetainedAuthorityAcceptance>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PublicIdentityQualificationV3(Declaration<3, RetainedGraphAcceptance>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Declaration<const VERSION: u8, A> {
    version: FixedVersion<VERSION>,
    lifecycle: QualificationLifecycle,
    #[serde(deserialize_with = "codec::store_identity")]
    store_identity: StoreIdentity,
    protocol: IdentityProtocol,
    history_version: FixedVersion<1>,
    acceptance: A,
}

impl<const VERSION: u8, A> Declaration<VERSION, A> {
    fn new(store_identity: StoreIdentity, acceptance: A) -> Self {
        Self {
            version: FixedVersion,
            lifecycle: QualificationLifecycle::FreshQualification,
            store_identity,
            protocol: IdentityProtocol::PublicIdentityLedgerV1,
            history_version: FixedVersion,
            acceptance,
        }
    }
}

impl PublicIdentityQualificationV1 {
    pub fn new(store_identity: StoreIdentity) -> Self {
        Self(Declaration::new(
            store_identity,
            IdentityAcceptance::SourceBoundEstablishmentV1,
        ))
    }

    pub fn store_identity(&self) -> &StoreIdentity {
        &self.0.store_identity
    }
}

impl PublicIdentityQualificationV2 {
    pub fn new(store_identity: StoreIdentity) -> Self {
        Self(Declaration::new(
            store_identity,
            RetainedAuthorityAcceptance::RetainedAuthorityContextV2,
        ))
    }

    pub fn store_identity(&self) -> &StoreIdentity {
        &self.0.store_identity
    }
}

impl PublicIdentityQualificationV3 {
    pub fn new(store_identity: StoreIdentity) -> Self {
        Self(Declaration::new(
            store_identity,
            RetainedGraphAcceptance::RetainedAuthorityGraphV3,
        ))
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RetainedAuthorityAcceptance {
    RetainedAuthorityContextV2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
struct FixedVersion<const VERSION: u8>;

impl<const VERSION: u8> TryFrom<u8> for FixedVersion<VERSION> {
    type Error = UnsupportedVersion;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value == VERSION {
            Ok(Self)
        } else {
            Err(UnsupportedVersion(value))
        }
    }
}

impl<const VERSION: u8> From<FixedVersion<VERSION>> for u8 {
    fn from(_: FixedVersion<VERSION>) -> Self {
        VERSION
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unsupported public identity qualification version {0}")]
struct UnsupportedVersion(u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RetainedGraphAcceptance {
    RetainedAuthorityGraphV3,
}
