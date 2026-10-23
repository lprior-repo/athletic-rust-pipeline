use super::{CheckedProviderSchoolPacketV1, ProviderSchoolPacketRefV1};
use crate::participation_evidence::invalid;
use crate::CrawlResult;
use census_store::{PublicSchoolIntakeCaptureV1, PublicSchoolIntakePacketV1, StoreResult};

mod capture;
mod child;
mod projection;

pub const OWNED_PROVIDER_SCHOOL_INTAKE_PHASE: &str = "public_resource_school_intake_v1";
pub const OWNED_PROVIDER_SCHOOL_INTERPRETATION: &str = "milesplit_owned_directory_roster_school_v2";
pub const OWNED_PROVIDER_SCHOOL_SEMANTIC_CONTRACT: &str =
    "milesplit-owned-retained-current-interpretation/v1";

#[derive(Debug, Clone, serde::Serialize)]
#[serde(transparent)]
pub struct OwnedProviderSchoolPacketV1 {
    packet: PublicSchoolIntakePacketV1,
}

impl OwnedProviderSchoolPacketV1 {
    pub fn from_packet(packet: PublicSchoolIntakePacketV1) -> Self {
        Self { packet }
    }

    pub fn decode(bytes: &[u8]) -> StoreResult<Self> {
        Ok(Self {
            packet: PublicSchoolIntakePacketV1::decode(bytes)?,
        })
    }

    pub fn encode(&self) -> StoreResult<Vec<u8>> {
        self.packet.encode()
    }

    pub fn reference(&self) -> StoreResult<ProviderSchoolPacketRefV1> {
        ProviderSchoolPacketRefV1::parse(self.packet.reference().as_str())
    }

    pub fn directory(&self) -> &PublicSchoolIntakeCaptureV1 {
        self.packet.directory()
    }

    pub fn roster(&self) -> &PublicSchoolIntakeCaptureV1 {
        self.packet.roster()
    }

    pub fn read(&self) -> CrawlResult<CheckedProviderSchoolPacketV1> {
        let directory = capture::read(self.directory())?;
        let roster = capture::read(self.roster())?;
        let selected = child::read(&self.packet, &directory.fetch, &roster.fetch)?;
        projection::read(self, selected, &directory, &roster)
    }

    pub fn read_semantic_contract(
        &self,
        contract: &str,
    ) -> CrawlResult<CheckedProviderSchoolPacketV1> {
        if contract != OWNED_PROVIDER_SCHOOL_SEMANTIC_CONTRACT {
            return Err(invalid("unsupported owned provider school semantic contract").into());
        }
        self.read()
    }
}
