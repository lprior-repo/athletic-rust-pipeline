use super::super::{
    codec, enrollment::AuthorityHead, RosterSourceIssuanceStoreError as Error,
    RosterStartupValidationBudgetV1,
};
use super::ReviewRecord;
use crate::{Application, Store, StoreError, StoreResult};
use fjall::{Keyspace, Readable, Snapshot};

impl Store {
    pub fn replay_roster_source_review_v1(
        &self,
        operation: &str,
        approval_digest: &str,
    ) -> StoreResult<Application> {
        let _guard = self.lock_appends();
        replay(self, operation, approval_digest)
    }
}

pub(super) fn replay(
    store: &Store,
    operation: &str,
    approval_digest: &str,
) -> StoreResult<Application> {
    if operation.is_empty() || operation.len() > 256 || approval_digest.len() != 64 {
        return Err(Error::OperationConflict.into());
    }
    let snapshot = store.db.snapshot();
    let mut budget = RosterStartupValidationBudgetV1::new();
    let (raw, record) = load_record(
        &snapshot,
        &store.journal,
        operation,
        approval_digest,
        &mut budget,
    )?;
    let identity = load_identity(store, &snapshot, &mut budget)?;
    let objects = load_closure(&snapshot, &store.journal, &record, &mut budget)?;
    let source_records = source_records(&snapshot, &store.journal, &record, &objects, &mut budget)?;
    check_chain(&record, &objects, &identity, &source_records)?;
    let receipt = load_receipt(&snapshot, &store.receipts, &raw, &record, &mut budget)?;
    Ok(Application::Repeated(receipt))
}

fn load_record(
    snapshot: &Snapshot,
    journal: &Keyspace,
    operation: &str,
    approval_digest: &str,
    budget: &mut RosterStartupValidationBudgetV1,
) -> StoreResult<(Vec<u8>, ReviewRecord)> {
    let key = format!(
        "{}{}",
        super::RECORD_PREFIX,
        codec::digest(operation.as_bytes())
    );
    let raw = read(snapshot, journal, &key, codec::MAX_BYTES, budget)?;
    let record: ReviewRecord = codec::decode(&raw)?;
    if record.basis.operation.as_str() != operation || record.approval_digest != approval_digest {
        return Err(Error::OperationConflict.into());
    }
    Ok((raw, record))
}

fn load_identity(
    store: &Store,
    snapshot: &Snapshot,
    budget: &mut RosterStartupValidationBudgetV1,
) -> StoreResult<census_domain::model::StoreIdentity> {
    let identity_key = Store::journal_key("export-lineage-v1", "store-identity");
    let size = snapshot
        .size_of(&store.journal, &identity_key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("historical review origin missing"))?;
    budget.charge(
        usize::try_from(size)
            .map_err(|_| Error::BudgetExceeded)?
            .checked_add(identity_key.len())
            .ok_or(Error::BudgetExceeded)?,
    )?;
    store.snapshot_locked().store_identity()
}

fn load_closure(
    snapshot: &Snapshot,
    journal: &Keyspace,
    record: &ReviewRecord,
    budget: &mut RosterStartupValidationBudgetV1,
) -> StoreResult<super::objects::Objects> {
    let mut objects = super::objects::Objects::new();
    for reference in &record.basis.closure {
        let bytes = read(
            snapshot,
            journal,
            &format!("{}{}", super::OBJECT_PREFIX, reference.digest),
            4 * 1024 * 1024,
            budget,
        )?;
        if bytes.len() != reference.bytes || codec::digest(&bytes) != reference.digest {
            return Err(Error::Corrupt("historical review object").into());
        }
        if objects.insert(reference.digest.clone(), bytes).is_some() {
            return Err(Error::Corrupt("historical duplicate object").into());
        }
    }
    Ok(objects)
}

fn check_chain(
    record: &ReviewRecord,
    objects: &super::objects::Objects,
    identity: &census_domain::model::StoreIdentity,
    source_records: &[(Vec<u8>, Vec<u8>)],
) -> StoreResult<()> {
    let prior: AuthorityHead =
        codec::decode(super::ledger::object(objects, &record.basis.prior_head)?)?;
    if prior.version != 1 || prior.ordinal.checked_add(1) != Some(record.basis.next_ordinal) {
        return Err(Error::Corrupt("historical review ordinal").into());
    }
    super::ledger::validate_record(record, objects, identity, &prior, source_records)
}

fn load_receipt(
    snapshot: &Snapshot,
    receipts: &Keyspace,
    raw: &[u8],
    record: &ReviewRecord,
    budget: &mut RosterStartupValidationBudgetV1,
) -> StoreResult<crate::Receipt> {
    let operation = format!(
        "{}{}",
        super::super::OPERATION_PREFIX,
        record.basis.operation.as_str()
    );
    let receipt: crate::Receipt = codec::decode(&read(
        snapshot,
        receipts,
        &operation,
        codec::MAX_BYTES,
        budget,
    )?)?;
    if receipt.operation != operation
        || receipt.digest != codec::digest(raw)
        || receipt.at != record.committed_at.to_rfc3339()
        || receipt.appended != 0
        || receipt.credited
    {
        return Err(Error::Corrupt("historical review receipt").into());
    }
    Ok(receipt)
}

fn read(
    snapshot: &Snapshot,
    table: &Keyspace,
    key: &str,
    cap: usize,
    budget: &mut RosterStartupValidationBudgetV1,
) -> StoreResult<Vec<u8>> {
    let size = snapshot
        .size_of(table, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("historical review object missing"))?;
    let size = usize::try_from(size).map_err(|_| Error::BudgetExceeded)?;
    budget.charge(size.checked_add(key.len()).ok_or(Error::BudgetExceeded)?)?;
    if size > cap {
        return Err(Error::BudgetExceeded.into());
    }
    let bytes = snapshot
        .get(table, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("historical review object disappeared"))?;
    if bytes.len() != size {
        return Err(Error::Corrupt("historical review object size").into());
    }
    Ok(bytes.to_vec())
}

fn source_records(
    snapshot: &Snapshot,
    journal: &Keyspace,
    record: &ReviewRecord,
    objects: &super::objects::Objects,
    budget: &mut RosterStartupValidationBudgetV1,
) -> StoreResult<Vec<(Vec<u8>, Vec<u8>)>> {
    if record.basis.schema != super::withdrawal::BASIS_SCHEMA {
        return Ok(Vec::new());
    }
    let review: super::SemanticReview =
        codec::decode(super::ledger::object(objects, &record.basis.review)?)?;
    let super::RosterSourceReviewDecisionV1::WithdrawExactSourceMember(target) =
        &review.fields.decision
    else {
        return Err(Error::Corrupt("withdrawal replay decision").into());
    };
    let key = format!(
        "{}{}",
        super::RECORD_PREFIX,
        codec::digest(target.source_operation().as_bytes())
    );
    let raw = read(snapshot, journal, &key, codec::MAX_BYTES, budget)?;
    let mut records = Vec::new();
    records
        .try_reserve_exact(1)
        .map_err(|_| Error::BudgetExceeded)?;
    records.push((key.into_bytes(), raw));
    Ok(records)
}
