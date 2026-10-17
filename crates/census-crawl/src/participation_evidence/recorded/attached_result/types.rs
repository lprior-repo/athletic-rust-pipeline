use crate::participation_evidence::result_ownership::{
    CheckedRetainedResultDocumentV1, CheckedRetainedResultRowV1,
};
use census_store::{CurrentEvidenceSubjectAttachmentV1, RetainedCaptureRootV1, StoreSnapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachedResultGapV1 {
    SchoolBindingUnresolved,
}

pub struct IdentityLinkedHtmlResultDocumentV1<'a, 's> {
    pub(super) document: &'a CheckedRetainedResultDocumentV1<'a>,
    pub(super) root: &'a RetainedCaptureRootV1,
    pub(super) snapshot: &'a StoreSnapshot<'s>,
    pub(super) attachments: Vec<CurrentEvidenceSubjectAttachmentV1<'a, 's>>,
}

impl<'a, 's> IdentityLinkedHtmlResultDocumentV1<'a, 's> {
    pub fn checked_document(&self) -> &CheckedRetainedResultDocumentV1<'a> {
        self.document
    }
    pub fn recorded_root(&self) -> &RetainedCaptureRootV1 {
        self.root
    }
    pub fn snapshot(&self) -> &StoreSnapshot<'s> {
        self.snapshot
    }
    pub fn row_count(&self) -> usize {
        self.attachments.len()
    }
    pub fn row(&self, index: usize) -> Option<IdentityLinkedHtmlResultRowV1<'_, 's>> {
        Some(IdentityLinkedHtmlResultRowV1 {
            source: self.document.row(index)?,
            attachment: self.attachments.get(index)?,
        })
    }
}

pub struct IdentityLinkedHtmlResultRowV1<'a, 's> {
    source: CheckedRetainedResultRowV1<'a>,
    attachment: &'a CurrentEvidenceSubjectAttachmentV1<'a, 's>,
}

impl<'a, 's> IdentityLinkedHtmlResultRowV1<'a, 's> {
    pub fn checked_row(&self) -> &CheckedRetainedResultRowV1<'a> {
        &self.source
    }
    pub fn attachment(&self) -> &CurrentEvidenceSubjectAttachmentV1<'a, 's> {
        self.attachment
    }
    pub fn gaps(&self) -> &'static [AttachedResultGapV1] {
        &[AttachedResultGapV1::SchoolBindingUnresolved]
    }
}

pub struct DiscoveredIdentityLinkedHtmlResultDocumentV1<'a, 's> {
    pub(super) document: &'a CheckedRetainedResultDocumentV1<'a>,
    pub(super) root: &'a RetainedCaptureRootV1,
    pub(super) attachments: super::view::Bindings<'a, 's>,
    pub(super) venue: std::sync::OnceLock<String>,
}

impl<'a, 's> DiscoveredIdentityLinkedHtmlResultDocumentV1<'a, 's> {
    pub fn checked_document(&self) -> &CheckedRetainedResultDocumentV1<'a> {
        self.document
    }
    pub fn recorded_root(&self) -> &RetainedCaptureRootV1 {
        self.root
    }
    pub fn row_count(&self) -> usize {
        match &self.attachments {
            super::view::Bindings::Current(rows) => rows.len(),
            super::view::Bindings::Frozen(rows) => rows.len(),
        }
    }
    pub fn row(&self, index: usize) -> Option<DiscoveredIdentityLinkedHtmlResultRowV1<'_, 's>> {
        let attachment = match &self.attachments {
            super::view::Bindings::Current(rows) => Attachment::Current(rows.get(index)?),
            super::view::Bindings::Frozen(rows) => Attachment::Frozen(rows.get(index)?),
        };
        Some(DiscoveredIdentityLinkedHtmlResultRowV1 {
            source: self.document.row(index)?,
            attachment: DiscoveredResultAttachmentV1 { inner: attachment },
        })
    }
}

pub struct DiscoveredIdentityLinkedHtmlResultRowV1<'a, 's> {
    source: CheckedRetainedResultRowV1<'a>,
    attachment: DiscoveredResultAttachmentV1<'a, 's>,
}

impl<'a, 's> DiscoveredIdentityLinkedHtmlResultRowV1<'a, 's> {
    pub fn checked_row(&self) -> &CheckedRetainedResultRowV1<'a> {
        &self.source
    }
    pub fn attachment(&self) -> &DiscoveredResultAttachmentV1<'a, 's> {
        &self.attachment
    }
    pub fn gaps(&self) -> &'static [AttachedResultGapV1] {
        &[AttachedResultGapV1::SchoolBindingUnresolved]
    }
}

pub struct DiscoveredResultAttachmentV1<'a, 's> {
    inner: Attachment<'a, 's>,
}

enum Attachment<'a, 's> {
    Current(&'a CurrentEvidenceSubjectAttachmentV1<'a, 's>),
    Frozen(&'a census_store::FrozenCurrentEvidenceSubjectAttachmentV1<'a, 's>),
}

impl DiscoveredResultAttachmentV1<'_, '_> {
    pub fn subject(
        &self,
    ) -> &census_domain::evidence_subject_attachment::RetainedDocumentSubjectReferenceV1 {
        match &self.inner {
            Attachment::Current(value) => value.subject(),
            Attachment::Frozen(value) => value.subject(),
        }
    }
    pub fn target(&self) -> &census_domain::evidence_subject_attachment::AttachmentTargetV1 {
        match &self.inner {
            Attachment::Current(value) => value.target(),
            Attachment::Frozen(value) => value.target(),
        }
    }
    pub fn record(&self) -> &census_domain::evidence_subject_attachment::ReviewedReferenceRecordV1 {
        match &self.inner {
            Attachment::Current(value) => value.record(),
            Attachment::Frozen(value) => value.record(),
        }
    }
    pub fn receipt(&self) -> &census_store::Receipt {
        match &self.inner {
            Attachment::Current(value) => value.receipt(),
            Attachment::Frozen(value) => value.receipt(),
        }
    }
    pub fn inventory(&self) -> &census_domain::evidence_subject_attachment::AttachmentInventoryV1 {
        match &self.inner {
            Attachment::Current(value) => value.inventory(),
            Attachment::Frozen(value) => value.inventory(),
        }
    }
    pub fn store_identity(&self) -> &census_domain::model::StoreIdentity {
        match &self.inner {
            Attachment::Current(value) => value.store_identity(),
            Attachment::Frozen(value) => value.store_identity(),
        }
    }
    pub fn accepted_public_head(&self) -> &census_domain::public_identity_history::RegistryHead {
        match &self.inner {
            Attachment::Current(value) => value.accepted_public_head(),
            Attachment::Frozen(value) => value.accepted_public_head(),
        }
    }
    pub fn snapshot_sequence(&self) -> u64 {
        match &self.inner {
            Attachment::Current(value) => value.snapshot().sequence(),
            Attachment::Frozen(value) => value.snapshot_sequence(),
        }
    }
    pub fn require_original_grant(
        &self,
        grant: &mut census_domain::evidence_subject_attachment::AttachmentReplayGrantV1,
    ) -> census_store::StoreResult<()> {
        match &self.inner {
            Attachment::Current(value) => value.require_original_grant(grant),
            Attachment::Frozen(value) => value.require_original_grant(grant),
        }
    }
}
