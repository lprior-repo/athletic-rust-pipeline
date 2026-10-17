use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use fjall::{Keyspace, Readable, Snapshot};

use super::{PublicIdentityError, DECLARATION};
use crate::{EvidenceSubjectAttachmentStoreError, StoreError, StoreResult};

pub(crate) fn retained_qualification_declaration_v1(
    snapshot: &Snapshot,
    meta: &Keyspace,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    grant
        .ensure_available()
        .map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
    let result = read(snapshot, meta, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn read(
    snapshot: &Snapshot,
    meta: &Keyspace,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<u8>> {
    grant
        .charge_steps(1)
        .map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
    let size = snapshot
        .size_of(meta, DECLARATION)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(PublicIdentityError::Corrupt(
            "missing qualification declaration",
        ))?;
    let bytes = usize::try_from(size).map_err(|_| StoreError::CounterOverflow)?;
    if bytes > 65_536 {
        return Err(StoreError::RowTooLarge { bytes, max: 65_536 });
    }
    let work_bytes = bytes
        .checked_mul(2)
        .and_then(|value| value.checked_add(DECLARATION.len()))
        .ok_or(StoreError::CounterOverflow)?;
    grant
        .charge_bytes(work_bytes)
        .map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
    grant
        .charge_steps(2)
        .map_err(EvidenceSubjectAttachmentStoreError::Domain)?;
    let raw = snapshot
        .get(meta, DECLARATION)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(PublicIdentityError::Corrupt(
            "sized declaration disappeared",
        ))?;
    if raw.len() != bytes {
        return Err(PublicIdentityError::Corrupt("declaration size changed").into());
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| PublicIdentityError::Corrupt("declaration allocation refused"))?;
    output.extend_from_slice(&raw);
    Ok(output)
}
