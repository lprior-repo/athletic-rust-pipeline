use super::super::super::codec;
use super::super::super::read;
use super::super::super::wire;
use super::super::super::*;
use crate::{StoreError, StoreRevision, StoreSnapshot};
use census_domain::model::StoreIdentity;
use fjall::{Keyspace, Readable, Snapshot};

pub(super) struct InspectedMetadata<'s> {
    pub(super) revision: StoreRevision<'s>,
    pub(super) store: StoreIdentity,
    pub(super) inventory: ResultSchoolInventoryV1,
}

pub(super) fn bounded<'s>(
    view: &StoreSnapshot<'s>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<InspectedMetadata<'s>> {
    view.with_roster_source_snapshot_v1(|snapshot, meta, journal, receipts| {
        let format = read::snapshot_format(snapshot, meta, grant)?;
        if format.lifecycle_revision != Some(super::super::super::LIFECYCLE)
            || format.schema_version != Some(2)
            || format.key_format != Some(1)
            || format.migration_target.is_some()
        {
            return Err(Error::EnrollmentRequired.into());
        }
        let proof = read::load(snapshot, meta, journal, receipts, &format, grant)?
            .ok_or(Error::EnrollmentRequired)?;
        let enrollment: wire::Enrollment = codec::decode(&proof.enrollment, grant)?;
        let raw = read::value(snapshot, meta, wire::HEAD.as_bytes(), grant)?;
        let head: wire::AuthorityHead = codec::decode(&raw, grant)?;
        if head.schema != "result-school-authority-head-v1"
            || head.inventory.version != 1
            || head.inventory.records
                > u64::try_from(MAX_RESULT_SCHOOL_RECORDS).map_err(|_| Error::BudgetExceeded)?
        {
            return Err(Error::Corrupt("authority head").into());
        }
        let count = usize::try_from(head.inventory.records).map_err(|_| Error::BudgetExceeded)?;
        require_record_keys(snapshot, journal, count, grant)?;
        require_receipt_count(snapshot, receipts, count, grant)?;
        Ok(InspectedMetadata {
            revision: view.revision(),
            store: enrollment.command.store,
            inventory: head.inventory,
        })
    })
}

fn require_record_keys(
    snapshot: &Snapshot,
    journal: &Keyspace,
    count: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant
        .charge_steps(count.checked_add(2).ok_or(Error::BudgetExceeded)?)
        .map_err(grant_error)?;
    let mut rows = snapshot.prefix(journal, NAME);
    for ordinal in 0..count {
        let row = rows
            .next()
            .ok_or(Error::Corrupt("missing issuance record key"))?;
        let key = row.key().map_err(|source| StoreError::Read { source })?;
        grant.charge_bytes(key.len()).map_err(grant_error)?;
        if key.as_ref() != wire::record_key(ordinal)?.as_bytes() {
            return Err(Error::Corrupt("record ordinal or namespace").into());
        }
    }
    if let Some(row) = rows.next() {
        row.key().map_err(|source| StoreError::Read { source })?;
        return Err(Error::Corrupt("extra issuance namespace row").into());
    }
    Ok(())
}

fn require_receipt_count(
    snapshot: &Snapshot,
    receipts: &Keyspace,
    count: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let expected = count.checked_add(1).ok_or(Error::BudgetExceeded)?;
    let mut seen = 0usize;
    snapshot
        .prefix(receipts, NAME)
        .take(MAX_RESULT_SCHOOL_RECORDS + 2)
        .try_for_each(|row| -> StoreResult<()> {
            grant.charge_steps(1).map_err(grant_error)?;
            let key = row.key().map_err(|source| StoreError::Read { source })?;
            grant.charge_bytes(key.len()).map_err(grant_error)?;
            seen = seen.checked_add(1).ok_or(Error::BudgetExceeded)?;
            if seen > expected {
                return Err(Error::Corrupt("extra issuance receipt").into());
            }
            Ok(())
        })?;
    if seen != expected {
        return Err(Error::Corrupt("missing issuance receipt").into());
    }
    Ok(())
}
