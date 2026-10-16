use super::{MembershipWithholdingInventoryV1, PreparedMembershipWithholdingV1};
use crate::public_identity::{
    authority::graph,
    codec, graph_operation,
    graph_witness::{HeldGraphWitness, LocalActor, Payload},
    paged_generation::VerifiedRawGraph,
    PublicIdentityError, PublicIdentityWriterV1,
};
use crate::{Store, StoreResult, StoreRevision};
use census_domain::model::serialized_digest;
use census_domain::public_identity_history::local_adjudication::{
    ClaimTreatmentV1, RetainedClaimDispositionV1, RetainedClaimV1, RetainedIdentitySectionV1,
};
use census_domain::public_identity_history::local_membership_withholding::{
    LocalMembershipWithholdV1, LocalMembershipWithholdingRecordV2,
    LOCAL_MEMBERSHIP_WITHHOLDING_GRAPH_POLICY,
};
use census_domain::public_identity_history::{
    prepare_next, HistoryProjection, PreparedHistoryRecord, PublicAthleteId, RegistryHead,
    ReplayLimits,
};
impl PublicIdentityWriterV1 {
    pub fn inspect_membership_withholding(
        &self,
        store: &Store,
        operation: &str,
        canonical: &PublicAthleteId,
    ) -> StoreResult<MembershipWithholdingInventoryV1> {
        let (reference, ledger) = graph_operation::inspect(self, store, operation)?;
        let snapshot = store.snapshot();
        let view = VerifiedRawGraph::verify(&snapshot, reference.capture(), reference.seal())?;
        let members = graph::members(&view, ledger.history.projection(), canonical)?;
        Ok(MembershipWithholdingInventoryV1 {
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
    pub fn prepare_membership_withholding<'s>(
        &self,
        store: &'s Store,
        operation: &str,
        request: LocalMembershipWithholdV1,
        expected: StoreRevision<'s>,
        head: &RegistryHead,
    ) -> StoreResult<PreparedMembershipWithholdingV1<'s>> {
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
        let inventory = graph::membership_inventory(
            &view,
            &members,
            ledger.history.projection(),
            ledger.history.records(),
        )?;
        let record = withholding_record(
            self, operation, request, head, &capture, members, &inventory,
        )?;
        let history = rebuild_history(store, &capture.graph, &record, &ledger)?;
        let witness = approval_witness(operation, &capture, &record, &history)?;
        graph_operation::reserve(store, operation, &digest, expected, head, &capture, witness)?;
        drop(guard);
        Ok(PreparedMembershipWithholdingV1 {
            record,
            graph: capture.graph,
            expected,
            history,
        })
    }
}
fn withholding_record(
    writer: &PublicIdentityWriterV1,
    operation: &str,
    request: LocalMembershipWithholdV1,
    head: &RegistryHead,
    capture: &graph_operation::Capture,
    members: Vec<census_domain::model::AthleteCandidateId>,
    inventory: &[RetainedClaimV1],
) -> StoreResult<LocalMembershipWithholdingRecordV2> {
    let binding = capture.graph.capture();
    Ok(LocalMembershipWithholdingRecordV2 {
        version: 2,
        policy: LOCAL_MEMBERSHIP_WITHHOLDING_GRAPH_POLICY.into(),
        store: writer.identity.clone(),
        operation: operation.into(),
        expected_head: head.clone(),
        evidence_generation: binding.evidence_generation(),
        derived_generation: binding.derived_generation(),
        snapshot_sequence: binding.snapshot_sequence(),
        graph: capture.graph.domain_reference()?,
        members,
        dispositions: dispositions(&request, inventory)?,
        request,
        observed_at: capture.observed_at.clone(),
    })
}

pub(super) fn request_digest(
    identity: &census_domain::model::StoreIdentity,
    operation: &str,
    request: &LocalMembershipWithholdV1,
) -> StoreResult<String> {
    graph_operation::raw_operation(identity, operation)?;
    codec::encode(request, 1024 * 1024)?;
    Ok(serialized_digest(&(
        "local-source-membership-withholding-request/v2",
        identity,
        operation,
        request,
    ))?)
}
fn dispositions(
    request: &LocalMembershipWithholdV1,
    claims: &[RetainedClaimV1],
) -> StoreResult<Vec<RetainedClaimDispositionV1>> {
    let mut result = Vec::new();
    result
        .try_reserve(claims.len())
        .map_err(|error| crate::StoreError::Refused {
            detail: format!("withholding disposition allocation: {error}"),
        })?;
    let mut remaining: usize = 1024 * 1024;
    for claim in claims {
        let target = claim.section == RetainedIdentitySectionV1::CandidateMembership
            && claim.ordinal == request.attachment.sequence
            && claim.row_digest == request.attachment.digest
            && claim.claim_path == request.attachment.membership_id;
        let disposition = RetainedClaimDispositionV1 {
            claim: claim.clone(),
            treatment: if target {
                ClaimTreatmentV1::RejectIdentityCorroboration
            } else {
                ClaimTreatmentV1::RetainAsContext
            },
            rationale: request.rationale.clone(),
        };
        remaining = remaining
            .checked_sub(codec::encode(&disposition, remaining)?.len())
            .ok_or(crate::StoreError::CounterOverflow)?;
        result.push(disposition);
    }
    Ok(result)
}

pub(super) fn prepare_history(
    record: &census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV1,
    claims: &[RetainedClaimV1],
    projection: &HistoryProjection,
) -> StoreResult<PreparedHistoryRecord> {
    codec::encode(record, 1024 * 1024)?;
    let request = record
        .history_request(claims)
        .map_err(PublicIdentityError::from)?;
    let history = prepare_next(
        &record.store,
        &record.expected_head,
        request,
        ReplayLimits::SMALL,
    )
    .map_err(PublicIdentityError::from)?;
    let after = projection
        .clone()
        .extend_candidate(&history.uncommitted_record)
        .map_err(PublicIdentityError::from)?;
    if !history.established_ids.is_empty()
        || after.withheld_memberships(history.next_head.sequence)
            != Some(record.request.expected_withheld_memberships.as_slice())
    {
        return Err(PublicIdentityError::Corrupt("withholding computed member set").into());
    }
    Ok(history)
}

pub(super) fn rebuild_history(
    store: &Store,
    graph: &crate::public_identity::graph_reference::GraphAuthorityReferenceV1,
    record: &LocalMembershipWithholdingRecordV2,
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
    crate::public_identity::graph_witness::manual::rebuild_withholding(
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
    record: &LocalMembershipWithholdingRecordV2,
    history: &census_domain::public_identity_history::PreparedHistoryRecord,
) -> StoreResult<HeldGraphWitness> {
    let approval = super::consume::graph_approval_bytes(record, &capture.graph)?;
    HeldGraphWitness::new(
        operation,
        super::consume::graph_approval_digest(&approval),
        capture.graph.clone(),
        &capture.observed_at,
        history,
        Payload::Withholding {
            actor: LocalActor::LocalOperator,
            adjudication: record.clone(),
        },
    )
}
