use super::*;

pub struct ActiveAttachmentSubjectDiscoveryV1<'a> {
    pub(super) rows: Vec<ActiveAttachmentSubjectV1<'a>>,
}
impl<'a> ActiveAttachmentSubjectDiscoveryV1<'a> {
    pub fn subjects(&self) -> &[ActiveAttachmentSubjectV1<'a>] {
        &self.rows
    }
}

pub struct ActiveAttachmentSubjectV1<'a> {
    pub(super) record: &'a ReviewedReferenceRecordV1,
    pub(super) subject: &'a RetainedDocumentSubjectReferenceV1,
    pub(super) capture: &'a Material,
    pub(super) locator: Locator<'a>,
    pub(super) context: &'a AttachmentObjectReferenceV1,
    pub(super) retained_root_anchors: Vec<&'a RetainedCaptureLeafV1>,
}

pub(super) enum Locator<'a> {
    Inline {
        reference: &'a AttachmentObjectReferenceV1,
        bytes: Vec<u8>,
    },
    Retained(&'a RetainedCaptureLeafV1),
}

pub enum DiscoveredAttachmentCaptureV1<'a> {
    Inline(&'a AttachmentObjectReferenceV1),
    Retained(&'a RetainedCaptureLeafV1),
}
pub enum DiscoveredAttachmentLocatorV1<'a> {
    Inline {
        reference: &'a AttachmentObjectReferenceV1,
        bytes: &'a [u8],
    },
    Retained(&'a RetainedCaptureLeafV1),
}

impl ActiveAttachmentSubjectV1<'_> {
    pub fn record(&self) -> &ReviewedReferenceRecordV1 {
        self.record
    }
    pub fn subject(&self) -> &RetainedDocumentSubjectReferenceV1 {
        self.subject
    }
    pub fn context(&self) -> &AttachmentObjectReferenceV1 {
        self.context
    }
    pub fn retained_root_anchors(&self) -> &[&RetainedCaptureLeafV1] {
        &self.retained_root_anchors
    }
    pub fn capture(&self) -> DiscoveredAttachmentCaptureV1<'_> {
        match self.capture {
            Material::Inline { reference, .. } => DiscoveredAttachmentCaptureV1::Inline(reference),
            Material::Retained { leaf } => DiscoveredAttachmentCaptureV1::Retained(leaf),
        }
    }
    pub fn locator(&self) -> DiscoveredAttachmentLocatorV1<'_> {
        match &self.locator {
            Locator::Inline { reference, bytes } => {
                DiscoveredAttachmentLocatorV1::Inline { reference, bytes }
            }
            Locator::Retained(leaf) => DiscoveredAttachmentLocatorV1::Retained(leaf),
        }
    }
}
