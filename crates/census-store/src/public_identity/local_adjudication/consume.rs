use super::{claims, PreparedLocalAdjudicationV1};
use crate::public_identity::{
    codec, context, qualification::Ledger, PublicIdentityError, PublicIdentityWriterV1,
};
use crate::{Store, StoreResult};
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV2;
use census_domain::public_identity_history::{prepare_next, PreparedHistoryRecord, ReplayLimits};
use serde::Serialize;
use sha2::{Digest, Sha256};

const APPROVAL_SCHEMA: &str = "local-identity-adjudication-approval/v1";

pub(in crate::public_identity) struct ApprovedLocalAdjudicationV1 {
    pub record: LocalIdentityAdjudicationRecordV2,
    pub approval_digest: String,
    pub history: PreparedHistoryRecord,
}

#[derive(Serialize)]
struct Envelope<'a> {
    schema: &'static str,
    actor: &'static str,
    prepared: &'a LocalIdentityAdjudicationRecordV2,
}

pub(in crate::public_identity) fn consume<R>(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    prepared: PreparedLocalAdjudicationV1<'_>,
    approval: (Vec<u8>, String),
    persist: impl FnOnce(&Store, ApprovedLocalAdjudicationV1, Ledger) -> StoreResult<R>,
) -> StoreResult<R> {
    let expected = approval_bytes(&prepared.record)?;
    if approval.0 != expected || approval.1 != approval_digest(&expected) {
        return Err(PublicIdentityError::Corrupt("manual approval binding").into());
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
    let replayed = replay_prepared(store, &prepared)?;
    ledger
        .history
        .projection()
        .clone()
        .extend_candidate(&replayed.uncommitted_record)
        .map_err(PublicIdentityError::from)?;
    persist(
        store,
        ApprovedLocalAdjudicationV1 {
            record: prepared.record,
            approval_digest: approval.1,
            history: replayed,
        },
        ledger,
    )
}

fn replay_prepared(
    store: &Store,
    prepared: &PreparedLocalAdjudicationV1<'_>,
) -> StoreResult<PreparedHistoryRecord> {
    let snapshot = store.db.snapshot();
    super::fence::validate_live(store, &prepared.record)?;
    let inputs = context::read_inputs(
        &snapshot,
        &store.journal,
        &prepared.record.authority_context,
        &prepared.record.store,
    )?;
    let affected =
        super::affected::read(&snapshot, &store.journal, &prepared.record.affected_context)?;
    affected.validate(&inputs, &prepared.record)?;
    let claims = claims::inventory(&inputs, &prepared.record.members)?;
    let request = prepared
        .record
        .history_request(&claims)
        .map_err(PublicIdentityError::from)?;
    let replayed = prepare_next(
        &prepared.record.store,
        &prepared.record.expected_head,
        request,
        ReplayLimits::SMALL,
    )
    .map_err(PublicIdentityError::from)?;
    if replayed != prepared.history {
        return Err(PublicIdentityError::Corrupt("manual prepared history binding").into());
    }
    Ok(replayed)
}

pub(super) fn approval_bytes(record: &LocalIdentityAdjudicationRecordV2) -> StoreResult<Vec<u8>> {
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
