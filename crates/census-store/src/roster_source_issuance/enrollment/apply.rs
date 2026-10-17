use super::super::{codec, format, RosterSourceIssuanceStoreError as Error};
use super::super::{
    RosterStartupValidationBudgetV1, ENROLLMENT, HEAD, LIFECYCLE, OPERATION_PREFIX,
};
use super::{AuthorityHead, EnrollmentRecord, PreparedRosterEnrollmentV1};
use crate::{Application, Receipt, Store, StoreError, StoreResult};
use chrono::Utc;
use fjall::{PersistMode, Readable};

impl Store {
    pub fn apply_roster_source_enrollment_v1(
        &self,
        prepared: PreparedRosterEnrollmentV1<'_>,
        approved: &[u8],
        approval_digest: &str,
    ) -> StoreResult<Application> {
        let _guard = self.lock_appends();
        prepared
            .revision
            .ensure_origin(self, prepared.command.operation.as_str())?;
        if approved != prepared.approval || approval_digest != prepared.approval_digest {
            return Err(Error::ApprovalMismatch.into());
        }
        let current = self.store_format()?;
        format::validate_format(
            &self.db.snapshot(),
            &self.meta,
            &self.journal,
            &self.receipts,
            &current,
            &mut RosterStartupValidationBudgetV1::new(),
        )?;
        if current.lifecycle_revision == Some(LIFECYCLE) {
            return replay(self, &prepared);
        }
        if current.lifecycle_revision != Some(7) || current.migration_target.is_some() {
            return Err(Error::EnrollmentRequired.into());
        }
        prepared
            .revision
            .ensure_current(self, prepared.command.operation.as_str())?;
        super::prepare::require_input(self, &prepared.revision, &prepared.command)?;
        commit(self, prepared)
    }
}

fn replay(store: &Store, prepared: &PreparedRosterEnrollmentV1<'_>) -> StoreResult<Application> {
    let snapshot = store.db.snapshot();
    let raw = snapshot
        .get(&store.meta, ENROLLMENT)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("missing enrollment"))?;
    let enrolled: EnrollmentRecord = codec::decode(&raw)?;
    if enrolled.command != prepared.command || enrolled.approval_digest != prepared.approval_digest
    {
        return Err(Error::OperationConflict.into());
    }
    let operation = format!("{OPERATION_PREFIX}{}", prepared.command.operation.as_str());
    let raw = snapshot
        .get(&store.receipts, operation)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(Error::Corrupt("missing enrollment receipt"))?;
    Ok(Application::Repeated(codec::decode(&raw)?))
}

fn commit(store: &Store, prepared: PreparedRosterEnrollmentV1<'_>) -> StoreResult<Application> {
    let committed_at = Utc::now();
    if committed_at < prepared.command.reviewed_at {
        return Err(Error::Corrupt("commit precedes explicit review").into());
    }
    let operation = format!("{OPERATION_PREFIX}{}", prepared.command.operation.as_str());
    let enrollment = codec::encode(&EnrollmentRecord {
        schema: "roster-source-enrollment-v1".into(),
        command: prepared.command,
        approval_digest: prepared.approval_digest,
        committed_at,
    })?;
    let digest = codec::digest(&enrollment);
    let head = codec::encode(&AuthorityHead {
        version: 1,
        ordinal: 0,
        digest: digest.clone(),
    })?;
    let receipt = Receipt {
        operation,
        digest,
        at: committed_at.to_rfc3339(),
        appended: 0,
        credited: false,
    };
    let encoded_receipt = codec::encode(&receipt)?;
    let mut batch = store.db.batch();
    batch.insert(&store.meta, ENROLLMENT, enrollment);
    batch.insert(&store.meta, HEAD, head);
    batch.insert(
        &store.receipts,
        receipt.operation.as_bytes(),
        encoded_receipt,
    );
    crate::meta::put_text(
        &mut batch,
        &store.meta,
        crate::format::LIFECYCLE_REVISION_KEY,
        "8",
    );
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    Ok(Application::Written(receipt))
}
