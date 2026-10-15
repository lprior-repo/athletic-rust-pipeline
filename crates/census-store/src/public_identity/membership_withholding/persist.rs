use super::{consume, MembershipWithholdingOutcomeV1, PreparedMembershipWithholdingV1};
use crate::public_identity::{
    codec, commit, operation, qualification::Ledger, PublicIdentityError, PublicIdentityWriterV1,
    MAX_PUBLIC_IDENTITY_HISTORY_BYTES,
};
use crate::{Store, StoreError, StoreResult};
use census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV2;
use census_domain::public_identity_history::PreparedHistoryRecord;

impl PublicIdentityWriterV1 {
    pub fn apply_membership_withholding(
        &self,
        store: &Store,
        prepared: PreparedMembershipWithholdingV1<'_>,
        approved_bytes: Vec<u8>,
        approval_digest: String,
    ) -> StoreResult<MembershipWithholdingOutcomeV1> {
        consume::consume(self, store, prepared, (approved_bytes, approval_digest))
    }
    pub fn replay_membership_withholding(
        &self,
        store: &Store,
        operation: &str,
        approval_digest: &str,
    ) -> StoreResult<Option<MembershipWithholdingOutcomeV1>> {
        let _guard = store.lock_appends();
        self.require_current_qualification()?;
        let ledger = self.verified(store)?;
        match operation::repeated(store, operation, approval_digest, &ledger)? {
            Some(operation::Replayed::MembershipWithheld(outcome)) => Ok(Some(outcome)),
            Some(_) => Err(PublicIdentityError::OperationConflict.into()),
            None => Ok(None),
        }
    }
}
pub(super) fn persist(
    store: &Store,
    record: LocalMembershipWithholdingRecordV2,
    graph: crate::public_identity::graph_reference::GraphAuthorityReferenceV1,
    history: PreparedHistoryRecord,
    approval_digest: String,
    ledger: Ledger,
) -> StoreResult<MembershipWithholdingOutcomeV1> {
    let witness = crate::public_identity::graph_witness::HeldGraphWitness::new(
        &record.operation,
        approval_digest,
        graph,
        &record.observed_at,
        &history,
        crate::public_identity::graph_witness::Payload::Withholding {
            actor: crate::public_identity::graph_witness::LocalActor::LocalOperator,
            adjudication: record.clone(),
        },
    )?;
    let operation::Replayed::MembershipWithheld(outcome) =
        witness.outcome(&history.uncommitted_record, false)?
    else {
        return Err(PublicIdentityError::OperationConflict.into());
    };
    let (mut records, _) = ledger.history.into_parts();
    records
        .try_reserve(1)
        .map_err(|error| StoreError::Refused {
            detail: format!("withholding history allocation: {error}"),
        })?;
    records.push(history.uncommitted_record);
    let encoded = codec::encode(&records, MAX_PUBLIC_IDENTITY_HISTORY_BYTES)?;
    codec::history(&encoded, &record.store, &history.next_head)?;
    commit::persist(
        store,
        &witness,
        &witness.operation,
        commit::DecisionWrite::RetainedInWitness,
        &history.next_head,
        encoded,
        &record.observed_at,
    )?;
    Ok(outcome)
}
