use super::*;
use crate::acquisition::{
    AcquisitionSelectionUsage, ExistingRecoveryRoute, PreparedRecoveryBudget, PreparedRecoveryRead,
    RetainedCaptureParts,
};

mod fixture;
use fixture::{execute, selected, PreparedChild};

#[test]
fn child_prepared_cold_recovery_and_replay_keep_parent_and_exact_six_or_eight_core_roots(
) -> TestResult {
    let prepared = PreparedChild::new()?.cold_roundtrip()?;
    let store = &prepared.f.store;
    let owner = &prepared.owner;
    let original = &prepared.original;
    let (target, usage) = selected(store, owner, &prepared.work)?;
    check!(eq; target.attempt(), &prepared.attempt);
    check!(eq; (usage.roots(), usage.receipt_probes(), usage.present_receipts()), (6, 1, 0));
    let expected = fixture::core_usage(store, &target)?;
    let mut budget = PreparedRecoveryBudget::admit(usage, 1)?;
    budget
        .reads()
        .recovery_narrow_physical_reads(6, usize::try_from(usage.encoded_bytes())?, 1);
    let parts = execute(store, owner, &target, &mut budget)?;
    check!(eq; budget.reads().selection_usage(), expected);
    check!(budget.reads().public_parent_usage().rows() > 0);
    check!(eq; parts.prepared, prepared.input);
    check!(eq; parts.body, prepared.body);
    check!(eq; parts.occurrence.attempt(), &prepared.attempt);
    check!(eq; parts.original_occurrence, parts.occurrence);
    replay(store, owner, &target, &parts, usage)?;
    let retained =
        store.read_retained_acquisition_v1(original, &mut AcquisitionReadBudget::for_work())?;
    check!(eq; retained.reference(), original);
    check!(eq; retained.body_reference(), original);
    Ok(())
}

fn replay(
    store: &Store,
    owner: &PublicResourceCaptureOwnerV1,
    target: &PreparedRecoveryTarget,
    original: &RetainedCaptureParts,
    selected_usage: AcquisitionSelectionUsage,
) -> TestResult {
    let usage = fixture::core_usage(store, target)?;
    check!(eq; (usage.roots(), usage.receipt_probes(), usage.present_receipts()), (8, 1, 1));
    let before = Witness::capture(store)?;
    let mut budget = PreparedRecoveryBudget::admit(selected_usage, 1)?;
    budget
        .reads()
        .recovery_narrow_physical_reads(8, usize::try_from(usage.encoded_bytes())?, 1);
    let repeated = execute(store, owner, target, &mut budget)?;
    check!(eq; repeated.occurrence, original.occurrence);
    check!(eq; repeated.receipt, original.receipt);
    check!(eq; repeated.prepared, original.prepared);
    check!(eq; repeated.body, original.body);
    check!(eq; budget.reads().selection_usage(), usage);
    check!(budget.reads().public_parent_usage().rows() > 0);
    check!(eq; Witness::capture(store)?, before);
    Ok(())
}

#[test]
fn cached_child_recovery_rechecks_parent_terminal_receipt_and_result_before_any_write() -> TestResult
{
    for fault in ["terminal-receipt", "result-reference"] {
        let prepared = PreparedChild::new()?;
        let store = &prepared.f.store;
        let (target, usage) = selected(store, &prepared.owner, &prepared.work)?;
        let mut budget = PreparedRecoveryBudget::admit(usage, 1)?;
        store.validate_prepared_recovery_owner(&prepared.owner, &mut budget)?;
        let assessed = assessment(store, &prepared.owner, &prepared.original)?;
        fixture::damage_parent(store, &prepared.original, fault)?;
        let before = Witness::capture(store)?;
        check!(store
            .begin_prepared_recovery(&prepared.owner, &target, &mut budget)
            .is_err());
        check!(eq; Witness::capture(store)?, before);
        check!(store.prepare_public_roster_child(&assessed).is_err());
        check!(store
            .read_existing_recovery_target(
                &prepared.owner,
                &prepared.work,
                &mut AcquisitionReadBudget::for_selection()
            )
            .is_err());
        check!(eq; Witness::capture(store)?, before);
    }
    Ok(())
}

#[test]
fn child_core_recovery_one_root_short_refuses_without_spending_parent_reads_as_child_roots(
) -> TestResult {
    for retained in [false, true] {
        let prepared = PreparedChild::new()?;
        let store = &prepared.f.store;
        let (target, usage) = selected(store, &prepared.owner, &prepared.work)?;
        if retained {
            let owned = registered(
                store,
                &prepared.f.writer,
                &prepared.owner,
                prepared.work.clone(),
            )?;
            store.finish_prepared_acquisition(owned, &mut AcquisitionReadBudget::for_work())?;
        }
        let expected = fixture::core_usage(store, &target)?;
        check!(eq; expected.roots(), if retained { 8 } else { 6 });
        let limit = expected.roots().checked_sub(1).ok_or("short child roots")?;
        let mut budget = PreparedRecoveryBudget::admit(usage, 1)?;
        budget.reads().recovery_narrow_physical_reads(
            usize::try_from(limit)?,
            usize::try_from(expected.encoded_bytes())?,
            1,
        );
        let before = Witness::capture(store)?;
        check!(matches!(
            execute(store, &prepared.owner, &target, &mut budget),
            Err(StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
                resource: "roots",
                ..
            }))
        ));
        check!(eq; budget.reads().selection_usage().roots(), limit);
        check!(eq; Witness::capture(store)?, before);
    }
    Ok(())
}
