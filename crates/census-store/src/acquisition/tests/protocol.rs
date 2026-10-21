use super::*;

#[test]
fn slot_registration_replays_exact_bytes_and_refuses_changed_request_budget_reset() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let original = registered(&store, &writer, &owner)?;
    let id = original.work_id().clone();
    drop(original);
    let before = rows(&store)?;
    let repeated = registered(&store, &writer, &owner)?;
    check!(eq; repeated.work_id(), &id);
    check!(eq; rows(&store)?, before.clone());
    drop(repeated);
    for url in [
        format!("{URL}&extra=1"),
        URL.replace("fields=fixture", "fields=changed"),
    ] {
        let changed = owner.for_owned_meet(
            UsJurisdiction::Michigan,
            "123",
            RequestRepresentationV1::get(&url, vec![], "Census/1.0 (public-source qualification)")?,
        )?;
        let grant =
            writer.continuation_for_owned_meet(&store, &owner, UsJurisdiction::Michigan, "123")?;
        check!(matches!(
            store.register_acquisition_work(&owner, changed, grant),
            Err(StoreError::Capture(CaptureError::ObligationConflict))
        ));
    }
    let changed_owner = support::owner(&store, &writer, 2026, CaptureCacheIntent::AcquireNew)?;
    check!(matches!(
        registered(&store, &writer, &changed_owner),
        Err(StoreError::Capture(CaptureError::ObligationConflict))
    ));
    check!(eq; rows(&store)?, before);
    Ok(())
}

#[test]
fn original_slot_lifetime_ceiling_survives_separate_owner_continuations() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    for ordinal in 1..=3 {
        let dispatch = dispatched(&store, &writer, &owner)?;
        check!(eq; dispatch.dispatch_id().attempt().ordinal().get(), ordinal);
        let terminal = store.fail_acquisition_dispatch(dispatch, AcquisitionFailureV1::Timeout)?;
        check!(matches!(
            terminal.terminal(),
            AcquisitionTerminal::OutcomeUnknown { .. }
        ));
    }
    let owned = registered(&store, &writer, &owner)?;
    check!(eq; store.inspect_acquisition_work(&owned, &mut AcquisitionReadBudget::for_work())?, AcquireDecision::Interrupted);
    check!(matches!(
        store.reserve_acquisition_attempt(owned),
        Err(StoreError::Capture(CaptureError::RetainedOutcomeUnknown))
    ));
    Ok(())
}

#[test]
fn reservation_reuses_ordinal_but_armed_state_never_reconstructs_dispatch() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let reserved = store.reserve_acquisition_attempt(registered(&store, &writer, &owner)?)?;
    let attempt = reserved.attempt_id().clone();
    drop(reserved);
    let resumed = store.reserve_acquisition_attempt(registered(&store, &writer, &owner)?)?;
    check!(eq; resumed.attempt_id(), &attempt);
    let armed = store.arm_acquisition_dispatch(
        resumed,
        DispatchRequestV1::new(URL, ConditionalRequestV1::Unconditional)?,
    )?;
    drop(armed);
    let owned = registered(&store, &writer, &owner)?;
    check!(eq; store.inspect_acquisition_work(&owned, &mut AcquisitionReadBudget::for_work())?, AcquireDecision::RecoveryRequired(attempt.clone()));
    check!(matches!(
        store.reserve_acquisition_attempt(owned),
        Err(StoreError::Capture(CaptureError::RecoveryRequired))
    ));
    let recovered = store.recover_acquisition_attempt(
        registered(&store, &writer, &owner)?,
        InterruptionCauseV1::OwnerRestarted,
    )?;
    check!(eq; recovered.attempt(), &attempt);
    Ok(())
}

#[test]
fn retained_retryable_responses_do_not_become_success_or_cache_originals() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let refused = retained(&store, &writer, &owner, 500)?;
    let owned = registered(&store, &writer, &owner)?;
    check!(matches!(
        store.inspect_acquisition_work(&owned, &mut AcquisitionReadBudget::for_work())?,
        AcquireDecision::RetryOwed(_)
    ));
    drop(owned);
    let second = support::owner(&store, &writer, 2025, CaptureCacheIntent::ReuseEligible)?;
    check!(matches!(
        store.reuse_acquisition_cache(
            registered(&store, &writer, &second)?,
            &refused,
            &mut AcquisitionReadBudget::for_work()
        ),
        Err(StoreError::Capture(CaptureError::CacheOccurrenceMismatch))
    ));
    let good = retained(&store, &writer, &owner, 200)?;
    check!(eq; good.attempt().ordinal().get(), 2);
    let owned = registered(&store, &writer, &owner)?;
    check!(eq; store.inspect_acquisition_work(&owned, &mut AcquisitionReadBudget::for_work())?, AcquireDecision::Replay(good));
    Ok(())
}

#[test]
fn generic_journal_receipt_and_credit_paths_cannot_overwrite_capture_roots() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let occurrence = retained(&store, &writer, &owner, 200)?;
    let key = format!("work/{}/result", occurrence.attempt().work_id());
    check!(store
        .journal_done(SOURCE_ACQUISITION_PHASE, &key, &"replacement")
        .is_err());
    let mut batch = store.write_batch();
    check!(batch
        .journal_done(SOURCE_ACQUISITION_PHASE, &key, &"replacement")
        .is_err());
    let mut stage = store.stage_derived()?;
    check!(stage
        .journal_done_once(SOURCE_ACQUISITION_PHASE, &key, &"replacement")
        .is_err());
    drop(stage);
    let operation = read::terminal_operation(occurrence.attempt(), CaptureProtocolVersion::V1);
    check!(store
        .write_batch()
        .commit_once(&operation, "replacement")
        .is_err());
    check!(store
        .stage_derived()?
        .publish(&operation, "replacement")
        .is_err());
    check!(store.credit_observations("caller", &operation).is_err());
    let committed = read::terminal_operation(occurrence.attempt(), CaptureProtocolVersion::V2);
    let before = store.receipt(&committed)?.ok_or("receipt")?;
    store.prune_receipts(&crate::ReplayFrontier {
        at: "2100-01-01".to_string(),
        witnesses: vec![],
    })?;
    check!(eq; store.receipt(&committed)?, Some(before));
    Ok(())
}

#[test]
fn full_three_attempt_five_dispatch_closure_fits_exactly_forty_two_roots() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let mut final_reference = None;
    for status in [500, 429, 200] {
        let mut dispatch = dispatched(&store, &writer, &owner)?;
        let mut redirects = Vec::new();
        let mut current = URL.to_string();
        for hop in 1..=4 {
            let target = format!("{URL}&hop={hop}");
            let response = store.record_acquisition_response(
                dispatch,
                ResponseObservationV1::new(
                    302,
                    &current,
                    "2026-10-14T05:00:00Z",
                    vec![("location".to_string(), target.clone())],
                )?,
            )?;
            redirects.push(serde_json::json!({"status":302,"url":target}));
            dispatch = store
                .arm_acquisition_redirect(
                    response,
                    DispatchRequestV1::new(&target, ConditionalRequestV1::Unconditional)?,
                )?
                .into_observation();
            current = target;
        }
        let observed = store.record_acquisition_response(
            dispatch,
            ResponseObservationV1::new(status, &current, "2026-10-14T05:00:00Z", vec![])?,
        )?;
        let body = b"shared body";
        let digest = format!("{:x}", Sha256::digest(body));
        let metadata = serde_json::to_vec_pretty(
            &serde_json::json!({"url":URL,"response_url":current,"method":"GET","status":status,"content_digest":digest,"bytes":body.len(),"fetched_at":"2026-10-14T05:00:00Z","redirects":redirects}),
        )?;
        let prepared = store.prepare_acquisition_capture(
            observed,
            PreparedCaptureInputV1::new(&digest, u64::try_from(body.len())?, metadata)?,
            CaptureDispositionV1::for_http_status(status)?,
        )?;
        publish(&store, &prepared, body)?;
        final_reference = Some(store.finish_acquisition_capture(prepared)?);
    }
    let reference = final_reference.ok_or("reference")?;
    let mut exact = AcquisitionReadBudget::new(42, 8 * 1024 * 1024, 64 * 1024 * 1024)?;
    check!(store
        .read_retained_acquisition_v1(&reference, &mut exact)
        .is_ok());
    let mut short = AcquisitionReadBudget::new(41, 8 * 1024 * 1024, 64 * 1024 * 1024)?;
    check!(matches!(
        store.read_retained_acquisition_v1(&reference, &mut short),
        Err(StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
            resource: "roots",
            ..
        }))
    ));
    Ok(())
}
