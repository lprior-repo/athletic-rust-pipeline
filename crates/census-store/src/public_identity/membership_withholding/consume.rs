use super::PreparedMembershipWithholdingV1;
use crate::public_identity::{
    codec, qualification::Ledger, retained::Live, PublicIdentityError, PublicIdentityWriterV1,
};
use crate::{Store, StoreResult};
use census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV1;
use census_domain::public_identity_history::PreparedHistoryRecord;
use serde::Serialize;
use sha2::{Digest, Sha256};
const APPROVAL_SCHEMA: &str = "local-source-membership-withholding-approval/v1";
#[derive(Serialize)]
struct Envelope<'a> {
    schema: &'static str,
    actor: &'static str,
    prepared: &'a LocalMembershipWithholdingRecordV1,
}

pub(super) fn approval_bytes(record: &LocalMembershipWithholdingRecordV1) -> StoreResult<Vec<u8>> {
    codec::encode(
        &Envelope {
            schema: APPROVAL_SCHEMA,
            actor: "local_operator",
            prepared: record,
        },
        1024 * 1024,
    )
}
pub(super) fn approval_digest(encoded: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(APPROVAL_SCHEMA.as_bytes());
    digest.update([0]);
    digest.update(encoded);
    format!("{:x}", digest.finalize())
}

pub(super) fn consume<R>(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    prepared: PreparedMembershipWithholdingV1<'_>,
    approval: (Vec<u8>, String),
    persist: impl FnOnce(
        &Store,
        LocalMembershipWithholdingRecordV1,
        PreparedHistoryRecord,
        String,
        Ledger,
    ) -> StoreResult<R>,
) -> StoreResult<R> {
    let expected = approval_bytes(&prepared.record)?;
    if approval.0 != expected || approval.1 != approval_digest(&expected) {
        return Err(PublicIdentityError::Corrupt("withholding approval binding").into());
    }
    let _guard = store.lock_appends();
    prepared
        .expected
        .ensure_current(store, &prepared.record.operation)?;
    let ledger = writer.verified(store)?;
    if store
        .receipts
        .contains_key(crate::public_identity::witness::receipt_key(
            &prepared.record.operation,
        ))
        .map_err(|source| crate::StoreError::Read { source })?
    {
        return Err(PublicIdentityError::OperationConflict.into());
    }
    if &prepared.record.expected_head != ledger.history.projection().head() {
        return Err(PublicIdentityError::HeadMoved.into());
    }
    let snapshot = store.db.snapshot();
    let reader = Live {
        snapshot: &snapshot,
        entities: &store.entities,
        meta: &store.meta,
        journal: &store.journal,
        receipts: &store.receipts,
    };
    let history = super::witness::rebuild(
        &reader,
        &prepared.record,
        ledger.history.projection(),
        ledger.history.records(),
    )?;
    if history != prepared.history {
        return Err(PublicIdentityError::Corrupt("withholding prepared history binding").into());
    }
    persist(store, prepared.record, history, approval.1, ledger)
}
