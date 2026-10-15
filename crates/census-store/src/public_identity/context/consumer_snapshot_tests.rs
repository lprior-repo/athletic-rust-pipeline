use super::test_fixture::{full_inputs, install_inputs, witness_root};
use crate::journal::object_manifest_key;
use crate::public_identity_tests::support::{fixture, retained_state, TestResult};
use crate::{Store, StoreError, Table};
use census_domain::model::AppliedAthleteIdentity;
use census_domain::public_identity_history::{
    EstablishmentOrdinal, PublicResolution, ResolvedSuccessor,
};

#[test]
fn owned_public_identity_consumer_keeps_original_resolution_through_later_inputs_and_reopen(
) -> TestResult {
    let (directory, store, writer) = fixture()?;
    let inputs = full_inputs()?;
    install_inputs(&store, &inputs)?;
    let first = inputs.athletes.first().ok_or("first candidate")?;
    let second = inputs.athletes.get(1).ok_or("second candidate")?;
    let first_commit = writer.establish_source_bound(
        &store,
        "consumer-first",
        &first.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let original = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("original accepted consumer")?;
    let expected = PublicResolution::Canonical(ResolvedSuccessor {
        public_id: first_commit.public_id().clone(),
        representative_candidate: first.id.cast(),
        established: EstablishmentOrdinal {
            sequence: 1,
            position: 0,
        },
    });
    check!(ne; first_commit.public_id().as_str(), first.id.as_str());
    check!(eq; original.resolve_candidate(&first.id.cast())?, expected);
    check!(eq; original.resolve_candidate(&second.id.cast())?, PublicResolution::Unknown);
    let mut later = first.clone();
    later
        .evidence
        .first_mut()
        .ok_or("first evidence")?
        .observed_on = "2026-10-15".into();
    store.append(Table::Athletes, &later)?;
    let second_commit = writer.establish_source_bound(
        &store,
        "consumer-second",
        &second.id.cast(),
        store.snapshot().revision(),
        first_commit.head(),
    )?;
    let current = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("current accepted consumer")?;
    check!(eq; current.resolve_candidate(&first.id.cast())?, expected);
    check!(eq; current.head(), second_commit.head());
    check!(eq; original.head(), first_commit.head());
    check!(eq; original.resolve_candidate(&second.id.cast())?, PublicResolution::Unknown);
    let before = retained_state(&store)?;
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v2(directory.path().join("identity"))?
            .into_parts();
    let reopened = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("reopened accepted consumer")?;
    check!(eq; reopened.store_identity(), original.store_identity());
    check!(eq; reopened.head(), current.head());
    check!(eq; reopened.resolve_candidate(&first.id.cast())?, expected);
    check!(eq; original.resolve_candidate(&first.id.cast())?, expected);
    check!(eq; reopened.resolve_candidate(&second.id.cast())?, current.resolve_candidate(&second.id.cast())?);
    check!(eq; writer.head(&store)?, *second_commit.head());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn unestablished_candidates_never_become_public_identity_fallbacks_after_reopen() -> TestResult {
    let (directory, store, writer) = fixture()?;
    let inputs = full_inputs()?;
    install_inputs(&store, &inputs)?;
    let original = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("empty accepted ledger")?;
    for athlete in &inputs.athletes {
        check!(eq; original.resolve_candidate(&athlete.id.cast())?, PublicResolution::Unknown);
    }
    check!(eq; original.head().sequence, 0);
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?.len(), 0);
    let before = retained_state(&store)?;
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v2(directory.path().join("identity"))?
            .into_parts();
    let reopened = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("reopened empty accepted ledger")?;
    check!(eq; reopened.head(), original.head());
    check!(eq; writer.head(&store)?, *original.head());
    for athlete in &inputs.athletes {
        check!(eq; reopened.resolve_candidate(&athlete.id.cast())?, PublicResolution::Unknown);
    }
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn consumer_snapshot_survives_context_corruption_while_current_and_reopened_reads_refuse(
) -> TestResult {
    let (directory, store, writer) = fixture()?;
    let inputs = full_inputs()?;
    install_inputs(&store, &inputs)?;
    let candidate = inputs.athletes.first().ok_or("first candidate")?.id.cast();
    let committed = writer.establish_source_bound(
        &store,
        "consumer-corruption",
        &candidate,
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let held = store.snapshot();
    let original = held
        .accepted_public_identity_snapshot()?
        .ok_or("original accepted consumer")?;
    let expected = original.resolve_candidate(&candidate)?;
    let PublicResolution::Canonical(resolved) = &expected else {
        return Err("established candidate is not canonical".into());
    };
    check!(eq; &resolved.public_id, committed.public_id());
    let root = witness_root(&store, 1)?;
    store.journal.remove(object_manifest_key(&root)?)?;
    store.flush()?;
    let before = retained_state(&store)?;
    match store.snapshot().accepted_public_identity_snapshot() {
        Err(StoreError::JournalManifestMissing { phase, key, object }) => {
            check!(eq; phase, "authority_context_v1");
            check!(eq; key, root);
            check!(eq; object, root);
        }
        Err(error) => return Err(format!("unexpected current consumer refusal: {error:?}").into()),
        Ok(_) => return Err("current consumer accepted missing retained context".into()),
    }
    let reread = held
        .accepted_public_identity_snapshot()?
        .ok_or("held accepted consumer")?;
    check!(eq; reread.resolve_candidate(&candidate)?, expected);
    check!(eq; original.resolve_candidate(&candidate)?, expected);
    check!(eq; retained_state(&store)?, before);
    drop(held);
    drop(writer);
    drop(store);
    match Store::open_public_identity_qualification_v2(directory.path().join("identity")) {
        Err(StoreError::JournalManifestMissing { phase, key, object }) => {
            check!(eq; phase, "authority_context_v1");
            check!(eq; key, root);
            check!(eq; object, root);
        }
        Err(error) => return Err(format!("unexpected reopened consumer refusal: {error:?}").into()),
        Ok(_) => return Err("reopened store accepted missing retained context".into()),
    }
    check!(eq; original.resolve_candidate(&candidate)?, expected);
    check!(eq; original.head(), committed.head());
    Ok(())
}
