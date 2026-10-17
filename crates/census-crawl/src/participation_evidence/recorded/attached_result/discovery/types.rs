use crate::participation_evidence::result_ownership::discovery::OriginalResultSpan;
use census_store::ActiveAttachmentSubjectV1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentHtmlResultDiscoveryGapV1 {
    UnsupportedCaptureMaterial,
    UnsupportedCaptureSchema,
    UnsupportedCaptureSlot,
    UnsupportedCaptureKind,
    UnsupportedLocatorMaterial,
    UnsupportedLocatorSyntax,
}

pub struct UnresolvedAttachmentHtmlResultV1<'a, 'm> {
    pub(super) source: &'a ActiveAttachmentSubjectV1<'m>,
    pub(super) reason: AttachmentHtmlResultDiscoveryGapV1,
}

impl<'a, 'm> UnresolvedAttachmentHtmlResultV1<'a, 'm> {
    pub fn source(&self) -> &ActiveAttachmentSubjectV1<'m> {
        self.source
    }
    pub fn reason(&self) -> AttachmentHtmlResultDiscoveryGapV1 {
        self.reason
    }
}

pub(super) struct Diagnostic {
    pub(super) index: usize,
    pub(super) reason: AttachmentHtmlResultDiscoveryGapV1,
}

#[derive(Clone, Copy)]
pub(super) struct Member {
    pub(super) index: usize,
    pub(super) span: OriginalResultSpan,
}
