use super::*;
use crate::public_identity_tests::support::{fixture, retained_state, subject, TestResult};
use crate::{PublicIdentityError, Store, StoreError, Table};
use census_domain::public_identity_history::PublicResolution;
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

mod budgets;
mod tamper;

fn two_births() -> TestResult<(
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
    let frozen = snapshot.freeze_public_identity_v1()?;
    check!(eq; retained_state(&store)?, before);
    let wire: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    check!(eq; wire.operations.len(), 2);
    check!(eq; wire.contexts.len(), 1);
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
    let replayed = replay_frozen_public_identity_v1(frozen.bytes(), frozen.digest())?;
    check!(eq; replayed.qualification_version(), PublicIdentityQualificationVersion::RetainedAuthorityContextV2);
    check!(eq; replayed.store_identity(), accepted.store_identity());
    check!(eq; replayed.head(), accepted.head());
    check!(eq; replayed.snapshot_sequence(), accepted.snapshot_sequence());
    check!(eq; replayed.resolve_candidate(&first)?, original);
    check!(eq; replayed.resolve_candidate(&later.id.cast())?, PublicResolution::Unknown);
    let (store, writer) =
        Store::open_public_identity_qualification_v2(directory.path().join("identity"))?
            .into_parts();
    check!(eq; writer.head(&store)?.sequence, 3);
    check!(eq; frozen.replay()?.head().sequence, 2);
    Ok(())
}

#[test]
fn frozen_bundle_copies_only_referenced_contexts() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let later = subject("unreferenced-context", "504")?;
    store.append(Table::Athletes, &later)?;
    let inputs = crate::public_identity::context::Inputs::capture(&store.snapshot())?;
    let orphan = inputs.stage(&store, &store.store_identity()?)?;
    let before = retained_state(&store)?;
    let frozen = store.snapshot().freeze_public_identity_v1()?;
    let bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    check!(eq; bundle.contexts.len(), 1);
    check!(!bundle
        .contexts
        .iter()
        .any(|context| context.root == orphan.object_id()));
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
    let frozen = fixture.store.snapshot().freeze_public_identity_v1()?;
    let replayed = frozen.replay()?;
    let bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    check!(eq; bundle.operations.len(), 3);
    let versions = bundle
        .operations
        .iter()
        .map(|operation| -> TestResult<u32> {
            let witness: serde_json::Value = serde_json::from_str(&operation.witness)?;
            let version = witness
                .get("version")
                .and_then(serde_json::Value::as_u64)
                .ok_or("version")?;
            Ok(u32::try_from(version)?)
        })
        .collect::<TestResult<Vec<_>>>()?;
    check!(eq; versions, vec![2, 2, 3]);
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
fn historical_v1_remains_readable_but_cannot_freeze_as_current_authority() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("historical");
    Store::create_public_identity_qualification_v1(&root)?;
    let (store, _writer) = Store::open_public_identity_qualification_v1(&root)?.into_parts();
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("historical snapshot")?;
    check!(eq; accepted.qualification_version(), PublicIdentityQualificationVersion::SourceBoundV1);
    let before = retained_state(&store)?;
    check!(matches!(
        store.snapshot().freeze_public_identity_v1(),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "frozen identity requires retained authority V2"
        )))
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn current_empty_ledger_freezes_with_exact_empty_authority_inventory() -> TestResult {
    let (_directory, store, _writer) = fixture()?;
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("snapshot")?;
    let before = retained_state(&store)?;
    let frozen = snapshot.freeze_public_identity_v1()?;
    let bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    check!(bundle.operations.is_empty());
    check!(bundle.contexts.is_empty());
    check!(eq; frozen.replay()?.head(), accepted.head());
    check!(eq; frozen.replay()?.snapshot_sequence(), accepted.snapshot_sequence());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
