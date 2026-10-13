use super::ProfileError;
use crate::model::engine_valid::valid_hex64;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::num::NonZeroU32;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
struct Hex64(String);

impl Hex64 {
    fn parse(value: &str) -> Result<Self, ProfileError> {
        if !valid_hex64(value) {
            return Err(ProfileError::InvalidDigest);
        }
        let mut retained = String::new();
        retained
            .try_reserve_exact(64)
            .map_err(|_| ProfileError::AllocationFailed)?;
        retained.push_str(value);
        Ok(Self(retained))
    }
}

struct HexVisitor;

impl Visitor<'_> for HexVisitor {
    type Value = Hex64;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("exactly 64 lowercase hexadecimal bytes")
    }

    fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
        Hex64::parse(value).map_err(E::custom)
    }
}

impl<'de> Deserialize<'de> for Hex64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_str(HexVisitor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProfileId(Hex64);

impl ProfileId {
    pub fn parse(value: &str) -> Result<Self, ProfileError> {
        Hex64::parse(value).map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0 .0
    }
}

impl fmt::Display for ProfileId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContractRefV1 {
    version: u32,
    revision: NonZeroU32,
    digest: Hex64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContractInput {
    version: u32,
    revision: u32,
    digest: Hex64,
}

impl ContractRefV1 {
    pub fn new(version: u32, revision: u32, digest: &str) -> Result<Self, ProfileError> {
        let digest = Hex64::parse(digest)?;
        Self::checked(ContractInput {
            version,
            revision,
            digest,
        })
    }

    fn checked(input: ContractInput) -> Result<Self, ProfileError> {
        if input.version != 1 {
            return Err(ProfileError::UnsupportedContractVersion {
                found: input.version,
            });
        }
        let revision = NonZeroU32::new(input.revision).ok_or(ProfileError::ZeroContractRevision)?;
        Ok(Self {
            version: 1,
            revision,
            digest: input.digest,
        })
    }

    pub fn revision(&self) -> NonZeroU32 {
        self.revision
    }

    pub fn digest(&self) -> &str {
        &self.digest.0
    }
}

impl Serialize for ContractRefV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut fields = serializer.serialize_struct("ContractRefV1", 3)?;
        fields.serialize_field("version", &self.version)?;
        fields.serialize_field("revision", &self.revision)?;
        fields.serialize_field("digest", &self.digest)?;
        fields.end()
    }
}

impl<'de> Deserialize<'de> for ContractRefV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::checked(ContractInput::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfilePoliciesV1 {
    pub cohort: ContractRefV1,
    pub event: ContractRefV1,
    pub identity: ContractRefV1,
    pub affiliation: ContractRefV1,
    pub contact: ContractRefV1,
    pub participation: ContractRefV1,
    pub publication: ContractRefV1,
}
