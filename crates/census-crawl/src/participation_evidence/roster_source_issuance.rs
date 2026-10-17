pub mod manifest;
use super::provider_school::CheckedProviderSchoolPacketV1;
use census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1;
use census_store::{RetainedCaptureRootV1, RosterSourceEvidenceV1, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;

mod source;
mod binding;
pub use binding::{assess_existing_roster_binding, RosterExistingBindingAssessmentV1, RosterBindingUnverifiedReferenceV1};
pub use source::assess_retained_roster_source;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RosterSourceUnverifiedJoinV1 {
    CaptureOccurrence,
    CaptureManifest,
    CaptureInterpretationObject,
    ParserQualification,
    ScopePolicy,
    ExistingSchoolProgramAndConflictInventory,
    SchoolYearConvention,
    SemanticReview,
}

pub struct RosterSourceEvidenceAssessmentV1<'a> {
    evidence: &'a RosterSourceEvidenceV1,
    parser_manifest: manifest::CheckedRosterParserManifestV1,
    proposal: LocallyCheckedRosterIssuanceV1,
    root: RetainedCaptureRootV1,
    checked: CheckedProviderSchoolPacketV1,
    scoped_rows: usize,
}

impl RosterSourceEvidenceAssessmentV1<'_> {
    pub fn parser_manifest(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&manifest::CheckedRosterParserManifestV1> {
        self.require_original_grant(grant)?;
        Ok(&self.parser_manifest)
    }

    pub fn evidence(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&RosterSourceEvidenceV1> {
        self.require_original_grant(grant)?;
        Ok(self.evidence)
    }

    pub fn require_original_grant(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
        self.evidence.require_original_grant(grant)
    }

    pub fn proposal(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&LocallyCheckedRosterIssuanceV1> {
        self.require_original_grant(grant)?;
        Ok(&self.proposal)
    }

    pub fn root(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&RetainedCaptureRootV1> {
        self.require_original_grant(grant)?;
        Ok(&self.root)
    }

    pub fn checked_packet(&self, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&CheckedProviderSchoolPacketV1> {
        self.require_original_grant(grant)?;
        Ok(&self.checked)
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
            CaptureOccurrence,
            CaptureManifest,
            CaptureInterpretationObject,
            ParserQualification,
            ScopePolicy,
            ExistingSchoolProgramAndConflictInventory,
            SchoolYearConvention,
            SemanticReview,
        ]
    }
}
