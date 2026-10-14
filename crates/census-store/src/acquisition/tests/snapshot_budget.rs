use super::*;

struct Admission {
    bytes: u64,
    lineage_bytes: u64,
}

fn expected_admission(store: &Store, protocol: CaptureProtocolVersion) -> TestResult<Admission> {
    let snapshot = store.db.snapshot();
    let mut bytes = 0_u64;
    for key in [
        "schema:store_version",
        "schema:key_format",
        "schema:lifecycle_revision",
        "schema:created_by",
        "schema:created_at",
        protocol.qualification_key(),
    ] {
        let value = snapshot
            .get(&store.meta, key)?
            .ok_or("missing admission fixture")?;
        bytes = bytes
            .checked_add(u64::try_from(
                key.len().checked_add(value.len()).ok_or("entry size")?,
            )?)
            .ok_or("admission size")?;
    }
    check!(snapshot.get(&store.meta, "schema:migrating_to")?.is_none());
    let key = Store::journal_key("export-lineage-v1", "store-identity");
    let value = snapshot
        .get(&store.journal, &key)?
        .ok_or("missing lineage fixture")?;
    let lineage_bytes = u64::try_from(key.len().checked_add(value.len()).ok_or("lineage size")?)?;
    Ok(Admission {
        bytes: bytes.checked_add(lineage_bytes).ok_or("admission size")?,
        lineage_bytes,
    })
}

fn leave_admission(budget: &mut AcquisitionReadBudget, remaining: u64) -> TestResult<u64> {
    let limit = budget.recovery_totals().admission.limit;
    let debit = limit
        .checked_sub(remaining)
        .ok_or("remaining admission exceeds limit")?;
    budget.admission_value(0, usize::try_from(debit)?)?;
    Ok(limit)
}

fn admission_refused<T>(result: StoreResult<T>) -> TestResult {
    match result {
        Err(StoreError::Capture(CaptureError::ReadBudgetExceeded {
            resource: "admission encoded bytes",
        })) => Ok(()),
        _ => Err("snapshot reader did not refuse exhausted admission bytes".into()),
    }
}

#[test]
fn reused_ordinary_budget_rechecks_corrupted_backing_on_the_same_snapshot() -> TestResult {
    for protocol in [CaptureProtocolVersion::V1, CaptureProtocolVersion::V2] {
        let (_dir, store, writer, owner) = fixture_for(protocol)?;
        let occurrence = retained(&store, &writer, &owner, 200)?;
        let snapshot = store.snapshot();
        let sequence = snapshot.sequence();
        let before = rows(&store)?;
        let mut budget = AcquisitionReadBudget::for_work();
        let verified = snapshot.read_retained_acquisition_v1(&occurrence, &mut budget)?;
        let original = std::fs::read(verified.body_path())?;
        let mut corrupted = original.clone();
        *corrupted.first_mut().ok_or("missing fixture body")? ^= 1;
        check!(eq; original.len(), corrupted.len());
        std::fs::write(verified.body_path(), &corrupted)?;
        check!(matches!(
            snapshot.read_retained_acquisition_v1(&occurrence, &mut budget),
            Err(StoreError::Capture(CaptureError::BackingIntegrity))
        ));
        check!(eq; std::fs::read(verified.body_path())?, corrupted);
        std::fs::write(verified.body_path(), &original)?;
        check!(eq; snapshot.read_retained_acquisition_v1(&occurrence, &mut budget)?.reference(), &occurrence);
        check!(eq; budget.selection_usage().roots(), 24);
        check!(eq; snapshot.sequence(), sequence);
        check!(eq; rows(&store)?, before);
    }
    Ok(())
}

#[test]
fn snapshot_selection_admission_exact_limit_succeeds_and_never_resets_on_reuse() -> TestResult {
    for protocol in [CaptureProtocolVersion::V1, CaptureProtocolVersion::V2] {
        let (_dir, store, writer, owner) = fixture_for(protocol)?;
        let occurrence = retained(&store, &writer, &owner, 200)?;
        let admission = expected_admission(&store, protocol)?;
        let snapshot = store.snapshot();
        let before = rows(&store)?;
        let mut budget = AcquisitionReadBudget::for_selection();
        let limit = leave_admission(&mut budget, admission.bytes)?;
        check!(eq; snapshot.read_retained_acquisition_v1(&occurrence, &mut budget)?.reference(), &occurrence);
        check!(eq; budget.recovery_totals().admission.used, limit);
        check!(eq; budget.recovery_totals().admission_probes.used, 12);
        let usage = budget.selection_usage();
        check!(eq; (usage.roots(), usage.receipt_probes(), usage.present_receipts()), (8, 1, 1));
        admission_refused(snapshot.read_retained_acquisition_v1(&occurrence, &mut budget))?;
        check!(eq; budget.recovery_totals().admission.used, limit);
        check!(eq; budget.recovery_totals().admission_probes.used, 13);
        check!(eq; budget.selection_usage(), usage);
        check!(eq; rows(&store)?, before);
    }
    Ok(())
}

#[test]
fn snapshot_selection_one_byte_short_refuses_before_closure_or_backing_reads() -> TestResult {
    for protocol in [CaptureProtocolVersion::V1, CaptureProtocolVersion::V2] {
        let (_dir, store, writer, owner) = fixture_for(protocol)?;
        let occurrence = retained(&store, &writer, &owner, 200)?;
        let admission = expected_admission(&store, protocol)?;
        let snapshot = store.snapshot();
        let before = rows(&store)?;
        let mut budget = AcquisitionReadBudget::for_selection();
        let remaining = admission
            .bytes
            .checked_sub(1)
            .ok_or("admission underflow")?;
        let limit = leave_admission(&mut budget, remaining)?;
        admission_refused(snapshot.read_retained_acquisition_v1(&occurrence, &mut budget))?;
        check!(eq; budget.recovery_totals().admission.used, limit.checked_sub(admission.lineage_bytes).and_then(|value| value.checked_add(1)).ok_or("partial admission")?);
        check!(eq; budget.recovery_totals().admission_probes.used, 12);
        let usage = budget.selection_usage();
        check!(eq; (usage.roots(), usage.encoded_bytes(), usage.receipt_probes()), (0, 0, 0));
        check!(eq; rows(&store)?, before);
    }
    Ok(())
}

#[test]
fn snapshot_recovery_exact_admission_preserves_backing_transition_refusal() -> TestResult {
    for protocol in [CaptureProtocolVersion::V1, CaptureProtocolVersion::V2] {
        let (_dir, store, writer, owner) = fixture_for(protocol)?;
        let occurrence = retained(&store, &writer, &owner, 200)?;
        let admission = expected_admission(&store, protocol)?;
        let snapshot = store.snapshot();
        let before = rows(&store)?;
        let mut budget = AcquisitionReadBudget::for_recovery(42, 3_122_688, 1)?;
        budget.begin_recovery_target()?;
        let limit = leave_admission(&mut budget, admission.bytes)?;
        check!(matches!(
            snapshot.read_retained_acquisition_v1(&occurrence, &mut budget),
            Err(StoreError::Capture(CaptureError::TransitionRefused))
        ));
        let totals = budget.recovery_totals();
        check!(eq; totals.admission.used, limit);
        check!(eq; totals.admission_probes.used, 12);
        check!(eq; (totals.roots.used, totals.receipts.used), (8, 1));
        check!(eq; totals.allocations.body.used, 0);
        admission_refused(snapshot.read_retained_acquisition_v1(&occurrence, &mut budget))?;
        check!(eq; budget.recovery_totals().admission.used, limit);
        check!(eq; budget.recovery_totals().admission_probes.used, 13);
        check!(eq; budget.recovery_totals().roots.used, 8);
        check!(eq; rows(&store)?, before);
    }
    Ok(())
}

#[test]
fn snapshot_recovery_one_byte_short_refuses_before_closure_or_backing_reads() -> TestResult {
    for protocol in [CaptureProtocolVersion::V1, CaptureProtocolVersion::V2] {
        let (_dir, store, writer, owner) = fixture_for(protocol)?;
        let occurrence = retained(&store, &writer, &owner, 200)?;
        let admission = expected_admission(&store, protocol)?;
        let snapshot = store.snapshot();
        let before = rows(&store)?;
        let mut budget = AcquisitionReadBudget::for_recovery(42, 3_122_688, 1)?;
        budget.begin_recovery_target()?;
        let remaining = admission
            .bytes
            .checked_sub(1)
            .ok_or("admission underflow")?;
        let limit = leave_admission(&mut budget, remaining)?;
        admission_refused(snapshot.read_retained_acquisition_v1(&occurrence, &mut budget))?;
        let totals = budget.recovery_totals();
        check!(eq; totals.admission.used, limit.checked_sub(admission.lineage_bytes).and_then(|value| value.checked_add(1)).ok_or("partial admission")?);
        check!(eq; totals.admission_probes.used, 12);
        check!(eq; (totals.roots.used, totals.encoded.used, totals.receipts.used), (0, 0, 0));
        check!(eq; totals.allocations.body.used, 0);
        check!(eq; rows(&store)?, before);
    }
    Ok(())
}
