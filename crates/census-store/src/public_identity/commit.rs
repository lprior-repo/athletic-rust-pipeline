mod decision;
#[cfg(feature = "native-fault-injection")]
mod native;

use super::{
    authority, codec, context, witness, IdentityCommitOutcomeV1, PublicIdentityError,
    PublicIdentityWriterV1, HEAD, HISTORY, MAX_PUBLIC_IDENTITY_HISTORY_BYTES,
};
use crate::clock::{Clock, SystemClock};
use crate::{Store, StoreError, StoreResult, StoreRevision};
use census_domain::model::{
    serialized_digest, AcceptedAthleteIdentity, AthleteCandidateId, SourceIdentity, StoreIdentity,
};
use census_domain::public_identity_history::{
    prepare_next, BindingClaim, CandidateDecisionReference, EffectiveScope, HistoryRecord,
    HistoryRequest, PublicResolution, RegistryHead, ReplayLimits, SourceObjectKey,
};
use fjall::PersistMode;

struct Pending<'a, 's> {
    operation: &'a str,
    candidate: &'a AthleteCandidateId,
    digest: String,
    expected: StoreRevision<'s>,
    head: &'a RegistryHead,
}

struct Prepared<'s> {
    snapshot: crate::StoreSnapshot<'s>,
    accepted: AcceptedAthleteIdentity,
    source: SourceIdentity,
    context_root: String,
    at: String,
}

impl PublicIdentityWriterV1 {
    pub fn establish_source_bound(
        &self,
        store: &Store,
        operation: &str,
        candidate: &AthleteCandidateId,
        expected: StoreRevision<'_>,
        head: &RegistryHead,
    ) -> StoreResult<IdentityCommitOutcomeV1> {
        let _guard = store.lock_appends();
        expected.ensure_origin(store, operation)?;
        let ledger = self.verified(store)?;
        let digest = witness::request_digest(&self.identity, operation, candidate)?;
        if let Some(outcome) = repeated(store, operation, &digest, &ledger)? {
            return Ok(outcome);
        }
        self.require_current_qualification()?;
        expected.ensure_current(store, operation)?;
        if head != ledger.history.projection().head() {
            return Err(PublicIdentityError::HeadMoved.into());
        }
        if !matches!(
            ledger
                .history
                .projection()
                .resolve_candidate(candidate)
                .map_err(PublicIdentityError::from)?,
            PublicResolution::Unknown
        ) {
            return Err(PublicIdentityError::AlreadyBound.into());
        }
        drop(_guard);
        self.commit_first(
            store,
            Pending {
                operation,
                candidate,
                digest,
                expected,
                head,
            },
        )
    }

    fn prepare<'s>(
        &self,
        store: &'s Store,
        candidate: &AthleteCandidateId,
    ) -> StoreResult<Prepared<'s>> {
        let snapshot = store.snapshot();
        let inputs = context::Inputs::capture(&snapshot)?;
        let at = SystemClock.today_iso8601();
        let (accepted, source) = authority::accept_inputs(&inputs, candidate, &at)?;
        let retained = inputs.stage(store, &self.identity)?;
        Ok(Prepared {
            snapshot,
            accepted,
            source,
            context_root: retained.object_id().to_owned(),
            at,
        })
    }

    fn commit_first(
        &self,
        store: &Store,
        pending: Pending<'_, '_>,
    ) -> StoreResult<IdentityCommitOutcomeV1> {
        let prepared = self.prepare(store, pending.candidate)?;
        let _guard = store.lock_appends();
        let ledger = self.verified(store)?;
        if let Some(outcome) = repeated(store, pending.operation, &pending.digest, &ledger)? {
            return Ok(outcome);
        }
        pending.expected.ensure_current(store, pending.operation)?;
        if pending.head != ledger.history.projection().head() {
            return Err(PublicIdentityError::HeadMoved.into());
        }
        self.commit_prepared(store, pending, prepared, ledger)
    }

    fn commit_prepared(
        &self,
        store: &Store,
        pending: Pending<'_, '_>,
        material: Prepared<'_>,
        ledger: super::qualification::Ledger,
    ) -> StoreResult<IdentityCommitOutcomeV1> {
        let Prepared {
            snapshot,
            accepted,
            source,
            context_root,
            at,
        } = material;
        let evidence = CandidateDecisionReference::from_accepted(&accepted, ReplayLimits::SMALL)
            .map_err(PublicIdentityError::from)?;
        let binding = source_binding(pending.candidate, source, &pending.digest);
        let prepared = prepare_next(
            &self.identity,
            ledger.history.projection().head(),
            HistoryRequest::Establish { binding, evidence },
            ReplayLimits::SMALL,
        )
        .map_err(PublicIdentityError::from)?;
        let (records, projection) = ledger.history.into_parts();
        projection
            .extend_candidate(&prepared.uncommitted_record)
            .map_err(PublicIdentityError::from)?;
        let witness = witness::Witness::new(
            pending.operation,
            pending.candidate,
            pending.digest,
            &snapshot,
            &prepared,
            accepted.record(),
            &context_root,
        )?;
        let outcome = witness.outcome(&prepared.uncommitted_record, false)?;
        let history = encode_next_history(
            records,
            prepared.uncommitted_record,
            &self.identity,
            &prepared.next_head,
        )?;
        persist(
            store,
            &witness,
            &witness.operation,
            DecisionWrite::Accepted(accepted.record()),
            &prepared.next_head,
            history,
            &at,
        )?;
        Ok(outcome)
    }
}

fn encode_next_history(
    mut records: Vec<HistoryRecord>,
    record: HistoryRecord,
    identity: &StoreIdentity,
    head: &RegistryHead,
) -> StoreResult<Vec<u8>> {
    records
        .try_reserve(1)
        .map_err(|error| StoreError::Refused {
            detail: format!("identity history allocation: {error}"),
        })?;
    records.push(record);
    let history = codec::encode(&records, MAX_PUBLIC_IDENTITY_HISTORY_BYTES)?;
    codec::history(&history, identity, head)?;
    Ok(history)
}

pub(super) fn source_binding(
    candidate: &AthleteCandidateId,
    source: census_domain::model::SourceIdentity,
    digest: &str,
) -> BindingClaim {
    BindingClaim {
        id: format!("source_binding_v1_{digest}"),
        source: SourceObjectKey {
            namespace: source.namespace,
            identifier: source.id,
        },
        candidate: candidate.clone(),
        effective: EffectiveScope::Unknown,
        supersedes: None,
    }
}

fn repeated(
    store: &Store,
    operation: &str,
    digest: &str,
    ledger: &super::qualification::Ledger,
) -> StoreResult<Option<IdentityCommitOutcomeV1>> {
    match super::operation::repeated(store, operation, digest, ledger)? {
        Some(super::operation::Replayed::Source(outcome)) => Ok(Some(outcome)),
        Some(_) => Err(PublicIdentityError::OperationConflict.into()),
        None => Ok(None),
    }
}

pub(super) enum DecisionWrite<'a> {
    Accepted(&'a census_domain::model::AppliedAthleteIdentity),
    RetainedInWitness,
}

pub(super) fn persist<T: serde::Serialize>(
    store: &Store,
    witness: &T,
    operation: &str,
    decision: DecisionWrite<'_>,
    head: &RegistryHead,
    history: Vec<u8>,
    at: &str,
) -> StoreResult<()> {
    let generation = store.generations.next_entities()?;
    let receipt_operation = witness::receipt_key(operation);
    let receipt = crate::Receipt {
        operation: receipt_operation.clone(),
        digest: serialized_digest(witness)?,
        at: at.to_owned(),
        appended: 1,
        credited: false,
    };
    let mut batch = store.db.batch();
    let witness_limit = decision.stage(store, &mut batch)?;
    batch.insert(&store.meta, HEAD, codec::encode(head, 1024)?);
    batch.insert(&store.meta, HISTORY, history);
    batch.insert(
        &store.journal,
        witness::key(head.sequence),
        codec::encode(witness, witness_limit)?,
    );
    batch.insert(
        &store.receipts,
        receipt_operation,
        codec::encode(&receipt, 65_536)?,
    );
    crate::generation::write_entities(&mut batch, &store.meta, generation);
    #[cfg(feature = "native-fault-injection")]
    native::pause_before_commit(operation, &receipt, head.sequence)?;
    batch
        .durability(Some(PersistMode::SyncData))
        .commit()
        .map_err(|source| StoreError::Write { source })?;
    store.generations.publish_entities(generation);
    #[cfg(feature = "native-fault-injection")]
    native::pause_after_commit(operation, &receipt, head.sequence)?;
    Ok(())
}
