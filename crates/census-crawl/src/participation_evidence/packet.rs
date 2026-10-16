use super::{
    bound, hash, invalid, json, schedule, status, IndependentParticipationProofV1,
    ParticipationEvidenceRefV1, RetainedHtmlCaptureV1, VerifiedIndependentParticipationV1,
    INTERPRETATION, MAX_PACKET,
};
use census_domain::model::StoreIdentity;
use census_store::StoreResult;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IndependentParticipationPacketV1 {
    version: u32,
    status: RetainedHtmlCaptureV1,
    schedule: RetainedHtmlCaptureV1,
}

impl IndependentParticipationPacketV1 {
    pub fn new(
        status: RetainedHtmlCaptureV1,
        schedule: RetainedHtmlCaptureV1,
    ) -> StoreResult<Self> {
        let packet = Self {
            version: 1,
            status,
            schedule,
        };
        packet.bytes()?;
        Ok(packet)
    }
    pub fn reference(&self) -> StoreResult<ParticipationEvidenceRefV1> {
        ParticipationEvidenceRefV1::parse(&hash(&self.bytes()?))
    }
    pub fn payload(&self) -> StoreResult<Value> {
        serde_json::from_slice(&self.bytes()?).map_err(json)
    }
    pub fn capture_references(&self) -> StoreResult<Vec<String>> {
        Ok(vec![
            self.reference()?.as_str().into(),
            self.status.body_sha256().into(),
            self.schedule.body_sha256().into(),
        ])
    }
    fn bytes(&self) -> StoreResult<Vec<u8>> {
        let bytes = super::codec::encode(self, MAX_PACKET)?;
        bound(
            bytes.len(),
            MAX_PACKET,
            "independent participation packet bytes",
        )?;
        Ok(bytes)
    }
    pub(super) fn read(
        &self,
        store_identity: StoreIdentity,
        snapshot_sequence: u64,
    ) -> StoreResult<VerifiedIndependentParticipationV1> {
        if self.version != 1 {
            return Err(invalid(
                "unsupported independent participation packet version",
            ));
        }
        let status = status::read(&self.status)?;
        let season = schedule::read(&self.schedule, &status.day)?;
        let cohort = census_domain::model::GradYear::of(status.grade, season.school_year)
            .ok_or_else(|| invalid("source status grade/season cohort unsupported"))?;
        let status_proof = proof(&self.status, &status.subject.locator);
        let season_proof = proof(&self.schedule, &season.locator);
        Ok(VerifiedIndependentParticipationV1 {
            store_identity,
            snapshot_sequence,
            reference: self.reference()?,
            subject: status.subject,
            ownership: status.ownership,
            day: status.day,
            school_year: season.school_year,
            cohort,
            competition: season.competition,
            venue: season.venue,
            season_scope: season.scope,
            status: status_proof,
            season: season_proof,
        })
    }
}

fn proof(capture: &RetainedHtmlCaptureV1, locator: &str) -> IndependentParticipationProofV1 {
    IndependentParticipationProofV1 {
        capture: capture.body_sha256().into(),
        interpretation: hash(
            format!("{INTERPRETATION}\n{}\n{locator}", capture.body_sha256()).as_bytes(),
        ),
        locator: locator.into(),
        acquired_at: capture.acquired_at(),
    }
}
