use super::{bound, hash, invalid, RetainedHtmlCaptureV1, MAX_PACKET};
use census_store::StoreResult;
use serde::{Deserialize, Serialize};

mod json;
mod linked;
mod linked_event_label;
mod profile;
pub mod profile_pr;
pub mod roster_child;
mod types;
mod urls;

pub use linked_event_label::CheckedLinkedEventLabelV1;
pub use types::{
    CheckedLinkedCompetitionCategoryV1, CheckedMileSplitProfileSubjectV1,
    CheckedProviderTargetIntakeV1, CompetitionCategoryEvidenceV1, ProviderTargetProofV1,
    SourceTargetPacketRefV1, UnmappedPersonGenderV1,
};

const INTERPRETATION: &str = "milesplit_profile_linked_competition_target_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceTargetPacketV1 {
    version: u32,
    interpretation: String,
    profile: RetainedHtmlCaptureV1,
    linked_results: RetainedHtmlCaptureV1,
}

impl SourceTargetPacketV1 {
    pub fn new(
        profile: RetainedHtmlCaptureV1,
        linked_results: RetainedHtmlCaptureV1,
    ) -> StoreResult<Self> {
        let packet = Self {
            version: 1,
            interpretation: INTERPRETATION.into(),
            profile,
            linked_results,
        };
        packet.encode()?;
        Ok(packet)
    }

    pub fn encode(&self) -> StoreResult<Vec<u8>> {
        if self.version != 1 || self.interpretation != INTERPRETATION {
            return Err(invalid("unsupported provider target interpretation"));
        }
        super::codec::encode(self, MAX_PACKET)
    }

    pub fn decode(bytes: &[u8]) -> StoreResult<Self> {
        bound(bytes.len(), MAX_PACKET, "provider target packet bytes")?;
        let json::MapOnly(packet): json::MapOnly<Self> =
            serde_json::from_slice(bytes).map_err(super::json)?;
        if packet.encode()? != bytes {
            return Err(invalid("provider target packet is not canonical"));
        }
        Ok(packet)
    }

    pub fn reference(&self) -> StoreResult<SourceTargetPacketRefV1> {
        SourceTargetPacketRefV1::parse(&hash(&self.encode()?))
    }

    pub fn read(&self) -> StoreResult<CheckedProviderTargetIntakeV1> {
        let reference = self.reference()?;
        let profile = profile::read(&self.profile)?;
        let categories = linked::read(&self.linked_results, &profile)?;
        Ok(CheckedProviderTargetIntakeV1 {
            reference,
            profile,
            categories,
        })
    }
    pub fn read_event_label(
        &self,
        row_proof: &ProviderTargetProofV1,
    ) -> StoreResult<CheckedLinkedEventLabelV1> {
        linked_event_label::read(self, row_proof)
    }

    pub fn read_profile_prs(&self) -> StoreResult<profile_pr::CheckedProfilePrTableV1<'_>> {
        self.reference()?;
        let person = urls::person(self.profile.requested_url())?;
        profile_pr::read(
            &self.profile,
            census_domain::UsJurisdiction::Michigan,
            &person.id,
        )
    }

    pub fn profile_capture(&self) -> &RetainedHtmlCaptureV1 {
        &self.profile
    }

    pub fn read_profile_prs_if_published(
        &self,
    ) -> StoreResult<Option<profile_pr::CheckedProfilePrTableV1<'_>>> {
        self.reference()?;
        let person = urls::person(self.profile.requested_url())?;
        profile_pr::read_if_published(
            &self.profile,
            census_domain::UsJurisdiction::Michigan,
            &person.id,
        )
    }
}

#[cfg(test)]
mod tests;
