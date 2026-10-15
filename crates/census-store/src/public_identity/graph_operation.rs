use super::{
    codec,
    graph_reference::GraphAuthorityReferenceV1,
    graph_witness::HeldGraphWitness,
    local_adjudication::admission::preparation::{self, Preparation},
    paged_generation::{capture::binding, RawGenerationOperationIdV2, VerifiedRawGraph},
    qualification::Ledger,
    retained::{GraphRead, Rows},
    witness, PublicIdentityError, PublicIdentityWriterV1,
};
use crate::clock::{Clock, SystemClock};
use crate::{Store, StoreResult, StoreRevision};
use census_domain::model::{serialized_digest, StoreIdentity};
use census_domain::public_identity_history::{HistoryRecord, RegistryHead};
use std::collections::BTreeSet;

pub(super) struct Capture {
    pub graph: GraphAuthorityReferenceV1,
    pub observed_at: String,
    pub reserved: Option<Preparation>,
}

pub(super) fn raw_operation(
    identity: &StoreIdentity,
    operation: &str,
) -> StoreResult<RawGenerationOperationIdV2> {
    if operation.is_empty() || operation.len() > 256 || operation.chars().any(char::is_control) {
        return Err(PublicIdentityError::OperationConflict.into());
    }
    let digest = serialized_digest(&(
        "public-identity-raw-operation/v1",
        identity,
        operation,
        "identity-authority",
    ))?;
    RawGenerationOperationIdV2::new(&format!("identity-authority-{digest}"))
}

pub(super) fn begin(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    operation: &str,
    digest: &str,
    expected: StoreRevision<'_>,
    head: &RegistryHead,
) -> StoreResult<Capture> {
    let guard = store.lock_appends();
    expected.ensure_origin(store, operation)?;
    writer.require_current_qualification()?;
    expected.ensure_current(store, operation)?;
    let ledger = writer.verified(store)?;
    if head != ledger.history.projection().head() {
        return Err(PublicIdentityError::HeadMoved.into());
    }
    require_uncommitted(store, operation)?;
    let reserved = preparation::load(store, &writer.identity, operation)?;
    if let Some(prior) = &reserved {
        prior.require_request(&writer.identity, operation, digest, expected, head)?;
    }
    let original = binding::capture(store, &store.snapshot_locked())?;
    drop(guard);
    let retained = writer.capture_raw_generation_at_v2(
        store,
        &raw_operation(&writer.identity, operation)?,
        expected,
        &original,
    )?;
    let graph = GraphAuthorityReferenceV1::new(
        retained.retained().binding().clone(),
        retained.retained().seal().clone(),
    );
    binding::require_inputs(&original, graph.capture())?;
    if graph.capture().evidence_generation() != expected.evidence_generation()
        || graph.capture().derived_generation() != expected.derived_generation()
    {
        return Err(PublicIdentityError::Corrupt(
            "historical capture cannot authorize new operation",
        )
        .into());
    }
    if reserved.as_ref().is_some_and(|prior| prior.graph != graph) {
        return Err(PublicIdentityError::OperationConflict.into());
    }
    let _guard = store.lock_appends();
    require_current(writer, store, operation, expected, head, &graph)?;
    let snapshot = store.snapshot_locked();
    VerifiedRawGraph::verify(&snapshot, graph.capture(), graph.seal())?;
    let observed_at = match &reserved {
        Some(prior) => prior.observed_at.clone(),
        None => SystemClock.today_iso8601(),
    };
    Ok(Capture {
        graph,
        observed_at,
        reserved,
    })
}

pub(super) fn require_current(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    operation: &str,
    expected: StoreRevision<'_>,
    head: &RegistryHead,
    graph: &GraphAuthorityReferenceV1,
) -> StoreResult<Ledger> {
    writer.require_current_qualification()?;
    expected.ensure_current(store, operation)?;
    let ledger = writer.verified(store)?;
    if head != ledger.history.projection().head() {
        return Err(PublicIdentityError::HeadMoved.into());
    }
    let current = binding::capture(store, &store.snapshot_locked())?;
    binding::require_inputs(graph.capture(), &current)?;
    if graph.capture().store_identity() != &writer.identity
        || graph.capture().evidence_generation() != expected.evidence_generation()
        || graph.capture().derived_generation() != expected.derived_generation()
    {
        return Err(PublicIdentityError::Corrupt("operation original capture fence").into());
    }
    Ok(ledger)
}

pub(super) fn reserve(
    store: &Store,
    operation: &str,
    digest: &str,
    expected: StoreRevision<'_>,
    head: &RegistryHead,
    capture: &Capture,
    witness: HeldGraphWitness,
) -> StoreResult<Preparation> {
    let proposed = Preparation {
        version: 4,
        store: capture.graph.capture().store_identity().clone(),
        operation: operation.into(),
        request_digest: digest.into(),
        expected_head: head.clone(),
        evidence_generation: expected.evidence_generation(),
        derived_generation: expected.derived_generation(),
        raw_operation: raw_operation(capture.graph.capture().store_identity(), operation)?,
        graph: capture.graph.clone(),
        observed_at: capture.observed_at.clone(),
        witness,
    };
    if let Some(prior) = &capture.reserved {
        if serialized_digest(prior)? != serialized_digest(&proposed)? {
            return Err(PublicIdentityError::OperationConflict.into());
        }
    }
    preparation::reserve(store, proposed)
}

pub(super) fn require_available(
    reader: &impl GraphRead,
    identity: &StoreIdentity,
    records: &[HistoryRecord],
    held: &HeldGraphWitness,
) -> StoreResult<()> {
    let mut superseded = BTreeSet::new();
    records.iter().try_for_each(|record| {
        let key = witness::key(record.sequence);
        let raw = reader
            .row(Rows::Witness, &key)?
            .ok_or(PublicIdentityError::Corrupt(
                "missing correction fence witness",
            ))?;
        HeldGraphWitness::decode(&key, &raw)?.retain_superseded_material(reader, &mut superseded)
    })?;
    held.require_available(reader, identity, &superseded)
}

pub(super) fn inspect(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    operation: &str,
) -> StoreResult<(GraphAuthorityReferenceV1, Ledger)> {
    let guard = store.lock_appends();
    writer.require_current_qualification()?;
    let ledger = writer.verified(store)?;
    let head = ledger.history.projection().head().clone();
    let snapshot = store.snapshot_locked();
    let expected = snapshot.revision();
    let original = binding::capture(store, &snapshot)?;
    drop(guard);
    let capture = writer.capture_raw_generation_at_v2(
        store,
        &raw_operation(&writer.identity, operation)?,
        expected,
        &original,
    )?;
    let graph = GraphAuthorityReferenceV1::new(
        capture.retained().binding().clone(),
        capture.retained().seal().clone(),
    );
    binding::require_inputs(&original, graph.capture())?;
    let _guard = store.lock_appends();
    let ledger = require_current(writer, store, operation, expected, &head, &graph)?;
    VerifiedRawGraph::verify(&store.snapshot_locked(), graph.capture(), graph.seal())?;
    Ok((graph, ledger))
}

#[cfg(test)]
mod tests;

fn require_uncommitted(store: &Store, operation: &str) -> StoreResult<()> {
    if codec::load(
        &store.db.snapshot(),
        &store.receipts,
        &witness::receipt_key(operation),
        65_536,
    )?
    .is_some()
    {
        return Err(PublicIdentityError::OperationConflict.into());
    }
    Ok(())
}

pub(super) fn require_available_live(
    store: &Store,
    identity: &StoreIdentity,
    ledger: &Ledger,
    witness: &HeldGraphWitness,
) -> StoreResult<()> {
    let retained = store.db.snapshot();
    let live = super::retained::Live {
        snapshot: &retained,
        entities: &store.entities,
        meta: &store.meta,
        journal: &store.journal,
        receipts: &store.receipts,
    };
    require_available(&live, identity, ledger.history.records(), witness)
}

#[cfg(test)]
mod exhaustion_tests;
