use super::*;

#[test]
fn v2_selector_counts_absent_existing_and_prepared_without_writes_or_backing() -> TestResult {
    let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    let exact = work(&owner)?;
    let mut budget = AcquisitionReadBudget::for_selection();
    let before = rows(&store)?;
    check!(eq; store.read_existing_recovery_target(&owner, &exact, &mut budget)?, ExistingRecoveryRoute::Unregistered);
    check!(eq; rows(&store)?, before);
    let usage = budget.selection_usage();
    check!(eq; (usage.roots(), usage.encoded_bytes(), usage.receipt_probes(), usage.present_receipts(), usage.targets_examined(), usage.absent_target_probes()), (0, 0, 0, 0, 1, 1));
    drop(registered(&store, &writer, &owner)?);
    let before = rows(&store)?;
    check!(eq; store.read_existing_recovery_target(&owner, &exact, &mut budget)?, ExistingRecoveryRoute::ExistingNoPrepared);
    check!(eq; rows(&store)?, before);
    let first = budget.selection_usage();
    check!(eq; (first.roots(), first.receipt_probes(), first.targets_examined()), (2, 0, 2));
    let (prepared, _) = support::prepared(&store, &writer, &owner, 200)?;
    let target = PreparedRecoveryTarget::new(
        owner.store_identity().clone(),
        exact.slot().id()?,
        prepared.attempt_id().clone(),
        facts::digest(&prepared.prepared)?,
    )?;
    let before = rows(&store)?;
    check!(eq; store.read_existing_recovery_target(&owner, &exact, &mut budget)?, ExistingRecoveryRoute::Prepared(target));
    check!(eq; rows(&store)?, before);
    let usage = budget.selection_usage();
    check!(eq; (usage.roots(), usage.receipt_probes(), usage.present_receipts(), usage.targets_examined(), usage.absent_target_probes()), (8, 1, 0, 3, 1));
    check!(usage.encoded_bytes() > first.encoded_bytes());
    Ok(())
}

#[test]
fn v2_selector_debits_present_receipt_but_never_materializes_retained_body() -> TestResult {
    let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    let occurrence = retained(&store, &writer, &owner, 200)?;
    let path = store.http_cache_dir().join("archive/bodies");
    let mut entries = std::fs::read_dir(&path)?;
    let body = entries.next().ok_or("body")??.path();
    std::fs::remove_file(body)?;
    let before = rows(&store)?;
    let mut budget = AcquisitionReadBudget::for_selection();
    check!(eq; store.read_existing_recovery_target(&owner, &work(&owner)?, &mut budget)?, ExistingRecoveryRoute::ExistingNoPrepared);
    check!(eq; rows(&store)?, before);
    let usage = budget.selection_usage();
    check!(eq; (usage.roots(), usage.receipt_probes(), usage.present_receipts(), usage.targets_examined(), usage.absent_target_probes()), (8, 1, 1, 1, 0));
    let operation = read::terminal_operation(occurrence.attempt(), CaptureProtocolVersion::V2);
    let receipt = store.receipt(&operation)?.ok_or("receipt")?;
    check!(eq; receipt.appended, 0);
    check!(!receipt.credited);
    Ok(())
}

#[test]
fn selection_shared_root_budget_never_resets_between_targets() -> TestResult {
    let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    drop(registered(&store, &writer, &owner)?);
    let mut budget = AcquisitionReadBudget::new(3, 65_536, 0)?;
    check!(eq; store.read_existing_recovery_target(&owner, &work(&owner)?, &mut budget)?, ExistingRecoveryRoute::ExistingNoPrepared);
    check!(matches!(
        store.read_existing_recovery_target(&owner, &work(&owner)?, &mut budget),
        Err(StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
            resource: "roots",
            ..
        }))
    ));
    check!(eq; budget.selection_usage().targets_examined(), 2);
    Ok(())
}

#[test]
fn v1_selector_refuses_before_mutation_and_partial_v2_registration_refuses() -> TestResult {
    let (_dir, store, writer, owner) = fixture()?;
    let before = rows(&store)?;
    check!(matches!(
        store.read_existing_recovery_target(
            &owner,
            &work(&owner)?,
            &mut AcquisitionReadBudget::for_selection()
        ),
        Err(StoreError::Capture(
            CaptureError::CaptureProtocolUnsupported
        ))
    ));
    check!(eq; rows(&store)?, before);
    drop(writer);
    drop(store);
    let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    let exact = work(&owner)?;
    drop(registered(&store, &writer, &owner)?);
    store.journal.remove(Store::journal_key(
        owner.protocol().phase(),
        &format!("work/{}/identity", exact.id()?),
    ))?;
    let before = rows(&store)?;
    check!(matches!(
        store.read_existing_recovery_target(
            &owner,
            &exact,
            &mut AcquisitionReadBudget::for_selection()
        ),
        Err(StoreError::Capture(CaptureError::SlotWorkMismatch))
    ));
    check!(eq; rows(&store)?, before);
    Ok(())
}
