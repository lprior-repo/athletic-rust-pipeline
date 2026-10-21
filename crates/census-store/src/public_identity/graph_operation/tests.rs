use super::*;
use crate::public_identity::{
    authority::graph,
    commit,
    graph_witness::{Decision, Payload},
    paged_generation::{RawGenerationCaptureOutcomeV2, VerifiedRawGraph},
};
use crate::public_identity_tests::support::{fixture, retained_state, subject, TestResult};
use crate::Table;
use census_domain::model::AthleteCandidateId;
use census_domain::public_identity_history::{
    prepare_next, CandidateDecisionReference, HistoryRequest, ReplayLimits,
};
use serde_json::Value;

const HEADER_KEY: &str = "manual-context-admission-v1\0header";
fn header(store: &Store) -> TestResult<Value> {
    let raw = store.journal.get(HEADER_KEY)?.ok_or("admission header")?;
    Ok(serde_json::from_slice(&raw)?)
}
fn number(value: &Value, key: &str) -> TestResult<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("header {key}").into())
}
fn stage_source_preparation(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    operation: &str,
    candidate: &AthleteCandidateId,
) -> TestResult<(Preparation, Vec<String>)> {
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
        return Err("source ID cardinality".into());
    };
    let witness = HeldGraphWitness::new(
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
    let _guard = store.lock_appends();
    require_current(writer, store, operation, expected, &head, &capture.graph)?;
    let before = header(store)?;
    let reserved = reserve(
        store,
        operation,
        &digest,
        expected,
        &head,
        &capture,
        witness.clone(),
    )?;
    let after = header(store)?;
    let key = format!(
        "manual-context-admission-v1\0reservation\0{}",
        crate::journal::object_digest(operation.as_bytes())
    );
    let raw = store.journal.get(&key)?.ok_or("operation preparation")?;
    let metadata = u64::try_from(
        key.len()
            .checked_add(raw.len())
            .ok_or("metadata overflow")?,
    )?;
    check!(eq;
        number(&after, "bytes")?,
        number(&before, "bytes")?
            .checked_add(metadata)
            .ok_or("charge overflow")?
    );
    check!(eq;number(&after, "roots")?, number(&before, "roots")?);
    check!(eq;
        number(&after, "reservations")?,
        number(&before, "reservations")?
            .checked_add(1)
            .ok_or("reservation overflow")?
    );
    let state = retained_state(store)?;
    let sequence = store.snapshot_locked().sequence();
    let repeated = reserve(
        store, operation, &digest, expected, &head, &capture, witness,
    )?;
    check!(eq;serialized_digest(&reserved)?, serialized_digest(&repeated)?);
    check!(eq;state, retained_state(store)?);
    check!(eq;sequence, store.snapshot_locked().sequence());
    Ok((
        reserved,
        prepared
            .established_ids
            .iter()
            .map(|id| id.as_str().into())
            .collect(),
    ))
}

#[test]
fn raw_capture_identity_is_store_and_logical_operation_bound_only() -> TestResult {
    let (_first_dir, first, first_writer) = fixture()?;
    let (_second_dir, _second, second_writer) = fixture()?;
    let id = raw_operation(first_writer.store_identity(), "one-logical-operation")?;
    check!(eq;
        id,
        raw_operation(first_writer.store_identity(), "one-logical-operation")?
    );
    check!(ne;
        id,
        raw_operation(first_writer.store_identity(), "another-logical-operation")?
    );
    check!(ne;
        id,
        raw_operation(second_writer.store_identity(), "one-logical-operation")?
    );
    check!(raw_operation(first_writer.store_identity(), "").is_err());
    check!(raw_operation(first_writer.store_identity(), "operation\n").is_err());
    first.append(Table::Athletes, &subject("changed-physical-input", "8341")?)?;
    check!(eq;
        id,
        raw_operation(first_writer.store_identity(), "one-logical-operation")?
    );
    Ok(())
}

#[test]
fn preparation_four_charges_only_metadata_once_and_resumes_after_reopen() -> TestResult {
    let (directory, store, writer) = fixture()?;
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    let athlete = subject("prepared-source", "8342")?;
    store.append(Table::Athletes, &athlete)?;
    let candidate = athlete.id.cast();
    let (reserved, ids) =
        stage_source_preparation(&store, &writer, "crash-after-preparation-four", &candidate)?;
    check!(eq;reserved.version, 4);
    check!(eq;writer.head(&store)?.sequence, 0);
    let before = header(&store)?;
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    let outcome = writer.establish_source_bound(
        &store,
        "crash-after-preparation-four",
        &candidate,
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(eq;ids, vec![outcome.public_id().as_str().to_owned()]);
    check!(eq;before, header(&store)?);
    let state = retained_state(&store)?;
    let replayed = writer.establish_source_bound(
        &store,
        "crash-after-preparation-four",
        &candidate,
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(replayed.replayed());
    check!(eq;replayed.public_id(), outcome.public_id());
    check!(eq;state, retained_state(&store)?);
    Ok(())
}

#[test]
fn preparation_four_rejects_changed_request_without_replacement_capture_or_charge() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    let first = subject("prepared-first", "8343")?;
    let second = subject("prepared-second", "8344")?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    stage_source_preparation(&store, &writer, "same-operation", &first.id.cast())?;
    let state = retained_state(&store)?;
    let sequence = store.snapshot().sequence();
    check!(matches!(
        writer.establish_source_bound(
            &store,
            "same-operation",
            &second.id.cast(),
            store.snapshot().revision(),
            &writer.head(&store)?
        ),
        Err(crate::StoreError::PublicIdentity(
            PublicIdentityError::OperationConflict
        ))
    ));
    check!(eq;state, retained_state(&store)?);
    check!(eq;sequence, store.snapshot().sequence());
    Ok(())
}

#[test]
fn current_operation_refuses_stale_historical_capture_without_any_write() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    let first = subject("historic-capture", "8345")?;
    store.append(Table::Athletes, &first)?;
    let operation = "captured-before-new-evidence";
    let id = raw_operation(writer.store_identity(), operation)?;
    writer.capture_raw_generation_v2(&store, &id)?;
    store.append(Table::Athletes, &subject("new-source-evidence", "8346")?)?;
    let state = retained_state(&store)?;
    let historical = writer.capture_raw_generation_v2(&store, &id)?;
    check!(matches!(
        historical,
        RawGenerationCaptureOutcomeV2::HistoricalReplay(_)
    ));
    check!(writer
        .establish_source_bound(
            &store,
            operation,
            &first.id.cast(),
            store.snapshot().revision(),
            &writer.head(&store)?
        )
        .is_err());
    check!(eq;state, retained_state(&store)?);
    Ok(())
}

#[test]
fn pinned_raw_capture_rechecks_original_fence_before_first_reservation() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    store.append(Table::Athletes, &subject("pinned-before", "8347")?)?;
    let snapshot = store.snapshot();
    let expected = snapshot.revision();
    let original = binding::capture(&store, &snapshot)?;
    store.append(Table::Athletes, &subject("pinned-after", "8348")?)?;
    let state = retained_state(&store)?;
    check!(matches!(
        writer.capture_raw_generation_at_v2(
            &store,
            &raw_operation(writer.store_identity(), "pinned-no-writes")?,
            expected,
            &original
        ),
        Err(crate::StoreError::RevisionMoved { .. })
    ));
    check!(eq;state, retained_state(&store)?);
    Ok(())
}

#[test]
fn journal_only_staging_preserves_pinned_source_and_preparation_replay() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    let athlete = subject("journal-only", "8349")?;
    store.append(Table::Athletes, &athlete)?;
    let snapshot = store.snapshot();
    let expected = snapshot.revision();
    let original = binding::capture(&store, &snapshot)?;
    let before = snapshot.sequence();
    let id = raw_operation(writer.store_identity(), "journal-only-capture")?;
    let capture = writer.capture_raw_generation_at_v2(&store, &id, expected, &original)?;
    check!(store.snapshot().sequence() > before);
    check!(eq;capture.retained().binding(), &original);
    let state = retained_state(&store)?;
    let repeated = writer.capture_raw_generation_at_v2(&store, &id, expected, &original)?;
    check!(eq;capture.retained(), repeated.retained());
    check!(eq;state, retained_state(&store)?);
    writer.establish_source_bound(
        &store,
        "journal-only-capture",
        &athlete.id.cast(),
        expected,
        &writer.head(&store)?,
    )?;
    Ok(())
}
