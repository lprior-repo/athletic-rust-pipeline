use super::{
    consume, ApprovedLocalAdjudicationV1, LocalAdjudicationCommitOutcomeV1,
    PreparedLocalAdjudicationV1,
};
use crate::public_identity::{
    codec, commit, operation, qualification::Ledger, PublicIdentityError, PublicIdentityWriterV1,
    MAX_PUBLIC_IDENTITY_HISTORY_BYTES,
};
use crate::{Store, StoreError, StoreResult};

impl PublicIdentityWriterV1 {
    pub fn apply_local_adjudication(
        &self,
        store: &Store,
        prepared: PreparedLocalAdjudicationV1<'_>,
        approved_bytes: Vec<u8>,
        approval_digest: String,
    ) -> StoreResult<LocalAdjudicationCommitOutcomeV1> {
        consume(self, store, prepared, (approved_bytes, approval_digest))
    }

    pub fn replay_local_adjudication(
        &self,
        store: &Store,
        operation: &str,
        approval_digest: &str,
    ) -> StoreResult<Option<LocalAdjudicationCommitOutcomeV1>> {
        let _guard = store.lock_appends();
        self.require_current_qualification()?;
        let ledger = self.verified(store)?;
        match operation::repeated(store, operation, approval_digest, &ledger)? {
            Some(operation::Replayed::Manual(outcome)) => Ok(Some(outcome)),
            Some(_) => Err(PublicIdentityError::OperationConflict.into()),
            None => Ok(None),
        }
    }
}

pub(super) fn persist(
    store: &Store,
    approved: ApprovedLocalAdjudicationV1,
    ledger: Ledger,
) -> StoreResult<LocalAdjudicationCommitOutcomeV1> {
    let head = approved.history.next_head.clone();
    let record = approved.history.uncommitted_record.clone();
    let at = approved.record.observed_at.clone();
    let witness = crate::public_identity::graph_witness::HeldGraphWitness::new(
        &approved.record.operation,
        approved.approval_digest,
        approved.graph,
        &at,
        &approved.history,
        crate::public_identity::graph_witness::Payload::Manual {
            actor: crate::public_identity::graph_witness::LocalActor::LocalOperator,
            adjudication: approved.record.clone(),
        },
    )?;
    let operation::Replayed::Manual(outcome) = witness.outcome(&record, false)? else {
        return Err(PublicIdentityError::OperationConflict.into());
    };
    let (mut records, _) = ledger.history.into_parts();
    records
        .try_reserve(1)
        .map_err(|error| StoreError::Refused {
            detail: format!("manual history allocation: {error}"),
        })?;
    records.push(record);
    let history = codec::encode(&records, MAX_PUBLIC_IDENTITY_HISTORY_BYTES)?;
    codec::history(&history, &approved.record.store, &head)?;
    commit::persist(
        store,
        &witness,
        &witness.operation,
        commit::DecisionWrite::RetainedInWitness,
        &head,
        history,
        &at,
    )?;
    Ok(outcome)
}
