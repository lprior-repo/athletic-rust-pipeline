use super::*;
mod expected;

#[test]
fn maximum_supported_lineage_cold_recovery_charges_two_actual_reads_and_34_probes() -> TestResult {
    exercise(None)
}

#[test]
fn padded_public_admission_keeps_exact_cap_and_one_byte_short_refusal() -> TestResult {
    for short in [false, true] {
        exercise(Some(short))?;
    }
    Ok(())
}

fn exercise(exhaust_admission: Option<bool>) -> TestResult {
    let f = cold_fixture()?;
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let work = owner.for_root(&f.store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let (prepared, body) = observed::prepare(&f.store, &f.writer, &owner)?;
    let input = prepared.prepared().clone();
    observed::publish(&f.store, &input, &body)?;
    let mut selection = AcquisitionReadBudget::for_selection();
    let ExistingRecoveryRoute::Prepared(target) =
        f.store
            .read_existing_recovery_target(&owner, &work, &mut selection)?
    else {
        return Err("padded public prepared target absent".into());
    };
    let exact = expected::admission(&f.store, &work)?;
    let mut budget = PreparedRecoveryBudget::admit(selection.selection_usage(), 1)?;
    f.store
        .validate_prepared_recovery_owner(&owner, &mut budget)?;
    let cap = budget.reads().recovery_totals().admission.limit;
    check!(eq; cap, 3_350_552);
    check!(exact < cap);
    if let Some(short) = exhaust_admission {
        let debit = cap
            .checked_sub(exact)
            .and_then(|value| value.checked_add(u64::from(short)))
            .ok_or("public admission predebit")?;
        budget.reads().admission_value(0, usize::try_from(debit)?)?;
    }
    let before = Witness::capture(&f.store)?;
    let result = execute(&f.store, &owner, &target, &mut budget);
    if exhaust_admission == Some(true) {
        check!(matches!(
            result,
            Err(StoreError::Capture(CaptureError::ReadBudgetLimitExceeded {
                resource: "admission encoded bytes",
                ..
            }))
        ));
        check!(eq; Witness::capture(&f.store)?, before);
    } else {
        let parts = result?;
        check!(eq; parts.body, body);
        check!(eq; parts.prepared, input);
        check!(eq; parts.occurrence.attempt(), target.attempt());
        check!(eq; budget.reads().recovery_totals().admission.used,
            if exhaust_admission.is_some() { cap } else { exact });
        check!(eq; budget.reads().recovery_totals().admission_probes.used, 34);
    }
    check!(eq; expected::lineage(&f.store)?.len(), crate::MAX_JOURNAL_VALUE_BYTES);
    check!(eq; std::fs::read(expected::body_path(&f.store, &input))?, body);
    Ok(())
}

fn execute(
    store: &Store,
    owner: &PublicResourceCaptureOwnerV1,
    target: &PreparedRecoveryTarget,
    budget: &mut PreparedRecoveryBudget,
) -> crate::StoreResult<RetainedCaptureParts> {
    match store
        .begin_prepared_recovery(owner, target, budget)?
        .read_backing()?
    {
        PreparedRecoveryRead::Unresolved(session) => session.finish().map(|body| body.into_parts()),
        _ => Err(CaptureError::PreparedTargetMismatch.into()),
    }
}

fn cold_fixture() -> TestResult<Fixture> {
    let f = Fixture::new()?;
    f.declare("2026-10-14T04:00:00Z")?;
    let key = Store::journal_key("export-lineage-v1", "store-identity");
    let mut raw = expected::lineage(&f.store)?;
    let missing = crate::MAX_JOURNAL_VALUE_BYTES
        .checked_sub(raw.len())
        .ok_or("lineage length")?;
    raw.extend(std::iter::repeat_n(b' ', missing));
    f.store.journal.insert(&key, &raw)?;
    f.store.flush()?;
    let before = Witness::capture(&f.store)?;
    let root = f.directory.path().join("store");
    let Fixture {
        directory,
        store,
        writer,
        binding,
    } = f;
    drop((writer, store));
    for current in [true, false] {
        let store = cold_open(&root, current)?;
        check!(eq; Witness::capture(&store)?, before);
        check!(eq; expected::lineage(&store)?, raw);
        drop(store);
    }
    let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
    check!(eq; expected::lineage(&store)?, raw);
    Ok(Fixture {
        directory,
        store,
        writer,
        binding,
    })
}
