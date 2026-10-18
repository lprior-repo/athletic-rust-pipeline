use super::{
    DocumentSubjectV1, IndependentParticipationProofV1, ParticipationEvidenceRefV1,
    SubjectOwnershipV1,
};
use census_domain::model::{GradYear, ProfileDate, ProfileDateRangeV1, SchoolYear, StoreIdentity};

#[derive(Debug)]
pub struct VerifiedIndependentParticipationWindowV1 {
    pub(super) store_identity: StoreIdentity,
    pub(super) snapshot_sequence: u64,
    pub(super) reference: ParticipationEvidenceRefV1,
    pub(super) subject: DocumentSubjectV1,
    pub(super) ownership: SubjectOwnershipV1,
    pub(super) interval: ProfileDateRangeV1,
    pub(super) documented_summer_end: ProfileDate,
    pub(super) school_year: SchoolYear,
    pub(super) cohort: GradYear,
    pub(super) proof: IndependentParticipationProofV1,
}

impl VerifiedIndependentParticipationWindowV1 {
    pub fn store_identity(&self) -> &StoreIdentity {
        &self.store_identity
    }

    pub fn snapshot_sequence(&self) -> u64 {
        self.snapshot_sequence
    }

    pub fn reference(&self) -> &ParticipationEvidenceRefV1 {
        &self.reference
    }

    pub fn subject(&self) -> &DocumentSubjectV1 {
        &self.subject
    }

    pub fn ownership(&self) -> &SubjectOwnershipV1 {
        &self.ownership
    }

    pub fn from(&self) -> &ProfileDate {
        self.interval.from()
    }

    pub fn through(&self) -> &ProfileDate {
        self.interval.through()
    }

    pub fn interval(&self) -> &ProfileDateRangeV1 {
        &self.interval
    }

    pub fn documented_summer_end(&self) -> &ProfileDate {
        &self.documented_summer_end
    }

    pub fn school_year(&self) -> SchoolYear {
        self.school_year
    }

    pub fn cohort(&self) -> GradYear {
        self.cohort
    }

    pub fn boundary_proof(&self) -> &IndependentParticipationProofV1 {
        &self.proof
    }
}
