use super::*;

#[test]
fn missing_or_changed_prepared_backing_never_allocates_another_attempt() -> TestResult {
    for damage in [
        "absent",
        "missing-body",
        "missing-metadata",
        "changed-body",
        "changed-metadata",
    ] {
        let (_dir, store, writer, owner) = fixture()?;
        let (prepared, body) = prepared(&store, &writer, &owner, 200)?;
        let attempt = prepared.attempt_id().clone();
        if damage != "absent" {
            let (body_path, meta_path) = publish(&store, &prepared, &body)?;
            match damage {
                "missing-body" => std::fs::remove_file(body_path)?,
                "missing-metadata" => std::fs::remove_file(meta_path)?,
                "changed-body" => std::fs::write(body_path, b"changed bytes")?,
                "changed-metadata" => std::fs::write(meta_path, b"changed metadata")?,
                _ => return Err("fixture damage".into()),
            }
        }
        drop(prepared);
        let before = rows(&store)?;
        check!(store
            .finish_prepared_acquisition(
                registered(&store, &writer, &owner)?,
                &mut AcquisitionReadBudget::for_work()
            )
            .is_err());
        check!(matches!(
            store.recover_acquisition_attempt(
                registered(&store, &writer, &owner)?,
                InterruptionCauseV1::OwnerRestarted
            ),
            Err(StoreError::Capture(CaptureError::PreparedRecoveryRequired))
        ));
        let owned = registered(&store, &writer, &owner)?;
        check!(eq; store.inspect_acquisition_work(&owned, &mut AcquisitionReadBudget::for_work())?, AcquireDecision::FinishPrepared(attempt));
        check!(matches!(
            store.reserve_acquisition_attempt(owned),
            Err(StoreError::Capture(CaptureError::PreparedRecoveryRequired))
        ));
        check!(eq; rows(&store)?, before);
    }
    Ok(())
}

#[test]
fn prepared_archive_reopens_and_finishes_original_attempt_without_dispatch() -> TestResult {
    let (dir, store, writer, owner) = fixture()?;
    let (prepared, body) = prepared(&store, &writer, &owner, 200)?;
    let attempt = prepared.attempt_id().clone();
    publish(&store, &prepared, &body)?;
    drop(prepared);
    drop(store);
    let (reopened, writer) =
        Store::open_capture_qualification_v1(dir.path().join("store"))?.into_parts();
    let owner = support::owner(&reopened, &writer, 2026, CaptureCacheIntent::ReuseEligible)?;
    let reference = reopened.finish_prepared_acquisition(
        registered(&reopened, &writer, &owner)?,
        &mut AcquisitionReadBudget::for_work(),
    )?;
    check!(eq; reference.attempt(), &attempt);
    let verified = reopened
        .read_retained_acquisition_v1(&reference, &mut AcquisitionReadBudget::for_work())?;
    check!(eq; std::fs::read(verified.body_path())?, body);
    check!(eq; verified.response().observed_at(), "2026-10-14T05:00:00Z");
    let owned = registered(&reopened, &writer, &owner)?;
    check!(eq; reopened.inspect_acquisition_work(&owned, &mut AcquisitionReadBudget::for_work())?, AcquireDecision::Replay(reference));
    Ok(())
}

#[test]
fn cache_replay_and_304_keep_exact_original_body_and_time_without_reference_chains() -> TestResult {
    let (_dir, store, writer, first) = fixture()?;
    let original = retained(&store, &writer, &first, 200)?;
    let replay_owner = support::owner(&store, &writer, 2025, CaptureCacheIntent::ReuseEligible)?;
    let verified = store.reuse_acquisition_cache(
        registered(&store, &writer, &replay_owner)?,
        &original,
        &mut AcquisitionReadBudget::for_work(),
    )?;
    check!(eq; verified.reference(), &original);
    let replay_work = work(&replay_owner)?.id()?;
    check!(!store.journal_contains(
        SOURCE_ACQUISITION_PHASE,
        &format!("work/{replay_work}/attempt/1/reserved")
    )?);
    let owner = support::owner(&store, &writer, 2024, CaptureCacheIntent::AcquireNew)?;
    let reservation = store.reserve_acquisition_attempt(registered(&store, &writer, &owner)?)?;
    let request = DispatchRequestV1::new(
        URL,
        ConditionalRequestV1::Pinned {
            original: original.clone(),
            etag: Some("fixture-etag".to_string()),
            last_modified: None,
        },
    )?;
    let dispatch = store
        .arm_acquisition_dispatch(reservation, request)?
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
    let verified = store
        .read_retained_acquisition_v1(&revalidation, &mut AcquisitionReadBudget::for_work())?;
    check!(eq; verified.reference(), &revalidation);
    check!(eq; verified.body_reference(), &original);
    check!(eq; verified.response().observed_at(), "2026-10-14T05:00:00Z");
    check!(eq; verified.revalidation().map(ResponseObservationV1::observed_at), Some("2026-10-14T06:00:00Z"));
    let third = support::owner(&store, &writer, 2023, CaptureCacheIntent::ReuseEligible)?;
    check!(matches!(
        store.reuse_acquisition_cache(
            registered(&store, &writer, &third)?,
            &revalidation,
            &mut AcquisitionReadBudget::for_work()
        ),
        Err(StoreError::Capture(CaptureError::CacheOccurrenceMismatch))
    ));
    Ok(())
}

#[test]
fn stale_dispatch_token_cannot_rewrite_unknown_after_a_successor_was_reserved() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let late = dispatched(&store, &writer, &owner)?;
    store.recover_acquisition_attempt(
        registered(&store, &writer, &owner)?,
        InterruptionCauseV1::WorkerDrained,
    )?;
    let second = store.reserve_acquisition_attempt(registered(&store, &writer, &owner)?)?;
    check!(eq; second.attempt_id().ordinal().get(), 2);
    let before = rows(&store)?;
    check!(matches!(
        store.record_acquisition_response(
            late,
            ResponseObservationV1::new(200, URL, "2026-10-14", vec![])?
        ),
        Err(StoreError::Capture(CaptureError::TransitionRefused))
    ));
    check!(eq; rows(&store)?, before);
    Ok(())
}

#[test]
fn actual_response_checkpoint_refusal_keeps_status_and_cannot_be_generic_dispatch_failure(
) -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let dispatch = dispatched(&store, &writer, &owner)?;
    let terminal = store.refuse_acquisition_response_checkpoint(
        dispatch,
        429,
        URL,
        ResponseCheckpointRefusalV1::UnrepresentableResponse,
    )?;
    check!(
        matches!(terminal.terminal(), AcquisitionTerminal::FailedUncaptured { checkpoint: AcquisitionCheckpointV1::Armed { .. }, failure: AcquisitionFailureV1::ResponseCheckpointRefused { status, .. } } if status.get() == 429)
    );
    let owned = registered(&store, &writer, &owner)?;
    check!(matches!(
        store.inspect_acquisition_work(&owned, &mut AcquisitionReadBudget::for_work())?,
        AcquireDecision::Settled(_)
    ));
    let next = support::owner(&store, &writer, 2025, CaptureCacheIntent::ReuseEligible)?;
    let dispatch = dispatched(&store, &writer, &next)?;
    check!(matches!(
        store.fail_acquisition_dispatch(dispatch, AcquisitionFailureV1::UnrepresentableResponse),
        Err(StoreError::Capture(CaptureError::TransitionRefused))
    ));
    Ok(())
}
