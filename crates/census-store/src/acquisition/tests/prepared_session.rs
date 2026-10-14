use super::*;
mod continuation;
mod freshness;
mod owner;
mod owner_binding;

fn selected(
    store: &Store,
    owner: &ResultsCaptureOwnerV1,
) -> TestResult<(PreparedRecoveryTarget, AcquisitionSelectionUsage)> {
    let mut selection = AcquisitionReadBudget::for_selection();
    let ExistingRecoveryRoute::Prepared(target) =
        store.read_existing_recovery_target(owner, &work(owner)?, &mut selection)?
    else {
        return Err("selected Prepared".into());
    };
    Ok((target, selection.selection_usage()))
}
fn unresolved<'store, 'budget>(
    read: PreparedRecoveryRead<'store, 'budget>,
) -> TestResult<PreparedRecoverySession<'store, 'budget>> {
    match read {
        PreparedRecoveryRead::Unresolved(session) => Ok(session),
        _ => Err("unresolved Prepared".into()),
    }
}

#[test]
fn prepared_session_exact_finish_moves_one_body_and_keeps_original_facts() -> TestResult {
    let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    let (prepared, body) = prepared(&store, &writer, &owner, 200)?;
    let original = store
        .journal
        .get(Store::journal_key(
            owner.protocol().phase(),
            &format!("{}/prepared", read::attempt_prefix(prepared.attempt_id())),
        ))?
        .ok_or("prepared")?
        .to_vec();
    publish(&store, &prepared, &body)?;
    let (target, usage) = selected(&store, &owner)?;
    let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
    store.validate_prepared_recovery_owner(&owner, &mut parent)?;
    let admission = parent.reads().recovery_totals().admission.used;
    let session = unresolved(
        store
            .begin_prepared_recovery(&owner, &target, &mut parent)?
            .read_backing()?,
    )?;
    check!(matches!(
        session.assessment()?,
        PreparedBackingAssessment::Exact
    ));
    let parts = session.finish()?.into_parts();
    check!(eq; parts.body, body);
    check!(eq; parts.occurrence.attempt(), target.attempt());
    check!(eq; parts.original_occurrence, parts.occurrence);
    check!(eq; parts.receipt.appended, 0);
    check!(!parts.receipt.credited);
    check!(eq; parent.reads().recovery_totals().admission.used, admission);
    check!(eq; store.journal.get(Store::journal_key(owner.protocol().phase(), &format!("{}/prepared", read::attempt_prefix(target.attempt()))))?.ok_or("prepared")?.to_vec(), original);
    Ok(())
}

#[test]
fn prepared_session_missing_close_replays_original_receipt_even_after_late_bytes() -> TestResult {
    let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    let (prepared, body) = prepared(&store, &writer, &owner, 200)?;
    let (target, usage) = selected(&store, &owner)?;
    let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
    let mut session = unresolved(
        store
            .begin_prepared_recovery(&owner, &target, &mut parent)?
            .read_backing()?,
    )?;
    let decision = session.missing_decision()?;
    let failed = session.close_missing(&decision)?;
    check!(matches!(
        failed.terminal().terminal(),
        AcquisitionTerminal::PreparedRetentionFailed { .. }
    ));
    let receipt = failed.receipt().clone();
    let terminal = failed.terminal().clone();
    publish(&store, &prepared, &body)?;
    let before = rows(&store)?;
    let mut replay = PreparedRecoveryBudget::admit(usage, 1)?;
    let repeated = store
        .begin_prepared_recovery(&owner, &target, &mut replay)?
        .close_missing(&decision)?;
    check!(eq; repeated.terminal(), &terminal);
    check!(eq; repeated.receipt(), &receipt);
    check!(eq; rows(&store)?, before);
    let result = Store::journal_key(
        owner.protocol().phase(),
        &format!("work/{}/result", target.attempt().work_id()),
    );
    check!(!store.journal.contains_key(result)?);
    Ok(())
}

#[test]
fn prepared_session_missing_close_refuses_new_exact_body_without_mutation() -> TestResult {
    let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    let (prepared, body) = prepared(&store, &writer, &owner, 200)?;
    let (target, usage) = selected(&store, &owner)?;
    let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
    let mut session = unresolved(
        store
            .begin_prepared_recovery(&owner, &target, &mut parent)?
            .read_backing()?,
    )?;
    let decision = session.missing_decision()?;
    publish(&store, &prepared, &body)?;
    let before = rows(&store)?;
    check!(matches!(
        session.close_missing(&decision),
        Err(StoreError::Capture(CaptureError::ExactRecoveryAvailable))
    ));
    check!(eq; rows(&store)?, before);
    Ok(())
}

#[test]
fn prepared_session_candidate_rejection_cannot_downgrade_to_missing_close() -> TestResult {
    let (dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    let (_prepared, body) = prepared(&store, &writer, &owner, 200)?;
    let (target, usage) = selected(&store, &owner)?;
    let candidate = dir.path().join("wrong-copy");
    std::fs::write(&candidate, b"wrong")?;
    let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
    let mut session = unresolved(
        store
            .begin_prepared_recovery(&owner, &target, &mut parent)?
            .read_backing()?,
    )?;
    check!(
        matches!(session.prepare_restore(ExactBodySource::SuppliedLocalCopy(&candidate)),
        Err(StoreError::Capture(CaptureError::PreparedCandidateRejected { reason: PreparedCandidateRejection::WrongLength { expected, observed: 5 } })) if expected == u64::try_from(body.len())?)
    );
    check!(matches!(
        session.missing_decision(),
        Err(StoreError::Capture(CaptureError::TransitionRefused))
    ));
    check!(matches!(
        session.finish(),
        Err(StoreError::Capture(CaptureError::TransitionRefused))
    ));
    check!(eq; std::fs::read(&candidate)?, b"wrong");
    Ok(())
}

#[test]
fn prepared_session_store_verification_rejects_noop_restore_and_out_of_order_finish() -> TestResult
{
    let (dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    let (_prepared, body) = prepared(&store, &writer, &owner, 200)?;
    let candidate = dir.path().join("exact-copy");
    std::fs::write(&candidate, &body)?;
    let (target, usage) = selected(&store, &owner)?;
    let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
    let mut session = unresolved(
        store
            .begin_prepared_recovery(&owner, &target, &mut parent)?
            .read_backing()?,
    )?;
    session.prepare_restore(ExactBodySource::SuppliedLocalCopy(&candidate))?;
    {
        let (_fence, _prepared, held, _io) = session.restoration_io()?;
        check!(eq; held, body.as_slice());
    }
    check!(matches!(
        session.verify_restored(),
        Err(StoreError::Capture(CaptureError::RequiredBackingMissing))
    ));
    check!(matches!(
        session.finish(),
        Err(StoreError::Capture(CaptureError::TransitionRefused))
    ));
    check!(eq; std::fs::read(&candidate)?, body);
    Ok(())
}

#[test]
fn prepared_session_recorded_retained_truth_survives_missing_consumption_backing() -> TestResult {
    let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    let (prepared, body) = prepared(&store, &writer, &owner, 200)?;
    let (target, usage) = selected(&store, &owner)?;
    let (body_path, _) = publish(&store, &prepared, &body)?;
    let occurrence = store.finish_acquisition_capture(prepared)?;
    let receipt = store
        .receipt(&read::terminal_operation(
            target.attempt(),
            owner.protocol(),
        ))?
        .ok_or("receipt")?;
    std::fs::remove_file(body_path)?;
    let before = rows(&store)?;
    let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
    let result = store
        .begin_prepared_recovery(&owner, &target, &mut parent)?
        .read_backing()?;
    check!(
        matches!(result, PreparedRecoveryRead::RetainedUnavailable { occurrence: held, receipt: held_receipt,
        consumption_error: StoreError::Capture(CaptureError::RequiredBackingMissing), .. } if held == occurrence && held_receipt == receipt)
    );
    check!(eq; rows(&store)?, before);
    Ok(())
}
