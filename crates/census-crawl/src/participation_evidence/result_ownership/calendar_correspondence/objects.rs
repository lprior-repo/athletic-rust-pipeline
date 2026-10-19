use crate::CrawlResult;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentListV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
    EvidenceSubjectAttachmentError,
};
use census_domain::result_calendar_correspondence::{
    MAX_RESULT_CALENDAR_OBJECTS, MAX_RESULT_CALENDAR_RECORD_BYTES,
};
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
        reserve(&mut inline, MAX_RESULT_CALENDAR_OBJECTS, grant)?;
        reserve(&mut retained, MAX_RESULT_CALENDAR_OBJECTS, grant)?;
        Ok(Self { inline, retained })
    }

    pub(super) fn json(
        &mut self,
        value: &impl Serialize,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<AttachmentObjectReferenceV1> {
        let bytes =
            crate::participation_evidence::codec::encode(value, MAX_RESULT_CALENDAR_RECORD_BYTES)
                .map_err(crate::CrawlError::from)?;
        self.owned(bytes, grant)
    }

    pub(super) fn bytes(
        &mut self,
        bytes: &[u8],
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<AttachmentObjectReferenceV1> {
        let reference = object_reference(bytes, grant)?;
        if let Some((held, value)) = self
            .inline
            .iter()
            .find(|(held, _)| held.digest == reference.digest)
        {
            if held != &reference || value != bytes {
                return Err(invalid("calendar object digest collision"));
            }
            return Ok(reference);
        }
        self.ensure_capacity()?;
        let mut owned = Vec::new();
        owned
            .try_reserve_exact(bytes.len())
            .map_err(|_| domain(EvidenceSubjectAttachmentError::Allocation))?;
        owned.extend_from_slice(bytes);
        self.inline.push((reference.clone(), owned));
        Ok(reference)
    }

    pub(super) fn owned(
        &mut self,
        bytes: Vec<u8>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<AttachmentObjectReferenceV1> {
        self.insert(bytes, grant)
    }

    fn insert(
        &mut self,
        bytes: Vec<u8>,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<AttachmentObjectReferenceV1> {
        let reference = object_reference(&bytes, grant)?;
        if let Some((held, value)) = self
            .inline
            .iter()
            .find(|(held, _)| held.digest == reference.digest)
        {
            if held != &reference || value != &bytes {
                return Err(invalid("calendar object digest collision"));
            }
            return Ok(reference);
        }
        self.ensure_capacity()?;
        self.inline.push((reference.clone(), bytes));
        Ok(reference)
    }
    pub(super) fn contains(&self, reference: &AttachmentObjectReferenceV1) -> bool {
        self.inline.iter().any(|(held, _)| held == reference)
            || self.retained.iter().any(|leaf| leaf.object == *reference)
    }

    pub(super) fn retain(
        &mut self,
        leaf: RetainedCaptureLeafV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<()> {
        if self.retained.iter().any(|held| held == &leaf) {
            return Ok(());
        }
        self.ensure_capacity()?;
        grant.charge_steps(1).map_err(domain)?;
        self.retained.push(leaf);
        Ok(())
    }

    pub(super) fn references(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<AttachmentListV1<AttachmentObjectReferenceV1>> {
        let count = self
            .inline
            .len()
            .checked_add(self.retained.len())
            .ok_or(census_store::StoreError::CounterOverflow)?;
        let mut references = Vec::new();
        reserve(&mut references, count, grant)?;
        self.inline
            .iter()
            .try_for_each(|(reference, _)| push_reference(&mut references, reference))?;
        self.retained
            .iter()
            .try_for_each(|leaf| push_reference(&mut references, &leaf.object))?;
        references.sort_by(|left, right| left.digest.cmp(&right.digest));
        AttachmentListV1::new(references).map_err(domain)
    }

    pub(super) fn borrowed(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> CrawlResult<Vec<AttachmentContextObjectV1<'_>>> {
        let count = self
            .inline
            .len()
            .checked_add(self.retained.len())
            .ok_or(census_store::StoreError::CounterOverflow)?;
        let mut context = Vec::new();
        reserve(&mut context, count, grant)?;
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

    fn ensure_capacity(&self) -> CrawlResult<()> {
        let count = self
            .inline
            .len()
            .checked_add(self.retained.len())
            .ok_or(census_store::StoreError::CounterOverflow)?;
        if count >= MAX_RESULT_CALENDAR_OBJECTS {
            return Err(crate::CrawlError::Resource {
                resource: "calendar source object inventory",
                requested: count
                    .checked_add(1)
                    .ok_or(census_store::StoreError::CounterOverflow)?,
                limit: MAX_RESULT_CALENDAR_OBJECTS,
            });
        }
        Ok(())
    }
}

fn push_reference(
    references: &mut Vec<AttachmentObjectReferenceV1>,
    reference: &AttachmentObjectReferenceV1,
) -> CrawlResult<()> {
    if let Some(previous) = references
        .iter()
        .find(|item| item.digest == reference.digest)
    {
        if previous != reference {
            return Err(invalid("calendar object length collision"));
        }
        return Ok(());
    }
    references.push(reference.clone());
    Ok(())
}

pub(super) fn object_reference(
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

pub(super) fn digest(
    bytes: &[u8],
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<AttachmentDigestV1> {
    Ok(object_reference(bytes, grant)?.digest)
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
        .map_err(|_| domain(EvidenceSubjectAttachmentError::Allocation))?;
    Ok(())
}

pub(super) fn guarded<T>(
    grant: &mut AttachmentReplayGrantV1,
    action: impl FnOnce(&mut AttachmentReplayGrantV1) -> CrawlResult<T>,
) -> CrawlResult<T> {
    grant.ensure_available().map_err(domain)?;
    let result = action(grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

pub(super) fn invalid(detail: &'static str) -> crate::CrawlError {
    crate::participation_evidence::invalid(detail).into()
}

pub(super) fn domain(error: EvidenceSubjectAttachmentError) -> crate::CrawlError {
    census_store::StoreError::from(census_store::EvidenceSubjectAttachmentStoreError::Domain(
        error,
    ))
    .into()
}
