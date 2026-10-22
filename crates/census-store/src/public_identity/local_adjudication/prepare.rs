use super::{LocalAdjudicationInventoryV1, PreparedLocalAdjudicationV1};
use crate::public_identity::{
    authority::graph,
    codec, context, graph_operation,
    graph_witness::{HeldGraphWitness, LocalActor, Payload},
    paged_generation::VerifiedRawGraph,
    PublicIdentityError, PublicIdentityWriterV1,
};
use crate::{Store, StoreResult, StoreRevision};
use census_domain::model::{serialized_digest, AthleteCandidateId};
use census_domain::public_identity_history::local_adjudication::{
    LocalEvidenceBasisV1, LocalIdentityAdjudicationRecordV3, LocalIdentitySplitV1,
    LOCAL_ADJUDICATION_GRAPH_POLICY,
};
use census_domain::public_identity_history::{
    HistoryProjection, HistoryRequest, PublicAthleteId, RegistryHead,
};
impl PublicIdentityWriterV1 {
    pub fn inspect_local_adjudication(
        &self,
        store: &Store,
        operation: &str,
        canonical: &PublicAthleteId,
    ) -> StoreResult<LocalAdjudicationInventoryV1> {
        let (reference, ledger) = graph_operation::inspect(self, store, operation)?;
        let snapshot = store.snapshot();
        let view = VerifiedRawGraph::verify(&snapshot, reference.capture(), reference.seal())?;
        let members = graph::members(&view, ledger.history.projection(), canonical)?;
        Ok(LocalAdjudicationInventoryV1 {
            affected_record_keys: graph::record_keys(&view, &members)?,
            claims: graph::membership_inventory(
                &view,
                &members,
                ledger.history.projection(),
                ledger.history.records(),
            )?,
            graph: reference,
        })
    }
    pub fn prepare_local_adjudication<'s>(
        &self,
        store: &'s Store,
        operation: &str,
        request: LocalIdentitySplitV1,
        expected: StoreRevision<'s>,
        head: &RegistryHead,
    ) -> StoreResult<PreparedLocalAdjudicationV1<'s>> {
        crate::public_identity::commit::source_batch::refuse_child_operation(operation)?;
        let digest = request_digest(&self.identity, operation, &request)?;
        let mut capture = graph_operation::begin(self, store, operation, &digest, expected, head)?;
        #[cfg(test)]
        graph_operation::preparation_gate::checkpoint(&capture)?;
        let snapshot = store.snapshot();
        let view =
            VerifiedRawGraph::verify(&snapshot, capture.graph.capture(), capture.graph.seal())?;
        let guard = store.lock_appends();
        let ledger = graph_operation::require_current(
            self,
            store,
            operation,
            expected,
            head,
            &capture.graph,
        )?;
        if let Some(prior) = graph_operation::current_preparation(
            store, operation, &digest, expected, head, &capture,
        )? {
            capture.observed_at.clone_from(&prior.observed_at);
            capture.reserved = Some(prior);
        }
        let members = graph::members(&view, ledger.history.projection(), &request.canonical)?;
        let binding = capture.graph.capture();
        let record = LocalIdentityAdjudicationRecordV3 {
            version: 3,
            policy: LOCAL_ADJUDICATION_GRAPH_POLICY.into(),
            store: self.identity.clone(),
            operation: operation.into(),
            expected_head: head.clone(),
            evidence_generation: binding.evidence_generation(),
            derived_generation: binding.derived_generation(),
            snapshot_sequence: binding.snapshot_sequence(),
            graph: capture.graph.domain_reference()?,
            members,
            request,
            observed_at: capture.observed_at.clone(),
            evidence_basis: LocalEvidenceBasisV1::RetainedObservationOnly,
        };
        let history = rebuild_history(store, &capture.graph, &record, &ledger)?;
        let witness = approval_witness(operation, &capture, &record, &history)?;
        graph_operation::reserve(store, operation, &digest, expected, head, &capture, witness)?;
        drop(guard);
        Ok(PreparedLocalAdjudicationV1 {
            record,
            graph: capture.graph,
            expected,
            history,
        })
    }
}
pub(super) fn request_digest(
    identity: &census_domain::model::StoreIdentity,
    operation: &str,
    request: &LocalIdentitySplitV1,
) -> StoreResult<String> {
    graph_operation::raw_operation(identity, operation)?;
    validate_request(request)?;
    Ok(serialized_digest(&(
        "local-identity-adjudication-request/v3",
        identity,
        operation,
        request,
    ))?)
}
fn validate_request(request: &LocalIdentitySplitV1) -> StoreResult<()> {
    if request.supersedes.len() > 128
        || request.partitions.len() > 128
        || request.dispositions.len() > 4096
        || request.unresolved_bindings.len() > 128
        || request.affected_record_keys.len() > 1024
    {
        return Err(PublicIdentityError::Corrupt("manual request collection bound").into());
    }
    codec::encode(request, 1024 * 1024)?;
    Ok(())
}

pub(in crate::public_identity) fn members(
    inputs: &context::Inputs,
    projection: &HistoryProjection,
    canonical: &PublicAthleteId,
) -> StoreResult<Vec<AthleteCandidateId>> {
    let mut members = super::members::Members::new(projection, canonical);
    inputs
        .athletes
        .iter()
        .try_for_each(|athlete| members.observe(athlete))?;
    Ok(members.finish())
}

pub(in crate::public_identity) fn validate_prior(
    request: &LocalIdentitySplitV1,
    records: &[census_domain::public_identity_history::HistoryRecord],
) -> StoreResult<()> {
    for prior in &request.supersedes {
        let record = records
            .iter()
            .find(|row| row.sequence == prior.sequence)
            .ok_or(PublicIdentityError::Corrupt(
                "manual superseded merge missing",
            ))?;
        if record.digest != prior.digest
            || !matches!(
                record.request,
                HistoryRequest::Merge { .. } | HistoryRequest::BindCandidateMembership { .. }
            )
        {
            return Err(PublicIdentityError::Corrupt("manual superseded merge mismatch").into());
        }
    }
    Ok(())
}

pub(super) fn validate_case(
    inputs: &context::Inputs,
    request: &LocalIdentitySplitV1,
    members: &[AthleteCandidateId],
) -> StoreResult<()> {
    let case = inputs
        .cases
        .iter()
        .find(|case| case.id == request.case_id)
        .ok_or(PublicIdentityError::Corrupt("manual review case missing"))?;
    let mut case_members = case.member_ids.clone();
    case_members.sort();
    if case_members != members {
        return Err(PublicIdentityError::Corrupt("manual review case members").into());
    }
    Ok(())
}

pub(super) fn rebuild_history(
    store: &Store,
    graph: &crate::public_identity::graph_reference::GraphAuthorityReferenceV1,
    record: &LocalIdentityAdjudicationRecordV3,
    ledger: &crate::public_identity::qualification::Ledger,
) -> StoreResult<census_domain::public_identity_history::PreparedHistoryRecord> {
    let retained = store.db.snapshot();
    let reader = crate::public_identity::retained::Live {
        snapshot: &retained,
        entities: &store.entities,
        meta: &store.meta,
        journal: &store.journal,
        receipts: &store.receipts,
    };
    crate::public_identity::graph_witness::manual::rebuild_manual(
        &reader,
        graph,
        record,
        ledger.history.projection(),
        ledger.history.records(),
    )
}

fn approval_witness(
    operation: &str,
    capture: &graph_operation::Capture,
    record: &LocalIdentityAdjudicationRecordV3,
    history: &census_domain::public_identity_history::PreparedHistoryRecord,
) -> StoreResult<HeldGraphWitness> {
    let approval = super::consume::graph_approval_bytes(record, &capture.graph)?;
    HeldGraphWitness::new(
        operation,
        super::consume::graph_approval_digest(&approval),
        capture.graph.clone(),
        &capture.observed_at,
        history,
        Payload::Manual {
            actor: LocalActor::LocalOperator,
            adjudication: record.clone(),
        },
    )
}
