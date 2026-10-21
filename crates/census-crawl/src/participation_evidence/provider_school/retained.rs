use super::{CheckedProviderSchoolPacketV1, OwnedProviderSchoolPacketV1};
use super::{ProviderSchoolPacketRefV1, ProviderSchoolPacketV1};
use crate::participation_evidence::{bound, invalid, json, MAX_PACKET};
use crate::CrawlResult;
use census_store::{RetainedCaptureRootSchemaV1, StoreResult};
use serde::{Serialize, Serializer};

mod capture;
mod metadata;

pub use capture::RetainedProviderSchoolCaptureV1;

#[derive(Debug, Clone)]
pub enum RetainedProviderSchoolPacketV1 {
    Raw(Box<ProviderSchoolPacketV1>),
    Owned(Box<OwnedProviderSchoolPacketV1>),
}

impl RetainedProviderSchoolPacketV1 {
    pub fn from_root(schema: RetainedCaptureRootSchemaV1, bytes: &[u8]) -> StoreResult<Self> {
        bound(
            bytes.len(),
            MAX_PACKET,
            "retained provider school packet bytes",
        )?;
        match schema {
            RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1 => Ok(Self::Raw(Box::new(
                serde_json::from_slice(bytes).map_err(json)?,
            ))),
            RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1 => Ok(Self::Owned(
                Box::new(OwnedProviderSchoolPacketV1::decode(bytes)?),
            )),
            _ => Err(invalid("retained provider school root schema differs")),
        }
    }

    pub fn encode(&self) -> StoreResult<Vec<u8>> {
        match self {
            Self::Raw(packet) => packet.encode(),
            Self::Owned(packet) => packet.encode(),
        }
    }

    pub fn reference(&self) -> StoreResult<ProviderSchoolPacketRefV1> {
        match self {
            Self::Raw(packet) => packet.reference(),
            Self::Owned(packet) => packet.reference(),
        }
    }

    pub(crate) fn read(&self) -> CrawlResult<CheckedProviderSchoolPacketV1> {
        match self {
            Self::Raw(packet) => packet.read(),
            Self::Owned(packet) => packet.read(),
        }
    }

    pub(crate) fn read_semantic_contract(
        &self,
        contract: &str,
    ) -> CrawlResult<CheckedProviderSchoolPacketV1> {
        match self {
            Self::Raw(packet) => packet.read_semantic_contract(contract),
            Self::Owned(packet) => packet.read_semantic_contract(contract),
        }
    }

    pub(crate) fn directory(&self) -> RetainedProviderSchoolCaptureV1<'_> {
        match self {
            Self::Raw(packet) => RetainedProviderSchoolCaptureV1::Raw(packet.directory()),
            Self::Owned(packet) => RetainedProviderSchoolCaptureV1::Owned(packet.directory()),
        }
    }

    pub fn roster(&self) -> RetainedProviderSchoolCaptureV1<'_> {
        match self {
            Self::Raw(packet) => RetainedProviderSchoolCaptureV1::Raw(packet.roster()),
            Self::Owned(packet) => RetainedProviderSchoolCaptureV1::Owned(packet.roster()),
        }
    }
}

impl Serialize for RetainedProviderSchoolPacketV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Raw(packet) => packet.serialize(serializer),
            Self::Owned(packet) => packet.serialize(serializer),
        }
    }
}

#[cfg(test)]
mod tests;
