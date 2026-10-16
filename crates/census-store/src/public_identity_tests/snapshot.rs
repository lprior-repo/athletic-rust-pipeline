use crate::{AcceptedPublicIdentitySnapshot, IdentityCommitOutcomeV1, PublicIdentityError};
use census_domain::model::AthleteCandidateId;
use census_domain::public_identity_history::PublicResolution;

use super::support::refused;
use super::*;

fn canonical(
    snapshot: &AcceptedPublicIdentitySnapshot,
    candidate: &AthleteCandidateId,
    committed: &IdentityCommitOutcomeV1,
) -> TestResult {
    let PublicResolution::Canonical(resolved) = snapshot.resolve_candidate(candidate)? else {
        return Err("expected accepted canonical public identity".into());
    };
    check!(eq; &resolved.public_id, committed.public_id());
    check!(eq; &resolved.representative_candidate, candidate);
    Ok(())
}

#[test]
fn public_identity_snapshot_keeps_its_head_and_resolution_after_later_establishment() -> TestResult
{
    let (_directory, store, writer) = fixture()?;
    let first = subject("snapshot-first", "601")?;
    let second = subject("snapshot-second", "602")?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let first_commit = writer.establish_source_bound(
        &store,
        "snapshot-first",
        &first.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let held = store.snapshot();
    let second_commit = writer.establish_source_bound(
        &store,
        "snapshot-second",
        &second.id.cast(),
        store.snapshot().revision(),
        first_commit.head(),
    )?;
    let before = retained_state(&store)?;
    let accepted = held
        .accepted_public_identity_snapshot()?
        .ok_or("qualification snapshot lacks accepted history")?;
    check!(eq; accepted.store_identity(), writer.store_identity());
    check!(eq; accepted.head(), first_commit.head());
    check!(eq; accepted.public_ids().collect::<Vec<_>>(), vec![first_commit.public_id()]);
    canonical(&accepted, &first.id.cast(), &first_commit)?;
    check!(eq; accepted.resolve_candidate(&second.id.cast())?, PublicResolution::Unknown);
    let current = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("current qualification snapshot lacks accepted history")?;
    check!(eq; current.store_identity(), accepted.store_identity());
    check!(eq; current.head(), second_commit.head());
    check!(ne; current.head(), accepted.head());
    canonical(&current, &first.id.cast(), &first_commit)?;
    canonical(&current, &second.id.cast(), &second_commit)?;
    let mut expected = vec![first_commit.public_id(), second_commit.public_id()];
    expected.sort();
    check!(eq; current.public_ids().collect::<Vec<_>>(), expected);
    check!(eq; accepted.head(), first_commit.head());
    check!(eq; accepted.public_ids().collect::<Vec<_>>(), vec![first_commit.public_id()]);
    check!(eq; accepted.resolve_candidate(&second.id.cast())?, PublicResolution::Unknown);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_legacy_snapshot_has_no_accepted_ledger_and_allows_legacy_publication(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open_with(directory.path(), 8_388_608)?;
    store.append(Table::Athletes, &subject("legacy-snapshot", "603")?)?;
    check!(eq; store.store_format()?.lifecycle_revision, Some(2));
    let before = retained_state(&store)?;
    let held = store.snapshot();
    check!(held.accepted_public_identity_snapshot()?.is_none());
    held.require_legacy_identity_publication()?;
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_qualification_snapshot_refuses_legacy_publication_before_and_after_commit(
) -> TestResult {
    let (_directory, store, writer) = fixture()?;
    check!(eq; store.store_format()?.lifecycle_revision, Some(7));
    let empty = store.snapshot();
    let before = retained_state(&store)?;
    check!(matches!(
        refused(empty.require_legacy_identity_publication())?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(eq; retained_state(&store)?, before);
    let athlete = subject("qualification-publication", "604")?;
    store.append(Table::Athletes, &athlete)?;
    writer.establish_source_bound(
        &store,
        "qualification-publication",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let before = retained_state(&store)?;
    check!(matches!(
        refused(store.snapshot().require_legacy_identity_publication())?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(matches!(
        refused(empty.require_legacy_identity_publication())?,
        StoreError::PublicIdentity(PublicIdentityError::OptInRequired)
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_snapshot_refuses_a_missing_witness_without_changing_the_pinned_view(
) -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("snapshot-missing-witness", "605")?;
    store.append(Table::Athletes, &athlete)?;
    let committed = writer.establish_source_bound(
        &store,
        "snapshot-missing-witness",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let held = store.snapshot();
    let key = format!("public_identity_v1:{:020}", committed.head().sequence);
    check!(store.journal.contains_key(&key)?);
    store.journal.remove(&key)?;
    store.flush()?;
    let before = retained_state(&store)?;
    check!(matches!(
        refused(store.snapshot().accepted_public_identity_snapshot())?,
        StoreError::PublicIdentity(PublicIdentityError::Corrupt("missing graph commit witness"))
    ));
    let accepted = held
        .accepted_public_identity_snapshot()?
        .ok_or("pinned qualification snapshot lost accepted history")?;
    check!(eq; accepted.head(), committed.head());
    canonical(&accepted, &athlete.id.cast(), &committed)?;
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_snapshot_refuses_a_forged_witness_without_yielding_an_accepted_projection(
) -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("snapshot-forged-witness", "606")?;
    store.append(Table::Athletes, &athlete)?;
    let committed = writer.establish_source_bound(
        &store,
        "snapshot-forged-witness",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let held = store.snapshot();
    let key = format!("public_identity_v1:{:020}", committed.head().sequence);
    let raw = store.journal.get(&key)?.ok_or("missing fixture witness")?;
    let mut witness: serde_json::Value = serde_json::from_slice(&raw)?;
    let candidate = witness
        .pointer_mut("/payload/candidate")
        .ok_or("missing witness candidate")?;
    check!(eq; candidate.as_str(), Some(athlete.id.as_str()));
    *candidate = serde_json::Value::String(subject("forged-candidate", "607")?.id.to_string());
    store.journal.insert(&key, serde_json::to_vec(&witness)?)?;
    store.flush()?;
    let before = retained_state(&store)?;
    check!(matches!(
        refused(store.snapshot().accepted_public_identity_snapshot())?,
        StoreError::PublicIdentity(PublicIdentityError::Corrupt("graph receipt binding"))
    ));
    let accepted = held
        .accepted_public_identity_snapshot()?
        .ok_or("pinned qualification snapshot lost accepted history")?;
    check!(eq; accepted.head(), committed.head());
    canonical(&accepted, &athlete.id.cast(), &committed)?;
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
