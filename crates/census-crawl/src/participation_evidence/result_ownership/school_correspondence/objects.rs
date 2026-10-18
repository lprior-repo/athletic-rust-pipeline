use super::{domain, invalid};
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentListV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
    EvidenceSubjectAttachmentError,
};
use census_domain::result_school_correspondence::MAX_RESULT_SCHOOL_OBJECTS;
use census_store::{AttachmentContextObjectV1, RetainedCaptureLeafV1};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub(super) struct Objects {
    inline: Vec<(AttachmentObjectReferenceV1, Vec<u8>)>,
    retained: Vec<RetainedCaptureLeafV1>,
}

impl Objects {
    pub(super) fn new(grant: &mut AttachmentReplayGrantV1) -> CrawlResult<Self> {
        let mut inline = Vec::new();
        let mut retained = Vec::new();
        reserve(&mut inline, 6, grant)?;
        reserve(&mut retained, 1, grant)?;
        Ok(Self { inline, retained })
    }

    pub(super) fn json(
        &mut self,
        value: &(impl Serialize + ?Sized),
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<AttachmentObjectReferenceV1> {
        let bytes = census_domain::model::serialized_bytes(value).map_err(|source| {
            crate::CrawlError::Canonical {
                table: "result school source object".into(),
                source,
            }
        })?;
        crate::participation_evidence::bound(
            bytes.len(),
            crate::participation_evidence::MAX_PACKET,
            "result school source object",
        )?;
        self.insert(bytes, grant)
    }

    pub(super) fn bytes(
        &mut self,
        bytes: &[u8],
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<AttachmentObjectReferenceV1> {
        grant.charge_bytes(bytes.len()).map_err(domain)?;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(bytes.len())
            .map_err(|_| domain(EvidenceSubjectAttachmentError::Allocation))?;
        owned.extend_from_slice(bytes);
        self.insert(owned, grant)
    }

    fn insert(
        &mut self,
        bytes: Vec<u8>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<AttachmentObjectReferenceV1> {
        let reference = reference(&bytes, grant)?;
        if let Some((existing, held)) = self
            .inline
            .iter()
            .find(|(held, _)| held.digest == reference.digest)
        {
            if existing != &reference || held != &bytes {
                return Err(invalid("source object digest collision"));
            }
            return Ok(reference);
        }
        if self.inline.len() >= 6 {
            return Err(invalid("source inline object capacity"));
        }
        self.inline.push((reference.clone(), bytes));
        Ok(reference)
    }

    pub(super) fn retain(&mut self, leaf: RetainedCaptureLeafV1) -> CrawlResult<()> {
        if !self.retained.is_empty() {
            return Err(invalid("source body leaf capacity"));
        }
        self.retained.push(leaf);
        Ok(())
    }

    pub(super) fn references(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<AttachmentListV1<AttachmentObjectReferenceV1>> {
        let mut references = Vec::new();
        reserve(&mut references, self.count()?, grant)?;
        references.extend(self.inline.iter().map(|(reference, _)| reference.clone()));
        references.extend(self.retained.iter().map(|leaf| leaf.object.clone()));
        references.sort_by(|left, right| left.digest.cmp(&right.digest));
        if references
            .windows(2)
            .any(|pair| matches!(pair, [left, right] if left.digest == right.digest))
        {
            return Err(invalid("duplicate source evidence object"));
        }
        AttachmentListV1::new(references).map_err(domain)
    }

    pub(super) fn borrowed(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<Vec<AttachmentContextObjectV1<'_>>> {
        let mut context = Vec::new();
        reserve(&mut context, self.count()?, grant)?;
        context.extend(
            self.inline
                .iter()
                .map(|(reference, bytes)| AttachmentContextObjectV1::Inline { reference, bytes }),
        );
        context.extend(
            self.retained
                .iter()
                .map(AttachmentContextObjectV1::Retained),
        );
        Ok(context)
    }

    fn count(&self) -> CrawlResult<usize> {
        let count = self
            .inline
            .len()
            .checked_add(self.retained.len())
            .ok_or(census_store::StoreError::CounterOverflow)?;
        if count > MAX_RESULT_SCHOOL_OBJECTS {
            return Err(invalid("source object count"));
        }
        Ok(count)
    }
}

pub(super) fn reference(
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<AttachmentObjectReferenceV1> {
    grant.charge_bytes(bytes.len()).map_err(domain)?;
    grant.charge_steps(1).map_err(domain)?;
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(bytes)))
            .map_err(domain)?,
        bytes: u64::try_from(bytes.len()).map_err(|_| census_store::StoreError::CounterOverflow)?,
    })
}

fn reserve<T>(
    values: &mut Vec<T>,
    count: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<()> {
    let bytes = count
        .checked_mul(std::mem::size_of::<T>())
        .ok_or(census_store::StoreError::CounterOverflow)?;
    grant.charge_bytes(bytes).map_err(domain)?;
    values
        .try_reserve_exact(count)
        .map_err(|_| domain(EvidenceSubjectAttachmentError::Allocation))
}
