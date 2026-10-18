use super::{
    hash, invalid, json, markup, status, DocumentSubjectV1, IndependentParticipationProofV1,
    ParticipationEvidenceRefV1, RetainedHtmlCaptureV1, SubjectOwnershipV1, MAX_PACKET,
};
use census_domain::model::StoreIdentity;
use census_store::StoreResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod read;
mod statement;
mod verified;
pub use verified::VerifiedIndependentParticipationWindowV1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndependentParticipationWindowPacketV1 {
    version: u16,
    window: RetainedHtmlCaptureV1,
}

impl IndependentParticipationWindowPacketV1 {
    pub fn new(window: RetainedHtmlCaptureV1) -> StoreResult<Self> {
        let packet = Self { version: 1, window };
        packet.bytes()?;
        Ok(packet)
    }

    pub fn reference(&self) -> StoreResult<ParticipationEvidenceRefV1> {
        ParticipationEvidenceRefV1::parse(&hash(&self.bytes()?))
    }

    pub fn payload(&self) -> StoreResult<Value> {
        serde_json::from_slice(&self.bytes()?).map_err(json)
    }

    fn bytes(&self) -> StoreResult<Vec<u8>> {
        super::codec::encode(self, MAX_PACKET)
    }

    pub(super) fn read(
        &self,
        store_identity: StoreIdentity,
        snapshot_sequence: u64,
    ) -> StoreResult<VerifiedIndependentParticipationWindowV1> {
        if self.version != 1 {
            return Err(invalid(
                "unsupported independent participation window packet version",
            ));
        }
        let parsed = read::read(&self.window)?;
        Ok(VerifiedIndependentParticipationWindowV1 {
            store_identity,
            snapshot_sequence,
            reference: self.reference()?,
            subject: parsed.subject,
            ownership: parsed.ownership,
            interval: census_domain::model::ProfileDateRangeV1::new(
                parsed.boundary.from,
                parsed.boundary.through,
            )
            .map_err(|_| invalid("declared participation window is reversed"))?,
            documented_summer_end: parsed.boundary.documented_summer_end,
            school_year: parsed.boundary.school_year,
            cohort: parsed.boundary.cohort,
            proof: parsed.proof,
        })
    }
}
