use super::*;

#[test]
fn retained_capture_reader_cannot_import_a_later_occurrence_into_an_older_snapshot() -> TestResult {
    for protocol in [CaptureProtocolVersion::V1, CaptureProtocolVersion::V2] {
        let (_dir, store, writer, owner) = fixture_for(protocol)?;
        let before = store.snapshot();
        let occurrence = retained(&store, &writer, &owner, 200)?;
        check!(matches!(
            before
                .read_retained_acquisition_v1(&occurrence, &mut AcquisitionReadBudget::for_work()),
            Err(StoreError::Capture(CaptureError::SlotWorkMismatch))
        ));
        let after = store.snapshot();
        let verified = after
            .read_retained_acquisition_v1(&occurrence, &mut AcquisitionReadBudget::for_work())?;
        check!(eq; verified.reference(), &occurrence);
        check!(after.sequence() > before.sequence());
    }
    Ok(())
}

#[test]
fn retained_capture_reader_uses_the_snapshotted_qualification_marker() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let occurrence = retained(&store, &writer, &owner, 200)?;
    let before = store.snapshot();
    store.meta.remove(CAPTURE_QUALIFICATION_KEY)?;
    let verified =
        before.read_retained_acquisition_v1(&occurrence, &mut AcquisitionReadBudget::for_work())?;
    check!(eq; verified.reference(), &occurrence);
    check!(matches!(
        store.read_retained_acquisition_v1(&occurrence, &mut AcquisitionReadBudget::for_work()),
        Err(StoreError::Capture(CaptureError::CaptureCapabilityMissing))
    ));
    Ok(())
}

#[test]
fn snapshot_reader_rejects_foreign_occurrence_without_mutating_either_store() -> TestResult {
    let (_left, source, writer, owner) = fixture()?;
    let occurrence = retained(&source, &writer, &owner, 200)?;
    let (_right, destination, _, _) = fixture()?;
    let original = rows(&destination)?;
    let source_rows = rows(&source)?;
    check!(matches!(
        destination
            .snapshot()
            .read_retained_acquisition_v1(&occurrence, &mut AcquisitionReadBudget::for_work()),
        Err(StoreError::Capture(
            CaptureError::CaptureCapabilityForeignStore
        ))
    ));
    check!(eq; rows(&destination)?, original);
    check!(eq; rows(&source)?, source_rows);
    Ok(())
}

#[test]
fn retained_capture_reader_uses_snapshotted_terminal_receipts_in_both_protocols() -> TestResult {
    for protocol in [CaptureProtocolVersion::V1, CaptureProtocolVersion::V2] {
        let (_dir, store, writer, owner) = fixture_for(protocol)?;
        let occurrence = retained(&store, &writer, &owner, 200)?;
        let before = store.snapshot();
        let operation = read::terminal_operation(occurrence.attempt(), protocol);
        let receipt = store
            .receipts
            .get(&operation)?
            .ok_or("missing fixture receipt")?;
        store.receipts.remove(&operation)?;
        let missing = store.snapshot();
        let verified = before
            .read_retained_acquisition_v1(&occurrence, &mut AcquisitionReadBudget::for_work())?;
        check!(eq; verified.reference(), &occurrence);
        check!(matches!(
            missing
                .read_retained_acquisition_v1(&occurrence, &mut AcquisitionReadBudget::for_work(),),
            Err(StoreError::Capture(CaptureError::CorruptClosure))
        ));
        store.receipts.insert(&operation, receipt)?;
        check!(matches!(
            missing
                .read_retained_acquisition_v1(&occurrence, &mut AcquisitionReadBudget::for_work(),),
            Err(StoreError::Capture(CaptureError::CorruptClosure))
        ));
        check!(eq; store.snapshot().read_retained_acquisition_v1(
            &occurrence, &mut AcquisitionReadBudget::for_work(),
        )?.reference(), &occurrence);
    }
    Ok(())
}

#[test]
fn snapshot_revalidation_checks_both_original_and_revalidation_receipts() -> TestResult {
    let (_dir, store, writer, first) = fixture()?;
    let original = retained(&store, &writer, &first, 200)?;
    let current = support::owner(&store, &writer, 2025, CaptureCacheIntent::AcquireNew)?;
    let reservation = store.reserve_acquisition_attempt(registered(&store, &writer, &current)?)?;
    let dispatch = store
        .arm_acquisition_dispatch(
            reservation,
            DispatchRequestV1::new(
                URL,
                ConditionalRequestV1::Pinned {
                    original: original.clone(),
                    etag: Some("fixture-etag".into()),
                    last_modified: None,
                },
            )?,
        )?
        .into_observation();
    let response = store.record_acquisition_response(
        dispatch,
        ResponseObservationV1::new(304, URL, "2026-10-14T06:00:00Z", vec![])?,
    )?;
    let revalidation = store.finish_acquisition_not_modified(
        response,
        &original,
        &mut AcquisitionReadBudget::for_work(),
    )?;
    let snapshot = store.snapshot();
    for reference in [&original, &revalidation] {
        let operation = read::terminal_operation(reference.attempt(), CaptureProtocolVersion::V1);
        let bytes = store
            .receipts
            .get(&operation)?
            .ok_or("missing terminal receipt")?;
        store.receipts.remove(&operation)?;
        let verified = snapshot
            .read_retained_acquisition_v1(&revalidation, &mut AcquisitionReadBudget::for_work())?;
        check!(eq; verified.body_reference(), &original);
        check!(eq; verified.revalidation().map(ResponseObservationV1::status), Some(304));
        check!(matches!(
            store.snapshot().read_retained_acquisition_v1(
                &revalidation,
                &mut AcquisitionReadBudget::for_work(),
            ),
            Err(StoreError::Capture(CaptureError::CorruptClosure))
        ));
        store.receipts.insert(&operation, bytes)?;
    }
    Ok(())
}
