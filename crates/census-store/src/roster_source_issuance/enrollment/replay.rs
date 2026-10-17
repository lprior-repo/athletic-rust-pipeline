use super::{approval, EnrollmentRecord};
use super::super::{codec, RosterSourceIssuanceStoreError as Error};
use super::super::{RosterStartupValidationBudgetV1, ENROLLMENT, OPERATION_PREFIX};
use crate::{Application, Receipt, Store, StoreError, StoreResult};
use fjall::{Keyspace, Readable, Snapshot};

impl Store {
    pub fn replay_roster_source_enrollment_v1(
        &self,
        operation: &str,
        approval_digest: &str,
    ) -> StoreResult<Application> {
        if operation.is_empty() || operation.len() > 256 || approval_digest.len() != 64 {
            return Err(Error::OperationConflict.into());
        }
        let _guard = self.lock_appends();
        let view = self.snapshot_locked();
        let snapshot = self.db.snapshot();
        let mut budget = RosterStartupValidationBudgetV1::new();
        let bytes = read(&snapshot, &self.meta, ENROLLMENT, &mut budget)?;
        let enrolled: EnrollmentRecord = codec::decode(&bytes)?;
        let identity_key = Store::journal_key("export-lineage-v1", "store-identity");
        let identity_bytes = snapshot.size_of(&self.journal, &identity_key)
            .map_err(|source| StoreError::Read { source })?
            .ok_or(Error::Corrupt("historical origin missing"))?;
        let identity_bytes = usize::try_from(identity_bytes).map_err(|_| Error::BudgetExceeded)?;
        budget.charge(identity_bytes.checked_add(identity_key.len()).ok_or(Error::BudgetExceeded)?)?;
        super::prepare::validate_command(&enrolled.command)?;
        if enrolled.command.store != view.store_identity()?
            || enrolled.command.operation.as_str() != operation
            || enrolled.schema != "roster-source-enrollment-v1"
            || enrolled.approval_digest != approval_digest
            || codec::digest(&approval(&enrolled.command)?) != approval_digest
            || enrolled.committed_at < enrolled.command.reviewed_at
        {
            return Err(Error::OperationConflict.into());
        }
        let key = format!("{OPERATION_PREFIX}{operation}");
        let receipt: Receipt = codec::decode(&read(&snapshot, &self.receipts, &key, &mut budget)?)?;
        if receipt.operation != key || receipt.digest != codec::digest(&bytes)
            || receipt.at != enrolled.committed_at.to_rfc3339()
            || receipt.appended != 0 || receipt.credited
        {
            return Err(Error::Corrupt("historical enrollment receipt").into());
        }
        Ok(Application::Repeated(receipt))
    }
}

fn read(snapshot: &Snapshot, table: &Keyspace, key: &str, budget: &mut RosterStartupValidationBudgetV1) -> StoreResult<Vec<u8>> {
    let size = snapshot.size_of(table, key).map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("historical enrollment object missing"))?;
    let size = usize::try_from(size).map_err(|_| Error::BudgetExceeded)?;
    budget.charge(size.checked_add(key.len()).ok_or(Error::BudgetExceeded)?)?;
    if size > codec::MAX_BYTES { return Err(Error::BudgetExceeded.into()); }
    let bytes = snapshot.get(table, key).map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("historical enrollment object missing"))?;
    if bytes.len() != size { return Err(Error::Corrupt("historical object length").into()); }
    Ok(bytes.to_vec())
}
