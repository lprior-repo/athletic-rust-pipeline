use super::*;
use census_domain::model::{
    AcquisitionFailureV1, CaptureProtocolVersion, ConditionalRequestV1, DispatchRequestV1,
    RequestRepresentationV1, ResponseObservationV1,
};
use census_store::read::{JournalNamespace, ReadBudget};
use census_store::{
    AcquireDecision, AcquisitionReadBudget, CaptureError, OwnedAcquisitionWork, StoreError,
    StoreSnapshot,
};

const URL: &str =
    "https://mi.milesplit.com/api/v1/meets/123/performances?isMeetPro=0&fields=athlete.id";

#[test]
fn original_owner_pass_continuations_keep_one_slot_and_lifetime_cap() -> TestResult {
    let f = fixture()?;
    bind(&f)?;
    let mut original = None;
    for expected in 1..=3 {
        let ResultsCaptureExecution::FreshQualification { writer, owner, .. } = admit(&f)? else {
            return Err("capture owner missing".into());
        };
        let work = owner.for_owned_meet(
            UsJurisdiction::Michigan,
            "123",
            RequestRepresentationV1::get(URL, Vec::new(), "test-client")?,
        )?;
        let identity = work.id()?;
        if let Some(first) = original.as_ref() {
            check!(eq; &identity, first);
        } else {
            original = Some(identity);
        }
        let grant = writer.continuation_for_owned_meet(
            &f.store,
            &owner,
            UsJurisdiction::Michigan,
            "123",
        )?;
        let work = f.store.register_acquisition_work(&owner, work, grant)?;
        let reservation = f.store.reserve_acquisition_attempt(work)?;
        let armed = f.store.arm_acquisition_dispatch(
            reservation,
            DispatchRequestV1::new(URL, ConditionalRequestV1::Unconditional)?,
        )?;
        check!(eq; armed.dispatch_id().attempt().ordinal().get(), expected);
        let terminal = f
            .store
            .fail_acquisition_dispatch(armed.into_observation(), AcquisitionFailureV1::Transport)?;
        check!(
            matches!(terminal.terminal(), census_store::AcquisitionTerminal::OutcomeUnknown { checkpoint: census_store::AcquisitionCheckpointV1::Armed { hop }, cause: census_domain::model::InterruptionCauseV1::TransportFailed } if hop.get() == 0)
        );
    }
    let ResultsCaptureExecution::FreshQualification { writer, owner, .. } = admit(&f)? else {
        return Err("capture owner missing".into());
    };
    let work = owner.for_owned_meet(
        UsJurisdiction::Michigan,
        "123",
        RequestRepresentationV1::get(URL, Vec::new(), "test-client")?,
    )?;
    let grant =
        writer.continuation_for_owned_meet(&f.store, &owner, UsJurisdiction::Michigan, "123")?;
    let work = f.store.register_acquisition_work(&owner, work, grant)?;
    let decision = f
        .store
        .inspect_acquisition_work(&work, &mut AcquisitionReadBudget::for_work())?;
    check!(matches!(decision, AcquireDecision::Interrupted));
    let before = f.store.snapshot();
    check!(matches!(
        f.store.reserve_acquisition_attempt(work),
        Err(StoreError::Capture(CaptureError::RetainedOutcomeUnknown))
    ));
    assert_three_attempts_unchanged(&f.store, &before)?;
    Ok(())
}

#[test]
fn same_original_owner_cannot_reset_allowance_by_changing_refresh_or_history() -> TestResult {
    let mut f = fixture()?;
    bind(&f)?;
    let ResultsCaptureExecution::FreshQualification { writer, owner, .. } = admit(&f)? else {
        return Err("capture owner missing".into());
    };
    let work = owner.for_owned_meet(
        UsJurisdiction::Michigan,
        "123",
        RequestRepresentationV1::get(URL, Vec::new(), "test-client")?,
    )?;
    let grant =
        writer.continuation_for_owned_meet(&f.store, &owner, UsJurisdiction::Michigan, "123")?;
    drop(f.store.register_acquisition_work(&owner, work, grant)?);
    f.request.refresh = true;
    let ResultsCaptureExecution::FreshQualification { writer, owner, .. } = admit(&f)? else {
        return Err("capture owner missing".into());
    };
    let changed = owner.for_owned_meet(
        UsJurisdiction::Michigan,
        "123",
        RequestRepresentationV1::get(URL, Vec::new(), "test-client")?,
    )?;
    let grant =
        writer.continuation_for_owned_meet(&f.store, &owner, UsJurisdiction::Michigan, "123")?;
    check!(matches!(
        f.store.register_acquisition_work(&owner, changed, grant),
        Err(StoreError::Capture(CaptureError::ObligationConflict))
    ));
    f.request.history = HistoryWindow::new(2022, 2026, "2026-10-14")?;
    check!(matches!(
        admit(&f),
        Err(CaptureAdmissionError::HistoryConflict)
    ));
    Ok(())
}

#[test]
fn observed_retryable_response_failures_exhaust_the_original_owner_lifetime_cap() -> TestResult {
    let f = fixture()?;
    bind(&f)?;
    for expected in 1..=3 {
        let reservation = f.store.reserve_acquisition_attempt(registered(&f)?)?;
        let armed = f.store.arm_acquisition_dispatch(
            reservation,
            DispatchRequestV1::new(URL, ConditionalRequestV1::Unconditional)?,
        )?;
        check!(eq; armed.dispatch_id().attempt().ordinal().get(), expected);
        let observed = f.store.record_acquisition_response(
            armed.into_observation(),
            ResponseObservationV1::new(500, URL, "2026-10-14T05:00:00Z", Vec::new())?,
        )?;
        check!(eq; observed.response().status(), 500);
        let terminal = f
            .store
            .fail_acquisition_response(observed, AcquisitionFailureV1::BodyIncomplete)?;
        check!(matches!(
            terminal.terminal(),
            census_store::AcquisitionTerminal::FailedUncaptured {
                checkpoint: census_store::AcquisitionCheckpointV1::ResponseObserved { hop },
                failure: AcquisitionFailureV1::BodyIncomplete,
            } if hop.get() == 0
        ));
    }
    let work = registered(&f)?;
    check!(matches!(
        f.store
            .inspect_acquisition_work(&work, &mut AcquisitionReadBudget::for_work())?,
        AcquireDecision::Exhausted
    ));
    let before = f.store.snapshot();
    check!(matches!(
        f.store.reserve_acquisition_attempt(work),
        Err(StoreError::Capture(
            CaptureError::AcquisitionBudgetExhausted
        ))
    ));
    assert_three_attempts_unchanged(&f.store, &before)?;
    Ok(())
}

fn registered(f: &Fixture) -> TestResult<OwnedAcquisitionWork> {
    let ResultsCaptureExecution::FreshQualification { writer, owner, .. } = admit(f)? else {
        return Err("capture owner missing".into());
    };
    let work = owner.for_owned_meet(
        UsJurisdiction::Michigan,
        "123",
        RequestRepresentationV1::get(URL, Vec::new(), "test-client")?,
    )?;
    let grant =
        writer.continuation_for_owned_meet(&f.store, &owner, UsJurisdiction::Michigan, "123")?;
    Ok(f.store.register_acquisition_work(&owner, work, grant)?)
}

fn assert_three_attempts_unchanged(store: &Store, before: &StoreSnapshot<'_>) -> TestResult {
    let after = store.snapshot();
    for prefix in ["work/", "slot/"] {
        let namespace = JournalNamespace::new(CaptureProtocolVersion::V2.phase(), prefix)?;
        let first = before.journal_namespace_fingerprint(namespace, journal_budget())?;
        let second = after.journal_namespace_fingerprint(namespace, journal_budget())?;
        check!(first.same_contents(store, &second)?);
    }
    check!(eq; reserved_count(before)?, 3);
    check!(eq; reserved_count(&after)?, 3);
    Ok(())
}

fn reserved_count(snapshot: &StoreSnapshot<'_>) -> TestResult<u64> {
    let mut reserved = 0_u64;
    snapshot.for_each_journal_root_bounded(
        JournalNamespace::new(CaptureProtocolVersion::V2.phase(), "work/")?,
        journal_budget(),
        |key| {
            if key.ends_with("/reserved") {
                reserved = reserved.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            }
            Ok(())
        },
    )?;
    Ok(reserved)
}

fn journal_budget() -> ReadBudget {
    ReadBudget {
        rows: 64,
        bytes: 8 * 1024 * 1024,
        record_bytes: census_store::MAX_JOURNAL_VALUE_BYTES,
    }
}
