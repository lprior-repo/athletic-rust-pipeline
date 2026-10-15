use super::{
    consume, ApprovedLocalAdjudicationV1, LocalAdjudicationCommitOutcomeV1,
    LocalAdjudicationWitnessV1, PreparedLocalAdjudicationV1,
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
        consume(
            self,
            store,
            prepared,
            (approved_bytes, approval_digest),
            persist,
        )
    }

    pub fn replay_local_adjudication(
        &self,
        store: &Store,
        operation: &str,
        approval_digest: &str,
    ) -> StoreResult<Option<LocalAdjudicationCommitOutcomeV1>> {
        let _guard = store.lock_appends();
        let ledger = self.verified(store)?;
        match operation::repeated(store, operation, approval_digest, &ledger)? {
            Some(operation::Replayed::Manual(outcome)) => Ok(Some(outcome)),
            Some(_) => Err(PublicIdentityError::OperationConflict.into()),
            None => Ok(None),
        }
    }
}

fn persist(
    store: &Store,
    approved: ApprovedLocalAdjudicationV1,
    ledger: Ledger,
) -> StoreResult<LocalAdjudicationCommitOutcomeV1> {
    let head = approved.history.next_head.clone();
    let record = approved.history.uncommitted_record.clone();
    let at = approved.record.observed_at.clone();
    let witness = LocalAdjudicationWitnessV1::new(approved);
    let outcome = witness.outcome(&record, false)?;
    let (mut records, _) = ledger.history.into_parts();
    records
        .try_reserve(1)
        .map_err(|error| StoreError::Refused {
            detail: format!("manual history allocation: {error}"),
        })?;
    records.push(record);
    let history = codec::encode(&records, MAX_PUBLIC_IDENTITY_HISTORY_BYTES)?;
    codec::history(&history, &witness.adjudication.store, &head)?;
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
