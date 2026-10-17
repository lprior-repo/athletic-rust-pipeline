pub mod manifest;
use super::provider_school::{CheckedProviderSchoolPacketV1, ProviderSchoolPacketV1};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1;
use census_store::{RetainedCaptureRootV1, RosterSourceEvidenceV1, StoreResult};

mod binding;
pub mod capture_objects;
pub mod policies;
mod source;
mod semantic;
pub use semantic::{assess_reviewed_roster_source_v1, ReviewedRosterSourceAssessmentV1};
pub use binding::{
    assess_existing_roster_binding, RosterBindingUnverifiedReferenceV1,
    RosterExistingBindingAssessmentV1,
};
pub use source::assess_retained_roster_source;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterSourceUnverifiedJoinV1 {
    ParserQualification,
    ExistingSchoolProgramAndConflictInventory,
    SemanticReview,
}

pub struct RosterSourceEvidenceAssessmentV1<'a> {
    evidence: &'a RosterSourceEvidenceV1,
    parser_manifest: manifest::CheckedRosterParserManifestV1,
    proposal: LocallyCheckedRosterIssuanceV1,
    root: RetainedCaptureRootV1,
    checked: CheckedProviderSchoolPacketV1,
    retained_packet: ProviderSchoolPacketV1,
    roster_body: String,
    scoped_rows: usize,
    capture_objects: capture_objects::PreparedRosterCaptureObjectsV1<'a>,
}

impl RosterSourceEvidenceAssessmentV1<'_> {
    pub fn capture_objects(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&capture_objects::PreparedRosterCaptureObjectsV1<'_>> {
        self.require_original_grant(grant)?;
        Ok(&self.capture_objects)
    }

    pub fn parser_manifest(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&manifest::CheckedRosterParserManifestV1> {
        self.require_original_grant(grant)?;
        Ok(&self.parser_manifest)
    }

    pub fn evidence(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&RosterSourceEvidenceV1> {
        self.require_original_grant(grant)?;
        Ok(self.evidence)
    }

    pub fn require_original_grant(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
        self.evidence.require_original_grant(grant)
    }

    pub fn proposal(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&LocallyCheckedRosterIssuanceV1> {
        self.require_original_grant(grant)?;
        Ok(&self.proposal)
    }

    pub fn root(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&RetainedCaptureRootV1> {
        self.require_original_grant(grant)?;
        Ok(&self.root)
    }

    pub fn checked_packet(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<&CheckedProviderSchoolPacketV1> {
        self.require_original_grant(grant)?;
        Ok(&self.checked)
    }

    pub fn retained_packet(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&ProviderSchoolPacketV1> {
        self.require_original_grant(grant)?;
        Ok(&self.retained_packet)
    }

    pub fn roster_body(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&str> {
        self.require_original_grant(grant)?;
        Ok(&self.roster_body)
    }

    pub fn scoped_rows(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<usize> {
        self.require_original_grant(grant)?;
        Ok(self.scoped_rows)
    }

    pub fn parser_provenance_coverage(&self) -> &'static str {
        "compiled source, configuration, schema and lock inputs; runtime binary and resolved feature graph excluded"
    }

    pub fn unverified_joins(&self) -> &'static [RosterSourceUnverifiedJoinV1] {
        use RosterSourceUnverifiedJoinV1::*;
        &[
            ParserQualification,
            ExistingSchoolProgramAndConflictInventory,
            SemanticReview,
        ]
    }
}
