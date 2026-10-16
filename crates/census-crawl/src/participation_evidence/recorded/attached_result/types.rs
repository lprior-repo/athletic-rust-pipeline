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
