use super::*;
use crate::public_identity::{
    authority::graph,
    commit,
    graph_witness::{Decision, Payload},
    local_adjudication::admission::graph::{
        load_graph, reserve_graph, GraphPlanBuilder, GraphReservation,
    },
    paged_generation::{admission::Node, capture::verify, RawGenerationCaptureOutcomeV2},
};
use crate::public_identity_tests::support::{fixture, subject, TestResult};
use crate::{StoreError, Table, MAX_JOURNAL_VALUE_BYTES};
use census_domain::model::AthleteCandidateId;
use census_domain::public_identity_history::{
    prepare_next, CandidateDecisionReference, HistoryRequest, PublicResolution, ReplayLimits,
};
use fjall::{PersistMode, Readable};
use serde_json::Value;

const HEADER_KEY: &str = "manual-context-admission-v1\0header";
const RESERVATION_PREFIX: &str = "manual-context-admission-v1\0reservation\0";
const RESERVATION_LIMIT: u64 = 4096;
const BYTE_LIMIT: u64 = 128 * 1024 * 1024;
const ROOT_LIMIT: u64 = 4096;
const SNAPSHOT_ROW_LIMIT: usize = 8192;
const COMMITTED: &str = "capacity-committed-source";
const LAST: &str = "capacity-last-preparation";
const OVERFLOW: &str = "capacity-overflow-preparation";

fn reservation_key(operation: &str) -> String {
    format!(
        "{RESERVATION_PREFIX}{}",
        crate::journal::object_digest(operation.as_bytes())
    )
}

fn number(value: &Value, field: &str) -> TestResult<u64> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("missing admission number {field}").into())
}

fn header(store: &Store) -> TestResult<Value> {
    let raw = store.journal.get(HEADER_KEY)?.ok_or("admission header")?;
    Ok(codec::decode(HEADER_KEY, &raw)?)
}

fn metadata_bytes(key: &str, raw: &[u8]) -> TestResult<u64> {
    Ok(u64::try_from(
        key.len().checked_add(raw.len()).ok_or("metadata length")?,
    )?)
}

fn add(left: u64, right: u64) -> TestResult<u64> {
    left.checked_add(right)
        .ok_or_else(|| "counter overflow".into())
}

fn row_charges(key: &str, raw: &[u8]) -> TestResult<(u64, u64)> {
    let value: Value = codec::decode(key, raw)?;
    let metadata = metadata_bytes(key, raw)?;
    match number(&value, "version")? {
        3 => {
            let _: GraphReservation = codec::decode(key, raw)?;
            let descriptors = value
                .get("descriptors")
                .and_then(Value::as_array)
                .ok_or("graph descriptors")?;
            descriptors
                .iter()
                .try_fold((metadata, 0), |(bytes, roots), entry| {
                    let charge = entry.get("incremental").ok_or("graph charge")?;
                    Ok((
                        add(bytes, number(charge, "bytes")?)?,
                        add(roots, number(charge, "roots")?)?,
                    ))
                })
        }
        4 => {
            let _: Preparation = codec::decode(key, raw)?;
            Ok((metadata, 0))
        }
        _ => Err("unexpected admission fixture version".into()),
    }
}

fn retained_charges(store: &Store) -> TestResult<(u64, u64, u64)> {
    store
        .db
        .snapshot()
        .prefix(&store.journal, RESERVATION_PREFIX)
        .take(4097)
        .try_fold((4096, 0, 0), |(bytes, roots, count), guard| {
            let (key, raw) = guard.into_inner()?;
            let (added_bytes, added_roots) = row_charges(std::str::from_utf8(&key)?, &raw)?;
            Ok((
                add(bytes, added_bytes)?,
                add(roots, added_roots)?,
                add(count, 1)?,
            ))
        })
}

fn source_preparation(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    operation: &str,
    candidate: &AthleteCandidateId,
) -> TestResult<Preparation> {
    let expected = store.snapshot().revision();
    let head = writer.head(store)?;
    let digest = witness::request_digest(writer.store_identity(), operation, candidate)?;
    let capture = begin(writer, store, operation, &digest, expected, &head)?;
    let snapshot = store.snapshot();
    let view = VerifiedRawGraph::verify(&snapshot, capture.graph.capture(), capture.graph.seal())?;
    let (accepted, source) = graph::accept_source(&view, candidate, &capture.observed_at)?;
    let evidence = CandidateDecisionReference::from_accepted(&accepted, ReplayLimits::SMALL)?;
    let prepared = prepare_next(
        writer.store_identity(),
        &head,
        HistoryRequest::Establish {
            binding: commit::source_binding(candidate, source, &digest),
            evidence,
        },
        ReplayLimits::SMALL,
    )?;
    let [public_id] = prepared.established_ids.as_slice() else {
        return Err("source public ID cardinality".into());
    };
    let held = HeldGraphWitness::new(
        operation,
        digest.clone(),
        capture.graph.clone(),
        &capture.observed_at,
        &prepared,
        Payload::Source {
            candidate: candidate.clone(),
            decision: Decision::new(accepted.record())?,
            public_id: public_id.as_str().into(),
        },
    )?;
    Ok(Preparation {
        version: 4,
        store: writer.store_identity().clone(),
        operation: operation.into(),
        request_digest: digest,
        expected_head: head,
        evidence_generation: expected.evidence_generation(),
        derived_generation: expected.derived_generation(),
        raw_operation: raw_operation(writer.store_identity(), operation)?,
        graph: capture.graph,
        observed_at: capture.observed_at,
        witness: held,
    })
}

fn zero_charge_template(
    store: &Store,
    reference: &GraphAuthorityReferenceV1,
) -> TestResult<(RawGenerationOperationIdV2, GraphReservation)> {
    let mut builder = GraphPlanBuilder::new(reference.capture().clone());
    let seal = verify::verify(
        &store.snapshot(),
        reference.capture(),
        reference.seal(),
        |node, _| match node {
            Node::Seal(_) => Ok(()),
            node => builder.add(store, node).map(|_| ()),
        },
    )?;
    let plan = builder.finish(store, &seal)?;
    let operation = raw_operation(reference.capture().store_identity(), "capacity-template")?;
    let _guard = store.lock_appends();
    let reservation = reserve_graph(store, &operation, plan)?;
    assert!(reservation.is_complete(store)?);
    let raw = codec::encode(&reservation, MAX_JOURNAL_VALUE_BYTES)?;
    let key = reservation_key(operation.as_str());
    assert_eq!(row_charges(&key, &raw)?, (metadata_bytes(&key, &raw)?, 0));
    Ok((operation, reservation))
}

fn fill_valid_reservation_rows(store: &Store, template: &GraphReservation) -> TestResult {
    let _guard = store.lock_appends();
    let mut header = header(store)?;
    let (mut bytes, roots, mut count) = retained_charges(store)?;
    assert_eq!(
        (
            number(&header, "bytes")?,
            number(&header, "roots")?,
            number(&header, "reservations")?
        ),
        (bytes, roots, count)
    );
    let mut batch = store.db.batch();
    let target = RESERVATION_LIMIT
        .checked_sub(1)
        .ok_or("reservation limit")?;
    for index in count..target {
        let operation = raw_operation(
            template.capture().store_identity(),
            &format!("capacity-fixture-{index:04}"),
        )?;
        let key = reservation_key(operation.as_str());
        assert!(store.journal.get(&key)?.is_none());
        let mut value = serde_json::to_value(template)?;
        let field = value
            .pointer_mut("/purpose/operation")
            .ok_or("raw operation")?;
        *field = serde_json::to_value(&operation)?;
        let candidate: GraphReservation = serde_json::from_value(value)?;
        let limit = MAX_JOURNAL_VALUE_BYTES
            .checked_sub(key.len())
            .ok_or("row limit")?;
        let raw = codec::encode(&candidate, limit)?;
        assert_eq!(row_charges(&key, &raw)?, (metadata_bytes(&key, &raw)?, 0));
        bytes = add(bytes, metadata_bytes(&key, &raw)?)?;
        count = add(count, 1)?;
        batch.insert(&store.journal, key.as_bytes(), raw);
    }
    assert_eq!(count, 4095);
    assert!(bytes < BYTE_LIMIT);
    assert!(roots < ROOT_LIMIT);
    *header.get_mut("bytes").ok_or("header bytes")? = bytes.into();
    *header.get_mut("reservations").ok_or("header count")? = count.into();
    batch.insert(&store.journal, HEADER_KEY, codec::encode(&header, 4096)?);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

fn unchanged(store: &Store, before: &fjall::Snapshot, sequence: u64) -> TestResult {
    for keyspace in [
        &store.entities,
        &store.journal,
        &store.meta,
        &store.receipts,
    ] {
        let after = store.db.snapshot();
        let mut left = before.iter(keyspace);
        let mut right = after.iter(keyspace);
        let mut complete = false;
        for _ in 0..SNAPSHOT_ROW_LIMIT {
            match (left.next(), right.next()) {
                (Some(left), Some(right)) => assert_eq!(left.into_inner()?, right.into_inner()?),
                (None, None) => {
                    complete = true;
                    break;
                }
                _ => return Err("physical keyspace row count changed".into()),
            }
        }
        assert!(complete, "physical snapshot comparison bound");
    }
    assert_eq!(store.snapshot().sequence(), sequence);
    Ok(())
}

fn unchanged_journal_prefix(store: &Store, before: &fjall::Snapshot, prefix: &[u8]) -> TestResult {
    let after = store.db.snapshot();
    let mut left = before.prefix(&store.journal, prefix);
    let mut right = after.prefix(&store.journal, prefix);
    for _ in 0..SNAPSHOT_ROW_LIMIT {
        match (left.next(), right.next()) {
            (Some(left), Some(right)) => assert_eq!(left.into_inner()?, right.into_inner()?),
            (None, None) => return Ok(()),
            _ => return Err("journal prefix row count changed".into()),
        }
    }
    Err("journal prefix comparison bound".into())
}

fn finish_pending_source_at_capacity(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    reserved: &Preparation,
) -> TestResult {
    let Payload::Source {
        candidate,
        public_id,
        ..
    } = &reserved.witness.payload
    else {
        return Err("pending source witness".into());
    };
    let before = store.db.snapshot();
    assert_eq!(writer.head(store)?, reserved.expected_head);
    let resumed = writer.establish_source_bound(
        store,
        &reserved.operation,
        candidate,
        store.snapshot().revision(),
        &reserved.expected_head,
    )?;
    assert!(!resumed.replayed());
    assert_eq!(resumed.public_id().as_str(), public_id.as_str());
    assert_eq!(
        resumed.head(),
        &RegistryHead {
            sequence: add(reserved.expected_head.sequence, 1)?,
            digest: reserved.witness.record_digest.clone(),
        }
    );
    let projection = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("accepted identity snapshot")?;
    assert_eq!(projection.head(), resumed.head());
    let PublicResolution::Canonical(resolved) = projection.resolve_candidate(candidate)? else {
        return Err("resumed source is not canonical".into());
    };
    assert_eq!(&resolved.public_id, resumed.public_id());
    assert_eq!(&resolved.representative_candidate, candidate);
    unchanged_journal_prefix(store, &before, b"manual-context-admission-v1\0")?;
    unchanged_journal_prefix(store, &before, b"j2\0")?;
    let committed = store.db.snapshot();
    let sequence = store.snapshot().sequence();
    let replayed = writer.establish_source_bound(
        store,
        &reserved.operation,
        candidate,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    assert!(replayed.replayed());
    assert_eq!(replayed.public_id(), resumed.public_id());
    unchanged(store, &committed, sequence)
}

fn capacity_refusal<T>(result: StoreResult<T>) -> TestResult {
    match result {
        Err(StoreError::PublicIdentity(PublicIdentityError::AdmissionCapacity {
            resource: crate::PublicIdentityAdmissionResource::Reservations,
            needed: 4097,
            available: 4096,
        })) => Ok(()),
        Err(error) => Err(format!("unexpected refusal: {error}").into()),
        Ok(_) => Err("exhausted admission accepted new operation".into()),
    }
}

#[test]
fn valid_reservation_cap_refuses_new_graph_and_preparation_but_replays_existing() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let first = subject("capacity-first", "9851")?;
    let last = subject("capacity-last", "9852")?;
    let overflow = subject("capacity-overflow", "9853")?;
    store.append_many(
        Table::Athletes,
        &[first.clone(), last.clone(), overflow.clone()],
    )?;
    let committed = writer.establish_source_bound(
        &store,
        COMMITTED,
        &first.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let final_preparation = source_preparation(&store, &writer, LAST, &last.id.cast())?;
    let refused_preparation = source_preparation(&store, &writer, OVERFLOW, &overflow.id.cast())?;
    let (template_operation, template) = zero_charge_template(&store, &final_preparation.graph)?;
    fill_valid_reservation_rows(&store, &template)?;
    let audited = load_graph(&store, writer.store_identity(), &template_operation)?
        .ok_or("audited template")?;
    assert_eq!(serialized_digest(&audited)?, serialized_digest(&template)?);
    let before = header(&store)?;
    assert_eq!(number(&before, "reservations")?, 4095);
    let final_reserved = {
        let _guard = store.lock_appends();
        preparation::reserve(&store, final_preparation.clone())?
    };
    let key = reservation_key(LAST);
    let raw = store.journal.get(&key)?.ok_or("last preparation")?;
    let full = header(&store)?;
    assert_eq!(number(&full, "reservations")?, RESERVATION_LIMIT);
    assert_eq!(
        number(&full, "bytes")?,
        add(number(&before, "bytes")?, metadata_bytes(&key, &raw)?)?
    );
    assert_eq!(number(&full, "roots")?, number(&before, "roots")?);
    assert!(number(&full, "bytes")? < BYTE_LIMIT);
    assert!(number(&full, "roots")? < ROOT_LIMIT);
    let audited = preparation::load(&store, writer.store_identity(), LAST)?
        .ok_or("audited full-cap preparation")?;
    assert_eq!(
        serialized_digest(&audited)?,
        serialized_digest(&final_reserved)?
    );
    let baseline = store.db.snapshot();
    let sequence = store.snapshot().sequence();
    capacity_refusal(writer.establish_source_bound(
        &store,
        "capacity-new-source",
        &overflow.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    ))?;
    unchanged(&store, &baseline, sequence)?;
    {
        let _guard = store.lock_appends();
        capacity_refusal(preparation::reserve(&store, refused_preparation))?;
    }
    unchanged(&store, &baseline, sequence)?;
    let repeated = {
        let _guard = store.lock_appends();
        preparation::reserve(&store, final_preparation)?
    };
    assert_eq!(
        serialized_digest(&repeated)?,
        serialized_digest(&final_reserved)?
    );
    unchanged(&store, &baseline, sequence)?;
    let graph_replayed = writer.capture_raw_generation_v2(&store, &final_reserved.raw_operation)?;
    assert!(matches!(
        &graph_replayed,
        RawGenerationCaptureOutcomeV2::HistoricalReplay(_)
    ));
    assert_eq!(
        graph_replayed.retained().binding(),
        final_reserved.graph.capture()
    );
    assert_eq!(
        graph_replayed.retained().seal(),
        final_reserved.graph.seal()
    );
    unchanged(&store, &baseline, sequence)?;
    let replayed = writer.establish_source_bound(
        &store,
        COMMITTED,
        &first.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    assert!(replayed.replayed());
    assert_eq!(replayed.public_id(), committed.public_id());
    unchanged(&store, &baseline, sequence)?;
    finish_pending_source_at_capacity(&store, &writer, &final_reserved)?;
    Ok(())
}

#[test]
fn fabricated_full_count_is_corruption_and_never_valid_exhaustion() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let mut invalid = header(&store)?;
    assert_eq!(number(&invalid, "reservations")?, 0);
    *invalid.get_mut("reservations").ok_or("header count")? = RESERVATION_LIMIT.into();
    let mut batch = store.db.batch();
    batch.insert(&store.journal, HEADER_KEY, codec::encode(&invalid, 4096)?);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    let baseline = store.db.snapshot();
    let sequence = store.snapshot().sequence();
    capacity_refusal(preparation::load(
        &store,
        writer.store_identity(),
        "missing-operation",
    ))?;
    unchanged(&store, &baseline, sequence)?;
    Ok(())
}
