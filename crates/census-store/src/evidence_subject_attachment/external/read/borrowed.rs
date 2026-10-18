use super::*;
use crate::read::with_retained_journal_entry_raw_bounded;
use std::cell::RefCell;

pub(crate) use super::super::leaf::capture::CheckedCapture;
pub(crate) use super::super::leaf::schema::{CheckedRoot, CheckedSchool};

pub(crate) fn with_checked<R>(
    view: &impl RetainedJournalReadSource,
    expected: &RetainedCaptureRootV1,
    grant: &mut AttachmentReplayGrantV1,
    visit: impl Fn(&RawValue, CheckedRoot<'_>, &mut AttachmentReplayGrantV1) -> StoreResult<R>,
) -> StoreResult<R> {
    grant.ensure_available().map_err(Error::Domain)?;
    let result = checked(view, expected, grant, visit);
    if result.is_err() {
        grant.poison();
    }
    result
}

fn checked<R>(
    view: &impl RetainedJournalReadSource,
    expected: &RetainedCaptureRootV1,
    grant: &mut AttachmentReplayGrantV1,
    visit: impl Fn(&RawValue, CheckedRoot<'_>, &mut AttachmentReplayGrantV1) -> StoreResult<R>,
) -> StoreResult<R> {
    let budget = super::inventory::budget(view, expected.schema, &expected.key, grant)?
        .ok_or(Error::Corrupt("external root is absent or ambiguous"))?;
    let grant = RefCell::new(grant);
    with_retained_journal_entry_raw_bounded(
        view,
        expected.schema.phase(),
        &expected.key,
        budget,
        |raw| {
            let mut grant = grant
                .try_borrow_mut()
                .map_err(|_| Error::Corrupt("nested borrowed root admission"))?;
            grant.charge_steps(2).map_err(Error::Domain)?;
            if expected.payload.bytes
                != u64::try_from(raw.get().len()).map_err(|_| StoreError::CounterOverflow)?
                || format!("{:x}", Sha256::digest(raw.get().as_bytes()))
                    != expected.payload.digest.as_str()
            {
                return Err(Error::Corrupt("external root payload reference differs").into());
            }
            let metadata = super::super::leaf::schema::checked(raw, expected.schema, &mut grant)?;
            visit(raw, metadata, &mut grant)
        },
    )?
    .ok_or_else(|| Error::Corrupt("external root disappeared from held view").into())
}
