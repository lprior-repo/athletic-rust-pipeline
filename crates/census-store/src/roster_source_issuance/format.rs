use super::{codec, enrollment, RosterSourceIssuanceStoreError as Error};
use super::{RosterStartupValidationBudgetV1, ENROLLMENT, HEAD, LIFECYCLE, OPERATION_PREFIX, ROOT};
use crate::{Receipt, StoreError, StoreFormat, StoreResult};
use fjall::{Keyspace, Readable, Snapshot};

pub(super) type Rows = Vec<(Vec<u8>, Vec<u8>)>;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;

pub(crate) fn validate_format(
    snapshot: &Snapshot,
    meta: &Keyspace,
    journal: &Keyspace,
    receipts: &Keyspace,
    format: &StoreFormat,
    budget: &mut RosterStartupValidationBudgetV1,
) -> StoreResult<()> {
    let metadata = protected_rows(snapshot, meta, budget, None)?;
    let records = protected_rows(snapshot, journal, budget, None)?;
    let issued = scan(snapshot, receipts, OPERATION_PREFIX, budget, None)?;
    if format.lifecycle_revision != Some(LIFECYCLE) {
        if metadata.is_empty() && records.is_empty() && issued.is_empty() {
            return Ok(());
        }
        return Err(Error::Corrupt("roster state outside admitted lifecycle").into());
    }
    if format.schema_version != Some(2)
        || format.key_format != Some(1)
        || format.migration_target.is_some()
    {
        return Err(Error::EnrollmentRequired.into());
    }
    let key = crate::Store::journal_key("export-lineage-v1", "store-identity");
    if let Some(length) = snapshot
        .size_of(journal, &key)
        .map_err(|source| StoreError::Read { source })?
    {
        let bytes = usize::try_from(length).map_err(|_| Error::BudgetExceeded)?;
        budget.charge(bytes.checked_add(key.len()).ok_or(Error::BudgetExceeded)?)?;
    }
    let identity = match crate::lineage::read(snapshot, journal)? {
        crate::lineage::RetainedIdentity::Present(identity) => identity,
        crate::lineage::RetainedIdentity::Absent => {
            return Err(Error::Corrupt("missing origin lineage").into())
        }
    };
    validate_enrollment(&metadata, &records, &issued, &identity)
}

pub(super) fn scan(
    snapshot: &Snapshot,
    table: &Keyspace,
    prefix: &str,
    budget: &mut RosterStartupValidationBudgetV1,
    mut grant: Option<&mut AttachmentReplayGrantV1>,
) -> StoreResult<Rows> {
    if let Some(grant) = grant.as_deref_mut() {
        grant.charge_steps(1).map_err(super::source_error)?;
    }
    let mut iterator = snapshot.prefix(table, prefix);
    let mut rows = Vec::new();
    for _ in 0..4097 {
        if let Some(grant) = grant.as_deref_mut() {
            grant.charge_steps(2).map_err(super::source_error)?;
        }
        let Some(guard) = iterator.next() else {
            return Ok(rows);
        };
        let key = guard.key().map_err(|source| StoreError::Read { source })?;
        let value = read_value(snapshot, table, &key, budget, grant.as_deref_mut())?;
        rows.try_reserve(1).map_err(|_| Error::BudgetExceeded)?;
        rows.push((key.to_vec(), value));
    }
    Err(Error::BudgetExceeded.into())
}

pub(super) fn validate_enrollment(
    metadata: &Rows,
    records: &Rows,
    issued: &Rows,
    identity: &census_domain::model::StoreIdentity,
) -> StoreResult<()> {
    if metadata.len() != 2 || issued.is_empty() {
        return Err(Error::Corrupt("enrollment inventory cardinality").into());
    }
    let enrolled: enrollment::EnrollmentRecord = codec::decode(find(metadata, ENROLLMENT)?)?;
    let head: enrollment::AuthorityHead = codec::decode(find(metadata, HEAD)?)?;
    let operation = format!("{OPERATION_PREFIX}{}", enrolled.command.operation.as_str());
    let (receipt_key, receipt_bytes) = issued
        .iter()
        .find(|(key, _)| key == operation.as_bytes())
        .ok_or(Error::Corrupt("missing enrollment receipt"))?;
    let receipt: Receipt = codec::decode(receipt_bytes)?;
    let enrollment_digest = codec::digest(find(metadata, ENROLLMENT)?);
    let approval = enrollment::approval(&enrolled.command)?;
    enrollment::prepare::validate_command(&enrolled.command)?;
    if &enrolled.command.store != identity
        || enrolled.schema != "roster-source-enrollment-v1"
        || enrolled.approval_digest != codec::digest(&approval)
        || enrolled.committed_at < enrolled.command.reviewed_at
        || head.version != 1
        || receipt_key != operation.as_bytes()
        || receipt.operation != operation
        || receipt.digest != enrollment_digest
        || receipt.at != enrolled.committed_at.to_rfc3339()
        || receipt.appended != 0
        || receipt.credited
    {
        return Err(Error::Corrupt("enrollment authority mismatch").into());
    }
    super::issuance::ledger::validate(records, issued, identity, &enrollment_digest, &head)
}

fn find<'a>(rows: &'a Rows, key: &str) -> StoreResult<&'a [u8]> {
    rows.iter()
        .find(|(held, _)| held == key.as_bytes())
        .map(|(_, bytes)| bytes.as_slice())
        .ok_or_else(|| Error::Corrupt("missing enrollment metadata").into())
}

pub(super) fn protected_rows(
    snapshot: &Snapshot,
    table: &Keyspace,
    budget: &mut RosterStartupValidationBudgetV1,
    mut grant: Option<&mut AttachmentReplayGrantV1>,
) -> StoreResult<Rows> {
    let mut rows = scan(snapshot, table, ROOT, budget, grant.as_deref_mut())?;
    let phase = scan(
        snapshot,
        table,
        "roster_source_issuance\0",
        budget,
        grant.as_deref_mut(),
    )?;
    rows.try_reserve(phase.len().checked_add(1).ok_or(Error::BudgetExceeded)?)
        .map_err(|_| Error::BudgetExceeded)?;
    rows.extend(phase);
    let bare = b"roster_source_issuance";
    if let Some(grant) = grant.as_deref_mut() {
        grant.charge_steps(1).map_err(super::source_error)?;
    }
    if snapshot
        .contains_key(table, bare)
        .map_err(|source| StoreError::Read { source })?
    {
        rows.push((
            bare.to_vec(),
            read_value(snapshot, table, bare, budget, grant)?,
        ));
    }
    Ok(rows)
}

fn read_value(
    snapshot: &Snapshot,
    table: &Keyspace,
    key: &[u8],
    budget: &mut RosterStartupValidationBudgetV1,
    mut grant: Option<&mut AttachmentReplayGrantV1>,
) -> StoreResult<Vec<u8>> {
    if let Some(grant) = grant.as_deref_mut() {
        grant.charge_steps(1).map_err(super::source_error)?;
    }
    let length = snapshot
        .size_of(table, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("missing inventory value"))?;
    let length = usize::try_from(length).map_err(|_| Error::BudgetExceeded)?;
    budget.charge(length.checked_add(key.len()).ok_or(Error::BudgetExceeded)?)?;
    let cap = if key.starts_with(super::issuance::OBJECT_PREFIX.as_bytes()) {
        4 * 1024 * 1024
    } else {
        codec::MAX_BYTES
    };
    if length > cap {
        return Err(Error::BudgetExceeded.into());
    }
    if let Some(grant) = grant {
        let interpreted = length
            .checked_add(key.len())
            .and_then(|size| size.checked_mul(4))
            .ok_or(Error::BudgetExceeded)?;
        grant
            .charge_bytes(interpreted)
            .map_err(super::source_error)?;
        grant.charge_steps(32).map_err(super::source_error)?;
    }
    let value = snapshot
        .get(table, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("missing inventory value"))?;
    if value.len() != length {
        return Err(Error::Corrupt("inventory length changed").into());
    }
    Ok(value.to_vec())
}
