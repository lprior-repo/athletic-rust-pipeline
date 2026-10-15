use super::{request, ReviewedIdentityCommitOutcomeV1, ReviewedIdentityRequestV1, ReviewedWitness};
use crate::clock::{Clock, SystemClock};
use crate::public_identity::{
    authority, codec, commit, context, operation, qualification::Ledger, PublicIdentityError,
    PublicIdentityWriterV1, MAX_PUBLIC_IDENTITY_HISTORY_BYTES,
};
use crate::{Store, StoreError, StoreResult, StoreRevision};
use census_domain::model::AcceptedAthleteIdentity;
use census_domain::public_identity_history::{
    prepare_next, PreparedHistoryRecord, RegistryHead, ReplayLimits,
};

struct Pending<'a, 's> {
    operation: &'a str,
    request: &'a ReviewedIdentityRequestV1,
    digest: String,
    expected: StoreRevision<'s>,
    head: &'a RegistryHead,
}
struct Prepared<'s> {
    snapshot: crate::StoreSnapshot<'s>,
    accepted: AcceptedAthleteIdentity,
    record: PreparedHistoryRecord,
    context: String,
    at: String,
}

impl PublicIdentityWriterV1 {
    pub fn apply_reviewed_identity(
        &self,
        store: &Store,
        operation: &str,
        request: &ReviewedIdentityRequestV1,
        expected: StoreRevision<'_>,
        head: &RegistryHead,
    ) -> StoreResult<ReviewedIdentityCommitOutcomeV1> {
        let guard = store.lock_appends();
        expected.ensure_origin(store, operation)?;
        let ledger = self.verified(store)?;
        let digest = request::digest(&self.identity, operation, request)?;
        if let Some(outcome) = repeated(store, operation, &digest, &ledger)? {
            return Ok(outcome);
        }
        self.require_current_qualification()?;
        expected.ensure_current(store, operation)?;
        if head != ledger.history.projection().head() {
            return Err(PublicIdentityError::HeadMoved.into());
        }
        drop(guard);
        let pending = Pending {
            operation,
            request,
            digest,
            expected,
            head,
        };
        let prepared = self.prepare_reviewed(store, &pending, &ledger)?;
        let _guard = store.lock_appends();
        let ledger = self.verified(store)?;
        if let Some(outcome) = repeated(store, operation, &pending.digest, &ledger)? {
            return Ok(outcome);
        }
        pending.expected.ensure_current(store, operation)?;
        if head != ledger.history.projection().head() {
            return Err(PublicIdentityError::HeadMoved.into());
        }
        self.persist_reviewed(store, pending, prepared, ledger)
    }

    fn prepare_reviewed<'s>(
        &self,
        store: &'s Store,
        pending: &Pending<'_, '_>,
        ledger: &Ledger,
    ) -> StoreResult<Prepared<'s>> {
        let snapshot = store.snapshot();
        let inputs = context::Inputs::capture(&snapshot)?;
        crate::public_identity::local_adjudication::fence::require_available(
            store,
            ledger.history.records(),
            &inputs,
            pending.request.case_id(),
        )?;
        let at = SystemClock.today_iso8601();
        let accepted = authority::accept_reviewed(&inputs, pending.request.case_id(), &at)?;
        let request = pending.request.authorized(&accepted)?;
        let record = prepare_next(&self.identity, pending.head, request, ReplayLimits::SMALL)
            .map_err(PublicIdentityError::from)?;
        let canonical = ReviewedIdentityRequestV1::from_history(
            pending.request.case_id(),
            &record.uncommitted_record.request,
        )?;
        if &canonical != pending.request {
            return Err(PublicIdentityError::NonCanonicalRequest.into());
        }
        ledger
            .history
            .projection()
            .clone()
            .extend_candidate(&record.uncommitted_record)
            .map_err(PublicIdentityError::from)?;
        let context = inputs.stage(store, &self.identity)?.object_id().to_owned();
        Ok(Prepared {
            snapshot,
            accepted,
            record,
            context,
            at,
        })
    }

    fn persist_reviewed(
        &self,
        store: &Store,
        pending: Pending<'_, '_>,
        prepared: Prepared<'_>,
        ledger: Ledger,
    ) -> StoreResult<ReviewedIdentityCommitOutcomeV1> {
        let Prepared {
            snapshot,
            accepted,
            record,
            context,
            at,
        } = prepared;
        let (mut records, projection) = ledger.history.into_parts();
        projection
            .extend_candidate(&record.uncommitted_record)
            .map_err(PublicIdentityError::from)?;
        let witness = ReviewedWitness::new(
            pending.operation,
            pending.request,
            &snapshot,
            &record,
            accepted.record(),
            &context,
        )?;
        let outcome = witness.outcome(&record.uncommitted_record, false)?;
        records
            .try_reserve(1)
            .map_err(|error| StoreError::Refused {
                detail: format!("identity history allocation: {error}"),
            })?;
        records.push(record.uncommitted_record);
        let history = codec::encode(&records, MAX_PUBLIC_IDENTITY_HISTORY_BYTES)?;
        codec::history(&history, &self.identity, &record.next_head)?;
        commit::persist(
            store,
            &witness,
            &witness.operation,
            commit::DecisionWrite::Accepted(accepted.record()),
            &record.next_head,
            history,
            &at,
        )?;
        Ok(outcome)
    }
}

fn repeated(
    store: &Store,
    operation: &str,
    digest: &str,
    ledger: &Ledger,
) -> StoreResult<Option<ReviewedIdentityCommitOutcomeV1>> {
    match operation::repeated(store, operation, digest, ledger)? {
        Some(operation::Replayed::Reviewed(outcome)) => Ok(Some(outcome)),
        Some(operation::Replayed::Source(_) | operation::Replayed::Manual(_)) => {
            Err(PublicIdentityError::OperationConflict.into())
        }
        None => Ok(None),
    }
}
