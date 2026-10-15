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
    let sequence = fixture.store.snapshot().sequence();
    assert!(fixture
        .writer
        .prepare_local_adjudication(
            &fixture.store,
            "admission-conflict",
            request,
            fixture.store.snapshot().revision(),
            &head
        )
        .is_err());
    assert_eq!(sequence, fixture.store.snapshot().sequence());
    Ok(())
}

#[test]
fn absent_enrollment_refuses_preparation_without_context_write() -> TestResult {
    let fixture = super::super::reviewed_fixture::ready_for_merge()?;
    results::install(&fixture)?;
    fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "unenrolled-merge",
        &fixture.request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let request = request(&fixture)?;
    let sequence = fixture.store.snapshot().sequence();
    assert!(fixture
        .writer
        .prepare_local_adjudication(
            &fixture.store,
            "unenrolled-manual",
            request,
            fixture.store.snapshot().revision(),
            &fixture.writer.head(&fixture.store)?
        )
        .is_err());
    assert_eq!(sequence, fixture.store.snapshot().sequence());
    assert!(fixture
        .writer
        .enroll_manual_context_admission_v1(&fixture.store)
        .is_err());
    assert_eq!(sequence, fixture.store.snapshot().sequence());
    Ok(())
}

#[test]
fn abandoned_reservation_reopens_and_resumes_original_prepared_bytes() -> TestResult {
    let fixture = merged()?;
    let request = request(&fixture)?;
    let head = fixture.writer.head(&fixture.store)?;
    let operation = "admission-crash-before-context";
    let before = fixture.store.snapshot().sequence();
    let reserved = fixture.writer.reserve_local_adjudication(
        &fixture.store,
        operation,
        request.clone(),
        fixture.store.snapshot().revision(),
        &head,
    )?;
    let original = serde_json::to_vec(reserved.prepared.record())?;
    assert!(fixture.store.snapshot().sequence() > before);
    drop(reserved);
    let path = fixture.directory.path().join("identity");
    drop(fixture.writer);
    drop(fixture.store);
    let (store, writer) = crate::Store::open_public_identity_qualification_v2(&path)?.into_parts();
    let prepared = writer.prepare_local_adjudication(
        &store,
        operation,
        request,
        store.snapshot().revision(),
        &head,
    )?;
    assert_eq!(original, serde_json::to_vec(prepared.record())?);
    Ok(())
}

#[test]
fn interruption_after_first_root_resumes_same_reservation() -> TestResult {
    let fixture = merged()?;
    let request = request(&fixture)?;
    let head = fixture.writer.head(&fixture.store)?;
    let operation = "admission-crash-between-roots";
    let reserved = fixture.writer.reserve_local_adjudication(
        &fixture.store,
        operation,
        request.clone(),
        fixture.store.snapshot().revision(),
        &head,
    )?;
    let original = serde_json::to_vec(reserved.prepared.record())?;
    reserved.inputs.stage_admitted(
        &fixture.store,
        &fixture.store.store_identity()?,
        &reserved.permits[0],
    )?;
    drop(reserved);
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        operation,
        request,
        fixture.store.snapshot().revision(),
        &head,
    )?;
    assert_eq!(original, serde_json::to_vec(prepared.record())?);
    Ok(())
}

#[test]
fn reserved_capability_refuses_foreign_store_and_changed_context_before_writes() -> TestResult {
    let fixture = merged()?;
    let request = request(&fixture)?;
    let mut reserved = fixture.writer.reserve_local_adjudication(
        &fixture.store,
        "admission-permit",
        request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let (_other_dir, other_store, _other_writer) =
        crate::public_identity_tests::support::fixture()?;
    let other_sequence = other_store.snapshot().sequence();
    assert!(reserved
        .inputs
        .stage_admitted(
            &other_store,
            &fixture.store.store_identity()?,
            &reserved.permits[0]
        )
        .is_err());
    assert_eq!(other_sequence, other_store.snapshot().sequence());
    reserved
        .inputs
        .athletes
        .push(crate::public_identity_tests::support::subject(
            "changed-reserved-input",
            "9977",
        )?);
    let sequence = fixture.store.snapshot().sequence();
    assert!(reserved
        .inputs
        .stage_admitted(
            &fixture.store,
            &fixture.store.store_identity()?,
            &reserved.permits[0]
        )
        .is_err());
    assert_eq!(sequence, fixture.store.snapshot().sequence());
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
