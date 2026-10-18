use super::*;
use census_domain::evidence_subject_attachment::{AttachmentInventoryV1, AttachmentTargetV1};
use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::RegistryHead;
use census_store::{CurrentEvidenceSubjectAttachmentV1, FrozenCurrentEvidenceSubjectAttachmentV1};

pub struct AttachedParticipationV1<'a, 's> {
    pub(super) sources: Vec<CheckedAttachedStatusV1<'a, 's>>,
    pub(super) windows: Vec<CheckedAttachedWindowV1<'a, 's>>,
    pub(super) unsupported: Vec<StatusBinding<'a, 's>>,
}

pub(super) struct CheckedAttachedStatusV1<'a, 's> {
    pub(super) recorded: super::super::RecordedIndependentParticipationV1,
    pub(super) binding: StatusBinding<'a, 's>,
}

pub(super) struct CheckedAttachedWindowV1<'a, 's> {
    pub(super) recorded: super::super::RecordedIndependentParticipationWindowV1,
    pub(super) binding: StatusBinding<'a, 's>,
}

pub(super) enum StatusBinding<'a, 's> {
    Current(CurrentEvidenceSubjectAttachmentV1<'a, 's>),
    Frozen(FrozenCurrentEvidenceSubjectAttachmentV1<'a, 's>),
}

impl StatusBinding<'_, '_> {
    pub(super) fn require_original_grant(
        &self,
        grant: &mut Grant,
    ) -> census_store::StoreResult<()> {
        match self {
            Self::Current(value) => value.require_original_grant(grant),
            Self::Frozen(value) => value.require_original_grant(grant),
        }
    }
    pub(super) fn target(&self) -> &AttachmentTargetV1 {
        match self {
            Self::Current(value) => value.target(),
            Self::Frozen(value) => value.target(),
        }
    }
    pub(super) fn inventory(&self) -> &AttachmentInventoryV1 {
        match self {
            Self::Current(value) => value.inventory(),
            Self::Frozen(value) => value.inventory(),
        }
    }
    pub(super) fn store_identity(&self) -> &StoreIdentity {
        match self {
            Self::Current(value) => value.store_identity(),
            Self::Frozen(value) => value.store_identity(),
        }
    }
    pub(super) fn accepted_public_head(&self) -> &RegistryHead {
        match self {
            Self::Current(value) => value.accepted_public_head(),
            Self::Frozen(value) => value.accepted_public_head(),
        }
    }
    pub(super) fn snapshot_sequence(&self) -> u64 {
        match self {
            Self::Current(value) => value.snapshot().sequence(),
            Self::Frozen(value) => value.snapshot_sequence(),
        }
    }
    pub(super) fn record_digest(&self) -> &str {
        match self {
            Self::Current(value) => value.record().record_digest.as_str(),
            Self::Frozen(value) => value.record().record_digest.as_str(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentaryCohortAgreementV1 {
    Agreed,
    Conflicting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentarySeasonApplicabilityV1 {
    Exact,
    CompetitionMismatch,
    VenueMismatch,
    CompetitionAndVenueMismatch,
}

pub struct CheckedDocumentaryParticipationFactsV1<'a> {
    pub(super) sources: &'a [CheckedDocumentarySourceFactsV1<'a>],
    pub(super) windows: &'a [CheckedDocumentaryWindowFactsV1<'a>],
}

impl<'a> CheckedDocumentaryParticipationFactsV1<'a> {
    pub fn sources(&self) -> &[CheckedDocumentarySourceFactsV1<'a>] {
        self.sources
    }

    pub fn windows(&self) -> &[CheckedDocumentaryWindowFactsV1<'a>] {
        self.windows
    }
}

pub struct CheckedDocumentarySourceFactsV1<'a> {
    pub(super) source: &'a VerifiedIndependentParticipationV1,
    pub(super) record_digest: &'a str,
    pub(super) season: DocumentarySeasonApplicabilityV1,
    pub(super) cohort: DocumentaryCohortAgreementV1,
}

impl CheckedDocumentarySourceFactsV1<'_> {
    pub fn source(&self) -> &VerifiedIndependentParticipationV1 {
        self.source
    }
    pub fn attachment_record_digest(&self) -> &str {
        self.record_digest
    }
    pub fn cohort_agreement(&self) -> DocumentaryCohortAgreementV1 {
        self.cohort
    }
    pub fn season(&self) -> DocumentarySeasonApplicabilityV1 {
        self.season
    }
}

pub struct CheckedDocumentaryWindowFactsV1<'a> {
    pub(super) source: &'a crate::participation_evidence::VerifiedIndependentParticipationWindowV1,
    pub(super) record_digest: &'a str,
    pub(super) cohort: DocumentaryCohortAgreementV1,
}

impl CheckedDocumentaryWindowFactsV1<'_> {
    pub fn source(
        &self,
    ) -> &crate::participation_evidence::VerifiedIndependentParticipationWindowV1 {
        self.source
    }

    pub fn attachment_record_digest(&self) -> &str {
        self.record_digest
    }

    pub fn cohort_agreement(&self) -> DocumentaryCohortAgreementV1 {
        self.cohort
    }
}
