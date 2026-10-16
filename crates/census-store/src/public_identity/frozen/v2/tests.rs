use super::*;
use crate::public_identity_tests::support::{fixture, retained_state, subject, TestResult};
use crate::{Store, Table};
use census_domain::public_identity_history::PublicResolution;
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

mod tamper;

fn two_births() -> TestResult<(
    tempfile::TempDir,
    Store,
    crate::PublicIdentityWriterV1,
    census_domain::model::AthleteCandidateId,
)> {
    let (directory, store, writer) = fixture()?;
    let first = subject("graph-frozen-first", "701")?;
    let second = subject("graph-frozen-second", "702")?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    [&first, &second]
        .into_iter()
        .enumerate()
        .try_for_each(|(index, athlete)| {
            writer.establish_source_bound(
                &store,
                &format!("graph-freeze-{index}"),
                &athlete.id.cast(),
                store.snapshot().revision(),
                &writer.head(&store)?,
            )?;
            Ok::<(), crate::StoreError>(())
        })?;
    Ok((directory, store, writer, first.id.cast()))
}

#[test]
fn frozen_graph_replays_offline_after_live_changes_with_identical_projection() -> TestResult {
    let (directory, store, writer, first) = two_births()?;
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("snapshot")?;
    let before = retained_state(&store)?;
    let frozen = snapshot.freeze_public_identity_v2()?;
    check!(eq; retained_state(&store)?, before);
    let later = subject("graph-frozen-later", "703")?;
    store.append(Table::Athletes, &later)?;
    writer.establish_source_bound(
        &store,
        "graph-freeze-later",
        &later.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    drop(snapshot);
    drop(writer);
    drop(store);
    let replayed = replay_frozen_public_identity_v2(frozen.bytes(), frozen.digest())?;
    accepted.require_same_projection(&replayed)?;
    check!(eq; replayed.qualification_version(), PublicIdentityQualificationVersion::RetainedAuthorityGraphV3);
    check!(eq; replayed.resolve_candidate(&first)?, accepted.resolve_candidate(&first)?);
    check!(eq; replayed.resolve_candidate(&later.id.cast())?, PublicResolution::Unknown);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    check!(eq; writer.head(&store)?.sequence, 3);
    check!(eq; frozen.replay()?.head().sequence, 2);
    Ok(())
}

#[test]
fn frozen_graph_preserves_every_original_physical_key_and_value() -> TestResult {
    let (_directory, store, _writer, _) = two_births()?;
    let frozen = store.snapshot().freeze_public_identity_v2()?;
    let bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    check!(!bundle.objects.is_empty());
    bundle.objects.iter().try_for_each(|object| {
        let mut original = std::collections::BTreeMap::new();
        crate::public_identity::paged_generation::graph_physical_rows(
            &store.db.snapshot(),
            &store.journal,
            &object.reference,
            |key, value| {
                original.insert(key.to_vec(), value.to_vec());
                Ok(())
            },
        )?;
        let captured = object
            .rows
            .iter()
            .map(wire::PhysicalRow::decode)
            .collect::<crate::StoreResult<std::collections::BTreeMap<_, _>>>()?;
        check!(eq; captured, original);
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    })?;
    check!(crate::replay_frozen_public_identity_v1(frozen.bytes(), frozen.digest()).is_err());
    Ok(())
}

#[test]
fn empty_graph_ledger_has_exact_empty_frozen_inventory() -> TestResult {
    let (_directory, store, _writer) = fixture()?;
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("snapshot")?;
    let before = retained_state(&store)?;
    let frozen = snapshot.freeze_public_identity_v2()?;
    let bundle: wire::Bundle = serde_json::from_slice(frozen.bytes())?;
    check!(bundle.operations.is_empty());
    check!(bundle.objects.is_empty());
    accepted.require_same_projection(&frozen.replay()?)?;
    check!(eq; retained_state(&store)?, before);
    check!(snapshot.freeze_public_identity_v1().is_err());
    Ok(())
}
