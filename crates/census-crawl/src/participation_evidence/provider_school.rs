use super::{bound, hash, invalid, RetainedHtmlCaptureV1, MAX_PACKET};
use crate::CrawlResult;
use census_store::StoreResult;
use serde::{Deserialize, Serialize};

mod directory;
mod json;
mod projection;
mod row;
mod types;

pub use types::{
    CheckedProviderSchoolPacketV1, ProviderSchoolDirectoryProofV1, ProviderSchoolPacketRefV1,
};

pub const PROVIDER_SCHOOL_INTAKE_PHASE: &str = "provider_school_intake_v1";
const INTERPRETATION: &str = "milesplit_directory_roster_school_v1";

#[derive(Debug, Clone, Serialize)]
pub struct ProviderSchoolPacketV1 {
    version: u32,
    interpretation: String,
    directory: RetainedHtmlCaptureV1,
    roster: RetainedHtmlCaptureV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    version: u32,
    interpretation: String,
    directory: json::Capture,
    roster: json::Capture,
}

impl<'de> Deserialize<'de> for ProviderSchoolPacketV1 {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let json::MapOnly(wire) = json::MapOnly::<Wire>::deserialize(decoder)?;
        Ok(Self {
            version: wire.version,
            interpretation: wire.interpretation,
            directory: wire.directory.0,
            roster: wire.roster.0,
        })
    }
}

impl ProviderSchoolPacketV1 {
    pub fn new(
        directory: RetainedHtmlCaptureV1,
        roster: RetainedHtmlCaptureV1,
    ) -> StoreResult<Self> {
        let packet = Self {
            version: 1,
            interpretation: INTERPRETATION.into(),
            directory,
            roster,
        };
        packet.encode()?;
        Ok(packet)
    }

    pub fn encode(&self) -> StoreResult<Vec<u8>> {
        if self.version != 1 || self.interpretation != INTERPRETATION {
            return Err(invalid("unsupported provider school interpretation"));
        }
        super::codec::encode(self, MAX_PACKET)
    }

    pub fn decode(bytes: &[u8]) -> StoreResult<Self> {
        bound(bytes.len(), MAX_PACKET, "provider school packet bytes")?;
        let packet: Self = serde_json::from_slice(bytes).map_err(super::json)?;
        if packet.encode()? != bytes {
            return Err(invalid("provider school packet is not canonical"));
        }
        Ok(packet)
    }

    pub fn reference(&self) -> StoreResult<ProviderSchoolPacketRefV1> {
        ProviderSchoolPacketRefV1::parse(&hash(&self.encode()?))
    }

    pub fn directory(&self) -> &RetainedHtmlCaptureV1 {
        &self.directory
    }
    pub fn roster(&self) -> &RetainedHtmlCaptureV1 {
        &self.roster
    }

    pub fn read(&self) -> CrawlResult<CheckedProviderSchoolPacketV1> {
        let reference = self.reference()?;
        let selected = directory::read(&self.directory, self.roster.requested_url())?;
        projection::read(self, reference, selected)
    }
}

#[cfg(test)]
mod tests;
