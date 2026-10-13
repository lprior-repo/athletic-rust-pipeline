use super::*;

#[test]
fn snapshot_revision_accepts_a_clone_of_its_originating_store() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let snapshot = store.snapshot();
    let revision = snapshot.revision();
    check!(eq; pair(revision), (snapshot.evidence_generation(), snapshot.derived_generation()));
    drop(snapshot);
    let cloned = store.clone();
    let (application, committed) = effects(&cloned, "clone-origin", "Clone High", 1)?
        .commit_once_at_revision("clone-origin", "clone-digest", revision)?
        .into_parts();
    check!(application.written());
    check!(eq; application.appended(), 1);
    check!(eq; pair(committed), pair(store.snapshot().revision()));
    check!(eq; pair(committed), pair(cloned.snapshot().revision()));
    let retained = RetainedState::capture(&store)?;
    drop(cloned);
    store.flush()?;
    drop(store);
    let reopened = Store::open(directory.path())?;
    retained.assert_unchanged(&reopened)?;
    check!(eq; reopened.receipt("clone-origin")?, Some(application.receipt().clone()));
    Ok(())
}

#[test]
fn matching_numeric_revisions_from_a_foreign_store_are_refused_without_writes() -> TestResult {
    let source_directory = tempfile::tempdir()?;
    let destination_directory = tempfile::tempdir()?;
    let source = Store::open(source_directory.path())?;
    let destination = Store::open(destination_directory.path())?;
    let foreign = source.snapshot().revision();
    check!(eq; pair(foreign), pair(destination.snapshot().revision()));
    let source_before = RetainedState::capture(&source)?;
    let destination_before = RetainedState::capture(&destination)?;
    let error = refused(
        effects(&destination, "foreign-origin", "Foreign High", 1)?.commit_once_at_revision(
            "foreign-origin",
            "foreign-digest",
            foreign,
        ),
    )?;
    match error {
        StoreError::RevisionOriginMismatch { operation } => {
            check!(eq; operation, "foreign-origin");
        }
        other => return Err(format!("expected origin mismatch: {other}").into()),
    }
    source_before.assert_unchanged(&source)?;
    destination_before.assert_unchanged(&destination)?;
    assert_absent(&destination, "foreign-origin", "Foreign High")?;
    destination.flush()?;
    drop(destination);
    let reopened = Store::open(destination_directory.path())?;
    destination_before.assert_unchanged(&reopened)?;
    assert_absent(&reopened, "foreign-origin", "Foreign High")
}

#[test]
fn invalid_operation_and_digest_are_refused_before_foreign_origin_checks() -> TestResult {
    let source_directory = tempfile::tempdir()?;
    let destination_directory = tempfile::tempdir()?;
    let source = Store::open(source_directory.path())?;
    let destination = Store::open(destination_directory.path())?;
    let before = RetainedState::capture(&destination)?;
    let revision = source.snapshot().revision();
    for (operation, digest) in [("", "digest"), ("invalid-digest", "")] {
        let error = refused(
            effects(&destination, "invalid-marker", "Invalid High", 1)?
                .commit_once_at_revision(operation, digest, revision),
        )?;
        check!(matches!(error, StoreError::Refused { .. }));
        before.assert_unchanged(&destination)?;
        assert_absent(&destination, "invalid-marker", "Invalid High")?;
    }
    Ok(())
}

#[test]
fn a_foreign_origin_cannot_borrow_a_matching_destination_receipt() -> TestResult {
    let source_directory = tempfile::tempdir()?;
    let destination_directory = tempfile::tempdir()?;
    let source = Store::open(source_directory.path())?;
    let destination = Store::open(destination_directory.path())?;
    let (_, foreign) = effects(&source, "source-first", "Source High", 1)?
        .commit_once_at_revision(
            "source-first",
            "source-digest",
            source.snapshot().revision(),
        )?
        .into_parts();
    let (first, accepted) = effects(&destination, "origin-repeat", "Destination High", 1)?
        .commit_once_at_revision(
            "origin-repeat",
            "destination-digest",
            destination.snapshot().revision(),
        )?
        .into_parts();
    check!(eq; pair(foreign), pair(accepted));
    let before = RetainedState::capture(&destination)?;
    let error =
        refused(
            effects(&destination, "origin-repeat", "Never Foreign High", 9)?
                .commit_once_at_revision("origin-repeat", "destination-digest", foreign),
        )?;
    match error {
        StoreError::RevisionOriginMismatch { operation } => {
            check!(eq; operation, "origin-repeat");
        }
        other => return Err(format!("matching receipt bypassed the origin fence: {other}").into()),
    }
    before.assert_unchanged(&destination)?;
    check!(eq; destination.receipt("origin-repeat")?, Some(first.receipt().clone()));
    destination.flush()?;
    drop(destination);
    let reopened = Store::open(destination_directory.path())?;
    before.assert_unchanged(&reopened)
}
