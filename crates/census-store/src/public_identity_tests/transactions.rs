use crate::PublicIdentityError;

use super::support::refused;
use super::*;

#[test]
fn public_identity_exact_replay_precedes_stale_revision_and_head_checks() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let first = subject("first", "101")?;
    let second = subject("second", "102")?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let original_revision = store.snapshot().revision();
    let original_head = writer.head(&store)?;
    let committed = writer.establish_source_bound(
        &store,
        "first",
        &first.id.cast(),
        original_revision,
        &original_head,
    )?;
    check!(!committed.replayed());
    check!(eq; committed.head().sequence, 1);
    check!(committed.public_id().as_str().starts_with("ath_public_v1_"));
    let next = writer.establish_source_bound(
        &store,
        "second",
        &second.id.cast(),
        store.snapshot().revision(),
        committed.head(),
    )?;
    check!(eq; next.head().sequence, 2);
    check!(ne; next.public_id(), committed.public_id());
    let before = retained_state(&store)?;
    let repeated = writer.establish_source_bound(
        &store,
        "first",
        &first.id.cast(),
        original_revision,
        &original_head,
    )?;
    check!(repeated.replayed());
    check!(eq; repeated.public_id(), committed.public_id());
    check!(eq; repeated.head(), committed.head());
    check!(eq; writer.head(&store)?, next.head().clone());
    check!(eq; store.count(Table::AthleteIdentityDecisions)?, 2);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_committed_id_and_receipt_survive_reopen() -> TestResult {
    let (directory, store, writer) = fixture()?;
    let athlete = subject("reopen", "103")?;
    store.append(Table::Athletes, &athlete)?;
    let initial_head = writer.head(&store)?;
    let committed = writer.establish_source_bound(
        &store,
        "reopen",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &initial_head,
    )?;
    let identity = store.store_identity()?;
    let before = retained_state(&store)?;
    drop(store);
    let root = directory.path().join("identity");
    let (store, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
    check!(eq; store.store_identity()?, identity);
    check!(eq; writer.head(&store)?, committed.head().clone());
    let repeated = writer.establish_source_bound(
        &store,
        "reopen",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &initial_head,
    )?;
    check!(repeated.replayed());
    check!(eq; repeated.public_id(), committed.public_id());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_changed_operation_payload_refuses_without_any_write() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let first = subject("payload-first", "104")?;
    let second = subject("payload-second", "105")?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let head = writer.head(&store)?;
    writer.establish_source_bound(
        &store,
        "immutable",
        &first.id.cast(),
        store.snapshot().revision(),
        &head,
    )?;
    let before = retained_state(&store)?;
    let error = refused(writer.establish_source_bound(
        &store,
        "immutable",
        &second.id.cast(),
        store.snapshot().revision(),
        &head,
    ))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::OperationConflict)
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_foreign_revision_refuses_even_for_an_exact_receipt() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let (_other_directory, other, _) = fixture()?;
    let athlete = subject("origin", "106")?;
    store.append(Table::Athletes, &athlete)?;
    let head = writer.head(&store)?;
    writer.establish_source_bound(
        &store,
        "origin",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &head,
    )?;
    let before = retained_state(&store)?;
    let error = refused(writer.establish_source_bound(
        &store,
        "origin",
        &athlete.id.cast(),
        other.snapshot().revision(),
        &head,
    ))?;
    check!(matches!(error, StoreError::RevisionOriginMismatch { .. }));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_new_operations_fence_both_store_revision_and_registry_head() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let first = subject("fence-first", "107")?;
    let second = subject("fence-second", "108")?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let revision = store.snapshot().revision();
    let head = writer.head(&store)?;
    let committed =
        writer.establish_source_bound(&store, "fence-first", &first.id.cast(), revision, &head)?;
    let before = retained_state(&store)?;
    let error = refused(writer.establish_source_bound(
        &store,
        "stale-revision",
        &second.id.cast(),
        revision,
        committed.head(),
    ))?;
    check!(matches!(error, StoreError::RevisionMoved { .. }));
    check!(eq; retained_state(&store)?, before);
    let error = refused(writer.establish_source_bound(
        &store,
        "stale-head",
        &second.id.cast(),
        store.snapshot().revision(),
        &head,
    ))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::HeadMoved)
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_writer_is_bound_to_one_live_store_origin() -> TestResult {
    let (directory, store, writer) = fixture()?;
    let (_other_directory, other, _) = fixture()?;
    let before = retained_state(&other)?;
    let foreign_candidate = subject("foreign-writer", "112")?;
    let error = refused(writer.establish_source_bound(
        &other,
        "foreign-writer",
        &foreign_candidate.id.cast(),
        other.snapshot().revision(),
        &writer.head(&store)?,
    ))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::ForeignStore)
    ));
    check!(eq; retained_state(&other)?, before);
    let error = refused(writer.head(&other))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::ForeignStore)
    ));
    check!(eq; retained_state(&other)?, before);
    let identity = store.store_identity()?;
    drop(store);
    let root = directory.path().join("identity");
    let (reopened, _) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
    check!(eq; reopened.store_identity()?, identity);
    let before = retained_state(&reopened)?;
    let error = refused(writer.head(&reopened))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::ForeignStore)
    ));
    check!(eq; retained_state(&reopened)?, before);
    Ok(())
}

#[test]
fn public_identity_missing_source_evidence_cannot_mint_or_write_a_receipt() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let mut athlete = subject("unparsed", "109")?;
    athlete.evidence.clear();
    store.append(Table::Athletes, &athlete)?;
    let before = retained_state(&store)?;
    let head = writer.head(&store)?;
    let error = refused(writer.establish_source_bound(
        &store,
        "unparsed",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &head,
    ))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::NotSourceBound)
    ));
    check!(eq; writer.head(&store)?, head);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_new_operation_cannot_remint_an_already_bound_candidate() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("already-bound", "111")?;
    store.append(Table::Athletes, &athlete)?;
    let committed = writer.establish_source_bound(
        &store,
        "original",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let before = retained_state(&store)?;
    let error = refused(writer.establish_source_bound(
        &store,
        "replacement",
        &athlete.id.cast(),
        store.snapshot().revision(),
        committed.head(),
    ))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::AlreadyBound)
    ));
    check!(eq; writer.head(&store)?, committed.head().clone());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
