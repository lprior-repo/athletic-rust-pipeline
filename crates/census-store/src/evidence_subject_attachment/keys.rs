use super::{
    EvidenceSubjectAttachmentStoreError as Error, MAX_OPERATION_BYTES, OPERATION_PREFIX,
    RECORD_PREFIX,
};
use crate::{StoreError, StoreResult};
use census_domain::evidence_subject_attachment::{
    AttachmentTextV1, EvidenceSubjectAttachmentError,
};
use census_domain::public_identity_history::ReplayLimits;
use std::fmt::Write;

pub(super) fn record(ordinal: u64) -> StoreResult<String> {
    require_ordinal(ordinal)?;
    let bytes = RECORD_PREFIX
        .len()
        .checked_add(20)
        .ok_or(StoreError::CounterOverflow)?;
    let mut key = allocate(bytes)?;
    write!(&mut key, "{RECORD_PREFIX}{ordinal:020}")
        .map_err(|_| Error::Corrupt("record key encoding"))?;
    Ok(key)
}

pub(super) fn ordinal(key: &str) -> StoreResult<u64> {
    let suffix = key
        .strip_prefix(RECORD_PREFIX)
        .ok_or(Error::Corrupt("unknown attachment record key"))?;
    if suffix.len() != 20 || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(Error::Corrupt("noncanonical attachment record ordinal").into());
    }
    let ordinal = suffix
        .parse::<u64>()
        .map_err(|_| Error::Corrupt("attachment record ordinal overflow"))?;
    require_ordinal(ordinal)?;
    Ok(ordinal)
}

pub(super) fn receipt(operation: &AttachmentTextV1) -> StoreResult<String> {
    let bytes = OPERATION_PREFIX
        .len()
        .checked_add(operation.as_str().len())
        .ok_or(StoreError::CounterOverflow)?;
    if bytes > MAX_OPERATION_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes,
            max: MAX_OPERATION_BYTES,
        });
    }
    let mut key = allocate(bytes)?;
    key.push_str(OPERATION_PREFIX);
    key.push_str(operation.as_str());
    Ok(key)
}

fn require_ordinal(ordinal: u64) -> StoreResult<()> {
    let maximum =
        u64::try_from(ReplayLimits::SMALL.records).map_err(|_| StoreError::CounterOverflow)?;
    if ordinal == 0 || ordinal > maximum {
        return Err(Error::Corrupt("attachment record ordinal range").into());
    }
    Ok(())
}

fn allocate(bytes: usize) -> StoreResult<String> {
    let mut value = String::new();
    value
        .try_reserve_exact(bytes)
        .map_err(|_| Error::Domain(EvidenceSubjectAttachmentError::Allocation))?;
    Ok(value)
}
