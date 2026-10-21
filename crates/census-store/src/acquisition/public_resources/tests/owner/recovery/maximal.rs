use super::*;
use crate::acquisition::AcquisitionSelectionUsage;
use crate::{RetainedCaptureParts, StoreResult};

mod wire;

#[test]
fn public_three_attempt_five_hop_recovery_uses_exact_40_or_42_closure_roots() -> TestResult {
    for retained in [false, true] {
        for prevalidated in [false, true] {
            exercise(retained, prevalidated, false)?;
        }
    }
    Ok(())
}

#[test]
fn public_three_attempt_five_hop_recovery_refuses_39_or_41_roots_without_mutation() -> TestResult {
    for retained in [false, true] {
        exercise(retained, true, true)?;
    }
    Ok(())
}

fn exercise(retained: bool, prevalidated: bool, short: bool) -> TestResult {
    let f = Fixture::new()?;
    f.declare("2026-10-14T04:00:00Z")?;
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let (prepared, body) = maximal(&f, &owner)?;
    let input = prepared.prepared().clone();
    observed::publish(&f.store, &input, &body)?;
    let (target, usage) = selected(&f.store, &owner)?;
    check!(eq; target.attempt().ordinal().get(), 3);
    check!(eq; (usage.roots(), usage.receipt_probes(), usage.present_receipts()), (40, 3, 2));
    if retained {
        check!(eq; f.store.finish_acquisition_capture(prepared)?.attempt(), target.attempt());
    }
    let expected = core_usage(&f.store, &target)?;
    check!(eq; expected.roots(), if retained { 42 } else { 40 });
    let limit = expected
        .roots()
        .checked_sub(u64::from(short))
        .ok_or("root limit")?;
    let mut budget = PreparedRecoveryBudget::admit(usage, 1)?;
    if prevalidated {
        f.store
            .validate_prepared_recovery_owner(&owner, &mut budget)?;
    }
    budget.reads().recovery_narrow_physical_reads(
        usize::try_from(limit)?,
        usize::try_from(expected.encoded_bytes())?,
        3,
    );
    let before = Witness::capture(&f.store)?;
    let result = execute(&f.store, &owner, &target, &mut budget);
    if short {
        check!(matches!(
            result,
            Err(StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
                resource: "roots",
                ..
            }))
        ));
        check!(eq; Witness::capture(&f.store)?, before);
        check!(eq; budget.reads().selection_usage().roots(), limit);
    } else {
        let parts = result?;
        check!(eq; parts.body, body);
        check!(eq; parts.prepared, input);
        check!(eq; parts.occurrence.attempt(), target.attempt());
        check!(eq; budget.reads().selection_usage(), expected);
        check!(budget.reads().recovery_totals().admission.used > 0);
        if retained {
            check!(eq; Witness::capture(&f.store)?, before);
        }
        replay(&f.store, &owner, &target, usage, &parts)?;
    }
    check!(eq; wire::archive_pair(&f.store, &input)?, (body, input.metadata_bytes().to_vec()));
    Ok(())
}

fn maximal(
    f: &Fixture,
    owner: &PublicResourceCaptureOwnerV1,
) -> TestResult<(crate::PreparedCapture, Vec<u8>)> {
    for status in [500, 429] {
        let (prepared, body) = wire::prepare(f, owner, status)?;
        observed::publish(&f.store, prepared.prepared(), &body)?;
        f.store.finish_acquisition_capture(prepared)?;
    }
    wire::prepare(f, owner, 200)
}

fn selected(
    store: &Store,
    owner: &PublicResourceCaptureOwnerV1,
) -> TestResult<(PreparedRecoveryTarget, AcquisitionSelectionUsage)> {
    let work = owner.for_root(store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let mut budget = AcquisitionReadBudget::for_selection();
    let ExistingRecoveryRoute::Prepared(target) =
        store.read_existing_recovery_target(owner, &work, &mut budget)?
    else {
        return Err("maximal public prepared target absent".into());
    };
    check!(budget.recovery_totals().admission.used > 0);
    Ok((target, budget.selection_usage()))
}

fn core_usage(
    store: &Store,
    target: &PreparedRecoveryTarget,
) -> TestResult<AcquisitionSelectionUsage> {
    let mut budget = AcquisitionReadBudget::for_work();
    let facts = crate::acquisition::read::read_work(
        &store.db.snapshot(),
        &store.journal,
        &store.receipts,
        target.attempt().work_id(),
        &mut budget,
        CaptureProtocolVersion::V2,
    )?;
    check!(eq; facts.attempts.len(), 3);
    for attempt in &facts.attempts {
        check!(eq; attempt.dispatches.len(), 5);
    }
    Ok(budget.selection_usage())
}

fn execute(
    store: &Store,
    owner: &PublicResourceCaptureOwnerV1,
    target: &PreparedRecoveryTarget,
    budget: &mut PreparedRecoveryBudget,
) -> StoreResult<RetainedCaptureParts> {
    match store
        .begin_prepared_recovery(owner, target, budget)?
        .read_backing()?
    {
        PreparedRecoveryRead::Unresolved(session) => session.finish().map(|body| body.into_parts()),
        PreparedRecoveryRead::Retained(body) => Ok(body.into_parts()),
        _ => Err(CaptureError::PreparedTargetMismatch.into()),
    }
}

fn replay(
    store: &Store,
    owner: &PublicResourceCaptureOwnerV1,
    target: &PreparedRecoveryTarget,
    usage: AcquisitionSelectionUsage,
    original: &RetainedCaptureParts,
) -> TestResult {
    let before = Witness::capture(store)?;
    let mut budget = PreparedRecoveryBudget::admit(usage, 1)?;
    let repeated = execute(store, owner, target, &mut budget)?;
    check!(eq; repeated.occurrence, original.occurrence);
    check!(eq; repeated.receipt, original.receipt);
    check!(eq; repeated.body, original.body);
    check!(eq; repeated.prepared, original.prepared);
    check!(eq; Witness::capture(store)?, before);
    check!(eq; budget.reads().selection_usage().roots(), 42);
    Ok(())
}
