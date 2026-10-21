use super::*;
use sha2::{Digest, Sha256};

#[derive(Default)]
pub(super) struct Objects {
    inline: Vec<(AttachmentObjectReferenceV1, Vec<u8>)>,
    retained: Vec<RetainedCaptureLeafV1>,
}

pub(super) fn reference(bytes: &[u8]) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(bytes)))?,
        bytes: u64::try_from(bytes.len())?,
    })
}

impl Objects {
    pub(super) fn bytes(&mut self, bytes: Vec<u8>) -> TestResult<AttachmentDigestV1> {
        let reference = reference(&bytes)?;
        if !self.inline.iter().any(|(held, _)| held == &reference) {
            self.inline.push((reference.clone(), bytes));
        }
        Ok(reference.digest)
    }

    pub(super) fn json(&mut self, value: &impl Serialize) -> TestResult<AttachmentDigestV1> {
        self.bytes(serialized_bytes(value)?)
    }

    pub(super) fn retained(&mut self, leaf: RetainedCaptureLeafV1) -> AttachmentDigestV1 {
        let digest = leaf.object.digest.clone();
        self.retained.push(leaf);
        digest
    }

    pub(super) fn references(&self) -> Vec<AttachmentObjectReferenceV1> {
        self.inline
            .iter()
            .map(|(reference, _)| reference.clone())
            .chain(self.retained.iter().map(|leaf| leaf.object.clone()))
            .collect()
    }

    pub(super) fn borrowed<'a>(
        &'a self,
        existing: &[AttachmentObjectReferenceV1],
    ) -> Vec<AttachmentContextObjectV1<'a>> {
        self.inline
            .iter()
            .filter(|(reference, _)| !existing.contains(reference))
            .map(|(reference, bytes)| AttachmentContextObjectV1::Inline { reference, bytes })
            .chain(
                self.retained
                    .iter()
                    .filter(|leaf| !existing.contains(&leaf.object))
                    .map(AttachmentContextObjectV1::Retained),
            )
            .collect()
    }
}
