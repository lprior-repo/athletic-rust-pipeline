use super::source::TestResult;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
};
use census_domain::model::serialized_bytes;
use census_domain::participation_candidates::MAX_PARTICIPATION_FACTS;
use census_store::{AttachmentContextObjectV1, RetainedCaptureLeafV1, Store};
use serde::Serialize;
use sha2::{Digest, Sha256};

enum Held {
    Inline {
        reference: AttachmentObjectReferenceV1,
        bytes: Vec<u8>,
    },
    Retained(RetainedCaptureLeafV1),
}

impl Held {
    fn reference(&self) -> &AttachmentObjectReferenceV1 {
        match self {
            Self::Inline { reference, .. } => reference,
            Self::Retained(leaf) => &leaf.object,
        }
    }

    fn borrowed(&self) -> AttachmentContextObjectV1<'_> {
        match self {
            Self::Inline { reference, bytes } => {
                AttachmentContextObjectV1::Inline { reference, bytes }
            }
            Self::Retained(leaf) => AttachmentContextObjectV1::Retained(leaf),
        }
    }
}

#[derive(Default)]
pub(super) struct Objects {
    held: Vec<Held>,
}

pub(super) fn reference(bytes: &[u8]) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(bytes)))?,
        bytes: u64::try_from(bytes.len())?,
    })
}

impl Objects {
    pub(super) fn bytes(&mut self, bytes: Vec<u8>) -> TestResult<AttachmentDigestV1> {
        self.insert(Held::Inline {
            reference: reference(&bytes)?,
            bytes,
        })
    }

    pub(super) fn json(&mut self, value: &impl Serialize) -> TestResult<AttachmentDigestV1> {
        self.bytes(serialized_bytes(value)?)
    }

    pub(super) fn retained(
        &mut self,
        leaf: RetainedCaptureLeafV1,
    ) -> TestResult<AttachmentDigestV1> {
        self.insert(Held::Retained(leaf))
    }

    pub(super) fn references(&self) -> TestResult<Vec<AttachmentObjectReferenceV1>> {
        let mut references = Vec::new();
        references.try_reserve_exact(self.held.len())?;
        references.extend(self.held.iter().map(|held| held.reference().clone()));
        Ok(references)
    }

    pub(super) fn borrowed_new(
        &self,
        store: &Store,
    ) -> TestResult<Vec<AttachmentContextObjectV1<'_>>> {
        let snapshot = store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let prior = snapshot.read_evidence_subject_attachment_references_v1(&mut grant)?;
        let mut borrowed = Vec::new();
        borrowed.try_reserve_exact(self.held.len())?;
        for held in &self.held {
            let known = prior.records().iter().any(|record| {
                record
                    .command
                    .reviewed_context
                    .objects
                    .as_slice()
                    .contains(held.reference())
            });
            if !known {
                borrowed.push(held.borrowed());
            }
        }
        Ok(borrowed)
    }

    fn insert(&mut self, material: Held) -> TestResult<AttachmentDigestV1> {
        let reference = material.reference().clone();
        match self
            .held
            .binary_search_by(|held| held.reference().digest.cmp(&reference.digest))
        {
            Ok(position) => {
                let existing = self
                    .held
                    .get(position)
                    .ok_or("context object position missing")?;
                if existing.reference() != &reference {
                    return Err("context object reference conflict".into());
                }
                if let (
                    Held::Inline { bytes: held, .. },
                    Held::Inline {
                        bytes: supplied, ..
                    },
                ) = (existing, &material)
                {
                    if held != supplied {
                        return Err("context object material conflict".into());
                    }
                }
            }
            Err(position) => {
                if self.held.len() >= MAX_PARTICIPATION_FACTS {
                    return Err("context object capacity".into());
                }
                self.held.try_reserve_exact(1)?;
                self.held.insert(position, material);
            }
        }
        Ok(reference.digest)
    }
}
