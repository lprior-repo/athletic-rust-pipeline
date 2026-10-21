use super::*;
use crate::public_identity_tests::support::{fixture, retained_state, subject, TestResult};
use crate::{PublicIdentityError, Store, StoreError, Table};
use census_domain::public_identity_history::PublicResolution;
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

mod budgets;
mod graph;
mod tamper;

pub(super) fn two_births() -> TestResult<(
    tempfile::TempDir,
    Store,
    crate::PublicIdentityWriterV1,
    census_domain::model::AthleteCandidateId,
)> {
    let (directory, store, writer) = fixture()?;
    let first = subject("frozen-first", "501")?;
    let second = subject("frozen-second", "502")?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    writer.establish_source_bound(
        &store,
        "frozen-first",
        &first.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    writer.establish_source_bound(
        &store,
        "frozen-second",
        &second.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    Ok((directory, store, writer, first.id.cast()))
}

#[test]
fn frozen_source_identity_replays_after_live_changes_and_store_close() -> TestResult {
    let (directory, store, writer, first) = two_births()?;
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted snapshot")?;
    let original = accepted.resolve_candidate(&first)?;
    let before = retained_state(&store)?;
    let frozen = snapshot.freeze_public_identity_v2()?;
    check!(eq; retained_state(&store)?, before);
    let (operations, objects) = graph::inspect(frozen.bytes())?;
    check!(eq; operations.len(), 2);
    check!(!objects.is_empty());
    for operation in operations {
        let held = crate::public_identity::graph_witness::HeldGraphWitness::decode(
            "frozen graph source witness",
            operation.witness.as_bytes(),
        )?;
        check!(eq; held.version, 8);
        check!(matches!(
            held.payload,
            crate::public_identity::graph_witness::Payload::Source { .. }
        ));
    }
    graph::assert_exact_inventory(&store, frozen.bytes())?;
    let later = subject("frozen-later", "503")?;
    store.append(Table::Athletes, &later)?;
    writer.establish_source_bound(
        &store,
        "frozen-later",
        &later.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(eq; writer.head(&store)?.sequence, 3);
    check!(store.snapshot().sequence() > accepted.snapshot_sequence());
    drop(snapshot);
    drop(writer);
    drop(store);
    let replayed = replay_frozen_public_identity_v2(frozen.bytes(), frozen.digest())?;
    check!(eq; replayed.qualification_version(), PublicIdentityQualificationVersion::RetainedAuthorityGraphV3);
    check!(eq; replayed.store_identity(), accepted.store_identity());
    check!(eq; replayed.head(), accepted.head());
    check!(eq; replayed.snapshot_sequence(), accepted.snapshot_sequence());
    check!(eq; replayed.resolve_candidate(&first)?, original);
    check!(eq; replayed.resolve_candidate(&later.id.cast())?, PublicResolution::Unknown);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    check!(eq; writer.head(&store)?.sequence, 3);
    check!(eq; frozen.replay()?.head().sequence, 2);
    Ok(())
}

#[test]
fn frozen_bundle_copies_only_referenced_graph_objects() -> TestResult {
    let (_directory, store, writer, _) = two_births()?;
    let later = subject("unreferenced-graph", "504")?;
    store.append(Table::Athletes, &later)?;
    let operation = crate::public_identity::paged_generation::RawGenerationOperationIdV2::new(
        "unreferenced-frozen-graph",
    )?;
    let orphan = writer.capture_raw_generation_v2(&store, &operation)?;
    let before = retained_state(&store)?;
    let frozen = store.snapshot().freeze_public_identity_v2()?;
    let (operations, objects) = graph::inspect(frozen.bytes())?;
    check!(eq; operations.len(), 2);
    check!(!objects.contains_key(orphan.retained().seal().root().as_str()));
    graph::assert_exact_inventory(&store, frozen.bytes())?;
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn frozen_reviewed_merge_keeps_historical_birth_ids_and_redirects() -> TestResult {
    let fixture = crate::public_identity::context::reviewed_fixture::ready_for_merge()?;
    let outcome = fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "frozen-reviewed",
        &fixture.request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let expected = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("merged snapshot")?;
    let frozen = fixture.store.snapshot().freeze_public_identity_v2()?;
    let replayed = frozen.replay()?;
    let (operations, objects) = graph::inspect(frozen.bytes())?;
    check!(eq; operations.len(), 4);
    check!(!objects.is_empty());
    let witnesses = operations
        .iter()
        .map(|operation| {
            crate::public_identity::graph_witness::HeldGraphWitness::decode(
                "frozen graph reviewed witness",
                operation.witness.as_bytes(),
            )
        })
        .collect::<crate::StoreResult<Vec<_>>>()?;
    check!(eq; witnesses.iter().map(|held| held.version).collect::<Vec<_>>(), vec![8, 8, 8, 9]);
    for held in witnesses.iter().take(3) {
        check!(matches!(
            held.payload,
            crate::public_identity::graph_witness::Payload::Source { .. }
        ));
    }
    check!(matches!(
        witnesses.last().ok_or("reviewed witness")?.payload,
        crate::public_identity::graph_witness::Payload::Reviewed { .. }
    ));
    graph::assert_exact_inventory(&fixture.store, frozen.bytes())?;
    expected.require_same_projection(&replayed)?;
    check!(eq; replayed.head(), outcome.head());
    for birth in &fixture.births {
        check!(eq; replayed.resolve_public(birth.public_id())?, expected.resolve_public(birth.public_id())?);
    }
    check!(matches!(
        replayed.resolve_public(fixture.births.get(1).ok_or("second birth")?.public_id())?,
        PublicResolution::Redirect { .. }
    ));
    let later = subject("reviewed-frozen-later", "505")?;
    fixture.store.append(Table::Athletes, &later)?;
    check!(eq; frozen.replay()?.head(), outcome.head());
    Ok(())
}

#[test]
fn historical_v1_fixture_is_refused_without_a_lineage() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("historical");
    check!(matches!(
        crate::public_identity::init::create_historical_test_fixture(
            &root,
            PublicIdentityQualificationVersion::SourceBoundV1,
        ),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "removed public identity V1 declaration"
        )))
    ));
    Ok(())
}

#[test]
fn current_empty_graph_ledger_freezes_with_exact_empty_object_inventory() -> TestResult {
    let (_directory, store, _writer) = fixture()?;
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("snapshot")?;
    let before = retained_state(&store)?;
    let frozen = snapshot.freeze_public_identity_v2()?;
    let (operations, objects) = graph::inspect(frozen.bytes())?;
    check!(operations.is_empty());
    check!(objects.is_empty());
    check!(eq; frozen.replay()?.head(), accepted.head());
    check!(eq; frozen.replay()?.snapshot_sequence(), accepted.snapshot_sequence());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
