use super::*;
use crate::public_identity_tests::support::retained_state;

#[test]
fn exact_prepare_retry_preserves_record_and_does_not_write() -> TestResult {
    let fixture = merged()?;
    let request = request(&fixture)?;
    let head = fixture.writer.head(&fixture.store)?;
    let first = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "admission-exact",
        request.clone(),
        fixture.store.snapshot().revision(),
        &head,
    )?;
    let bytes = serde_json::to_vec(first.record())?;
    let before = retained_state(&fixture.store)?;
    let sequence = fixture.store.snapshot().sequence();
    let second = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "admission-exact",
        request,
        fixture.store.snapshot().revision(),
        &head,
    )?;
    assert_eq!(bytes, serde_json::to_vec(second.record())?);
    assert_eq!(before, retained_state(&fixture.store)?);
    assert_eq!(sequence, fixture.store.snapshot().sequence());
    Ok(())
}

#[test]
fn changed_request_cannot_reuse_reservation_or_write() -> TestResult {
    let fixture = merged()?;
    let mut request = request(&fixture)?;
    let head = fixture.writer.head(&fixture.store)?;
    fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "admission-conflict",
        request.clone(),
        fixture.store.snapshot().revision(),
        &head,
    )?;
    request.rationale.push_str(" changed");
    let before = retained_state(&fixture.store)?;
    let refused = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "admission-conflict",
        request,
        fixture.store.snapshot().revision(),
        &head,
    );
    check!(matches!(
        refused,
        Err(StoreError::PublicIdentity(
            PublicIdentityError::OperationConflict
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn absent_enrollment_refuses_preparation_without_graph_write() -> TestResult {
    let enrolled = merged()?;
    let request = request(&enrolled)?;
    let (_directory, store, writer) = crate::public_identity_tests::support::unenrolled_fixture()?;
    let before = retained_state(&store)?;
    let result = writer.prepare_local_adjudication(
        &store,
        "unenrolled-manual",
        request,
        store.snapshot().revision(),
        &writer.head(&store)?,
    );
    check!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "manual admission missing, corrupt, or exhausted"
        )))
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn abandoned_preparation_reopens_and_resumes_original_record_graph_and_approval() -> TestResult {
    let fixture = merged()?;
    let request = request(&fixture)?;
    let head = fixture.writer.head(&fixture.store)?;
    let operation = "admission-crash-before-approval";
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        operation,
        request.clone(),
        fixture.store.snapshot().revision(),
        &head,
    )?;
    let original = (
        serde_json::to_vec(prepared.record())?,
        serde_json::to_vec(prepared.graph())?,
        prepared.approval_bytes()?,
        prepared.approval_digest()?,
    );
    let committed = committed_state(&fixture.store)?;
    let state = retained_state(&fixture.store)?;
    drop(prepared);
    let path = fixture.directory.path().join("identity");
    drop(fixture.writer);
    drop(fixture.store);
    let (store, writer) = crate::Store::open_public_identity_qualification_v3(&path)?.into_parts();
    let prepared = writer.prepare_local_adjudication(
        &store,
        operation,
        request,
        store.snapshot().revision(),
        &head,
    )?;
    check!(eq; (serde_json::to_vec(prepared.record())?, serde_json::to_vec(prepared.graph())?, prepared.approval_bytes()?, prepared.approval_digest()?), original);
    check!(eq; committed_state(&store)?, committed);
    check!(eq; retained_state(&store)?, state);
    Ok(())
}

#[test]
fn interruption_after_complete_graph_before_preparation_reuses_the_same_graph() -> TestResult {
    let fixture = merged()?;
    let request = request(&fixture)?;
    let head = fixture.writer.head(&fixture.store)?;
    let operation = "admission-crash-after-graph";
    let inventory = stage_inventory(&fixture, operation)?;
    let original = inventory.graph().clone();
    let committed = committed_state(&fixture.store)?;
    let path = fixture.directory.path().join("identity");
    drop(fixture.writer);
    drop(fixture.store);
    let (store, writer) = crate::Store::open_public_identity_qualification_v3(&path)?.into_parts();
    let prepared = writer.prepare_local_adjudication(
        &store,
        operation,
        request.clone(),
        store.snapshot().revision(),
        &head,
    )?;
    check!(eq; prepared.graph(), &original);
    check!(eq; committed_state(&store)?, committed);
    let bytes = prepared.approval_bytes()?;
    let before = retained_state(&store)?;
    let retry = writer.prepare_local_adjudication(
        &store,
        operation,
        request,
        store.snapshot().revision(),
        &head,
    )?;
    check!(eq; retry.approval_bytes()?, bytes);
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn prepared_graph_refuses_foreign_store_before_any_write() -> TestResult {
    let fixture = merged()?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "admission-foreign",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let approval = approval(&prepared)?;
    let (_directory, other, _) = crate::public_identity_tests::support::fixture()?;
    let before = retained_state(&other)?;
    let original = retained_state(&fixture.store)?;
    let result = fixture
        .writer
        .apply_local_adjudication(&other, prepared, approval.0, approval.1);
    check!(
        matches!(result, Err(StoreError::RevisionOriginMismatch { operation }) if operation == "admission-foreign")
    );
    check!(eq; retained_state(&other)?, before);
    check!(eq; retained_state(&fixture.store)?, original);
    Ok(())
}

#[test]
fn concurrent_exact_preparations_share_original_record() -> TestResult {
    let fixture = merged()?;
    let request = request(&fixture)?;
    let head = fixture.writer.head(&fixture.store)?;
    let run = || -> Result<Vec<u8>, String> {
        let prepared = fixture
            .writer
            .prepare_local_adjudication(
                &fixture.store,
                "admission-concurrent",
                request.clone(),
                fixture.store.snapshot().revision(),
                &head,
            )
            .map_err(|error| error.to_string())?;
        serde_json::to_vec(prepared.record()).map_err(|error| error.to_string())
    };
    let (first, second) = std::thread::scope(|scope| {
        let first = scope.spawn(&run);
        let second = scope.spawn(&run);
        (first.join(), second.join())
    });
    let first = first.map_err(|_| "first preparation thread failed")??;
    let second = second.map_err(|_| "second preparation thread failed")??;
    assert_eq!(first, second);
    let sequence = fixture.store.snapshot().sequence();
    assert_eq!(first, run()?);
    assert_eq!(sequence, fixture.store.snapshot().sequence());
    Ok(())
}

mod accounting;

mod variants;
