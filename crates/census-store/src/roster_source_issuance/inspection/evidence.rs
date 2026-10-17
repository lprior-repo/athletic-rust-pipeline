use super::RosterPhysicalProjectionRowV1;
use crate::{RetainedCaptureRootReadV1, StoreResult};
use census_domain::evidence_subject_attachment::{AttachmentReplayContinuationV1, AttachmentReplayGrantV1};

pub struct RosterSourceEvidenceV1 {
    store: census_domain::model::StoreIdentity,
    snapshot_sequence: u64,
    school: census_domain::model::SchoolId,
    program: census_domain::model::TeamId,
    held: RetainedCaptureRootReadV1,
    rows: Vec<RosterPhysicalProjectionRowV1>,
    continuation: AttachmentReplayContinuationV1,
}

impl RosterSourceEvidenceV1 {
    pub(super) fn new(
        store: census_domain::model::StoreIdentity,
        snapshot_sequence: u64,
        school: census_domain::model::SchoolId,
        program: census_domain::model::TeamId,
        held: RetainedCaptureRootReadV1,
        rows: Vec<RosterPhysicalProjectionRowV1>,
        continuation: AttachmentReplayContinuationV1,
    ) -> Self { Self { store, snapshot_sequence, school, program, held, rows, continuation } }

    pub fn store_identity(&self) -> &census_domain::model::StoreIdentity { &self.store }
    pub fn requested_school(&self) -> &census_domain::model::SchoolId { &self.school }
    pub fn requested_program(&self) -> &census_domain::model::TeamId { &self.program }
    pub fn snapshot_sequence(&self) -> u64 { self.snapshot_sequence }

    pub fn require_original_grant(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
        self.continuation.require(grant).map_err(super::super::source_error)
    }

    pub fn retained_root<'a>(&'a self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&'a RetainedCaptureRootReadV1> {
        self.require_original_grant(grant)?;
        Ok(&self.held)
    }

    pub fn physical_projection<'a>(&'a self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&'a [RosterPhysicalProjectionRowV1]> {
        self.require_original_grant(grant)?;
        Ok(&self.rows)
    }
}
