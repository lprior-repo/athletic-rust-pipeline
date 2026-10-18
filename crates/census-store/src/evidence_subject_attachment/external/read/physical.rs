use super::super::super::EvidenceSubjectAttachmentStoreError as Error;
use super::super::RetainedCaptureRootSchemaV1;
use crate::read::{
    for_each_retained_journal_root_with_admission, read_retained_journal_entry_raw_bounded,
    JournalNamespace, JournalRawEnvelope, RetainedJournalReadSource,
};
use crate::{JournalReadBudget, ReadBudget, StoreError, StoreResult, MAX_JOURNAL_VALUE_BYTES};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;

use super::super::MAX_RETAINED_CAPTURE_ENVELOPE_BYTES;
const MAX_PHYSICAL_ROWS: u64 = 6;
const MAX_PHYSICAL_BYTES: u64 = 4 * 1024 * 1024;
const DECODER_PASS_SLOTS: usize = 17;

pub(super) fn read(
    view: &impl RetainedJournalReadSource,
    schema: RetainedCaptureRootSchemaV1,
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<JournalRawEnvelope>> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = read_checked(view, schema, key, grant);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn read_checked(
    view: &impl RetainedJournalReadSource,
    schema: RetainedCaptureRootSchemaV1,
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<JournalRawEnvelope>> {
    let Some(budget) = budget(view, schema, key, grant)? else {
        return Ok(None);
    };
    let envelope = read_retained_journal_entry_raw_bounded(view, schema.phase(), key, budget)?
        .ok_or(Error::Corrupt("external root disappeared from held view"))?;
    Ok(Some(envelope))
}

pub(super) fn budget(
    view: &impl RetainedJournalReadSource,
    schema: RetainedCaptureRootSchemaV1,
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Option<JournalReadBudget>> {
    steps(grant, 1)?;
    if key.len() != 64
        || !key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Corrupt("external root descriptor bounds").into());
    }
    steps(grant, 2)?;
    let inventory = for_each_retained_journal_root_with_admission(
        view,
        JournalNamespace::new(schema.phase(), key)?,
        ReadBudget {
            rows: MAX_PHYSICAL_ROWS,
            bytes: MAX_PHYSICAL_BYTES,
            record_bytes: MAX_JOURNAL_VALUE_BYTES,
        },
        grant,
        |grant| steps(grant, 4),
        |grant, found| {
            steps(grant, 1)?;
            if found != key {
                return Err(Error::Corrupt("external root prefix has a foreign root").into());
            }
            Ok(())
        },
    )?;
    if inventory.logical_roots() == 0 {
        return Ok(None);
    }
    if inventory.logical_roots() != 1 {
        return Err(Error::Corrupt("external root is absent or ambiguous").into());
    }
    let physical = inventory.contents();
    let passes = usize::try_from(physical.physical_rows())
        .map_err(|_| StoreError::CounterOverflow)?
        .checked_add(1)
        .and_then(|n| n.checked_mul(DECODER_PASS_SLOTS))
        .ok_or(StoreError::CounterOverflow)?;
    steps(grant, passes)?;
    let measured =
        usize::try_from(physical.encoded_bytes()).map_err(|_| StoreError::CounterOverflow)?;
    JournalReadBudget::new(measured.min(MAX_RETAINED_CAPTURE_ENVELOPE_BYTES)).map(Some)
}

fn steps(grant: &mut AttachmentReplayGrantV1, count: usize) -> StoreResult<()> {
    grant
        .charge_steps(count)
        .map_err(|source| Error::Domain(source).into())
}

#[cfg(test)]
mod tests;
