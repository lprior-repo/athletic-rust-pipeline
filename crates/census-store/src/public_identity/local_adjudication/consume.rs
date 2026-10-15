use super::{LocalAdjudicationCommitOutcomeV1, PreparedLocalAdjudicationV1};
use crate::public_identity::{
    codec, graph_operation,
    graph_reference::GraphAuthorityReferenceV1,
    graph_witness::{HeldGraphWitness, LocalActor, Payload},
    local_adjudication::admission::preparation,
    operation, PublicIdentityError, PublicIdentityWriterV1,
};
use crate::{Store, StoreResult};
use census_domain::model::serialized_digest;
use census_domain::public_identity_history::local_adjudication::{
    LocalIdentityAdjudicationRecordV2, LocalIdentityAdjudicationRecordV3,
};
use census_domain::public_identity_history::PreparedHistoryRecord;
use serde::Serialize;
use sha2::{Digest, Sha256};
const APPROVAL_SCHEMA: &str = "local-identity-adjudication-approval/v1";
const GRAPH_APPROVAL_SCHEMA: &str = "local-identity-adjudication-approval/v2";
#[derive(Serialize)]
struct Envelope<'a> {
    schema: &'static str,
    actor: &'static str,
    prepared: &'a LocalIdentityAdjudicationRecordV2,
}
#[derive(Serialize)]
struct GraphEnvelope<'a> {
    schema: &'static str,
    actor: &'static str,
    prepared: &'a LocalIdentityAdjudicationRecordV3,
    graph: &'a GraphAuthorityReferenceV1,
}
pub(in crate::public_identity) struct ApprovedLocalAdjudicationV1 {
    pub record: LocalIdentityAdjudicationRecordV3,
    pub graph: GraphAuthorityReferenceV1,
    pub approval_digest: String,
    pub history: PreparedHistoryRecord,
}
pub(in crate::public_identity) fn graph_approval_bytes(
    record: &LocalIdentityAdjudicationRecordV3,
    graph: &GraphAuthorityReferenceV1,
) -> StoreResult<Vec<u8>> {
    graph.require_domain(&record.graph)?;
    codec::encode(
        &GraphEnvelope {
            schema: GRAPH_APPROVAL_SCHEMA,
            actor: "local_operator",
            prepared: record,
            graph,
        },
        1024 * 1024,
    )
}
pub(in crate::public_identity) fn graph_approval_digest(encoded: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(GRAPH_APPROVAL_SCHEMA.as_bytes());
    digest.update([0]);
    digest.update(encoded);
    format!("{:x}", digest.finalize())
}
pub(in crate::public_identity) fn consume(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    prepared: PreparedLocalAdjudicationV1<'_>,
    approval: (Vec<u8>, String),
) -> StoreResult<LocalAdjudicationCommitOutcomeV1> {
    let bytes = graph_approval_bytes(&prepared.record, &prepared.graph)?;
    if approval.0 != bytes || approval.1 != graph_approval_digest(&bytes) {
        return Err(PublicIdentityError::Corrupt("manual graph approval binding").into());
    }
    let _guard = store.lock_appends();
    prepared
        .expected
        .ensure_origin(store, &prepared.record.operation)?;
    writer.require_current_qualification()?;
    let ledger = writer.verified(store)?;
    match operation::repeated(store, &prepared.record.operation, &approval.1, &ledger)? {
        Some(operation::Replayed::Manual(outcome)) => return Ok(outcome),
        Some(_) => return Err(PublicIdentityError::OperationConflict.into()),
        None => (),
    }
    let ledger = graph_operation::require_current(
        writer,
        store,
        &prepared.record.operation,
        prepared.expected,
        &prepared.record.expected_head,
        &prepared.graph,
    )?;
    let history =
        super::prepare::rebuild_history(store, &prepared.graph, &prepared.record, &ledger)?;
    if history != prepared.history {
        return Err(PublicIdentityError::Corrupt("manual graph prepared history").into());
    }
    let witness = HeldGraphWitness::new(
        &prepared.record.operation,
        approval.1.clone(),
        prepared.graph.clone(),
        &prepared.record.observed_at,
        &history,
        Payload::Manual {
            actor: LocalActor::LocalOperator,
            adjudication: prepared.record.clone(),
        },
    )?;
    require_reserved(store, &prepared, &witness)?;
    super::persist::persist(
        store,
        ApprovedLocalAdjudicationV1 {
            record: prepared.record,
            graph: prepared.graph,
            history,
            approval_digest: approval.1,
        },
        ledger,
    )
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

fn require_reserved(
    store: &Store,
    prepared: &PreparedLocalAdjudicationV1<'_>,
    witness: &HeldGraphWitness,
) -> StoreResult<()> {
    let reserved = preparation::load(store, &prepared.record.store, &prepared.record.operation)?
        .ok_or(PublicIdentityError::Corrupt(
            "missing graph operation preparation",
        ))?;
    let digest = super::prepare::request_digest(
        &prepared.record.store,
        &prepared.record.operation,
        &prepared.record.request,
    )?;
    reserved.require_request(
        &prepared.record.store,
        &prepared.record.operation,
        &digest,
        prepared.expected,
        &prepared.record.expected_head,
    )?;
    if serialized_digest(&reserved.witness)? != serialized_digest(witness)? {
        return Err(PublicIdentityError::Corrupt("manual reserved approval binding").into());
    }
    Ok(())
}
