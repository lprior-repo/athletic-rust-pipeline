use super::*;

pub(super) fn second_prepared(
    store: &Store,
    writer: &CaptureWriterV1,
    owner: &ResultsCaptureOwnerV1,
) -> TestResult<AcquisitionWorkV1> {
    let url = URL.replace("meets/123", "meets/124");
    let request =
        RequestRepresentationV1::get(&url, vec![], "Census/1.0 (public-source qualification)")?;
    let exact = owner.for_owned_meet(UsJurisdiction::Michigan, "124", request)?;
    let grant =
        writer.continuation_for_owned_meet(store, owner, UsJurisdiction::Michigan, "124")?;
    let held = store.register_acquisition_work(owner, exact.clone(), grant)?;
    let reserved = store.reserve_acquisition_attempt(held)?;
    let dispatch = store.arm_acquisition_dispatch(
        reserved,
        DispatchRequestV1::new(&url, ConditionalRequestV1::Unconditional)?,
    )?;
    let observed = store.record_acquisition_response(
        dispatch.into_observation(),
        ResponseObservationV1::new(200, &url, "2026-10-14T05:00:00Z", vec![])?,
    )?;
    let digest = format!("{:x}", Sha256::digest(b"second"));
    let metadata = serde_json::to_vec_pretty(
        &serde_json::json!({"url":url,"response_url":url,"method":"GET","status":200,"content_digest":digest,"bytes":6,"fetched_at":"2026-10-14T05:00:00Z"}),
    )?;
    store.prepare_acquisition_capture(
        observed,
        PreparedCaptureInputV1::new(&digest, 6, metadata)?,
        CaptureDispositionV1::ReusableResponse,
    )?;
    Ok(exact)
}

#[test]
fn prepared_recovery_owner_carry_n_targets_performs_n_effect_reads_without_reset() -> TestResult {
    let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    prepared(&store, &writer, &owner, 200)?;
    let second = second_prepared(&store, &writer, &owner)?;
    let mut selection = AcquisitionReadBudget::for_selection();
    let mut targets = Vec::new();
    for exact in [work(&owner)?, second] {
        let ExistingRecoveryRoute::Prepared(target) =
            store.read_existing_recovery_target(&owner, &exact, &mut selection)?
        else {
            return Err("target".into());
        };
        targets.push(target);
    }
    let mut parent = PreparedRecoveryBudget::admit(selection.selection_usage(), targets.len())?;
    store.validate_prepared_recovery_owner(&owner, &mut parent)?;
    let gate = parent.reads().recovery_totals();
    for (index, target) in targets.iter().enumerate() {
        let mut session = unresolved(
            store
                .begin_prepared_recovery(&owner, target, &mut parent)?
                .read_backing()?,
        )?;
        let decision = session.missing_decision()?;
        session.close_missing(&decision)?;
        let total = parent.reads().recovery_totals();
        check!(eq; total.admission.used, gate.admission.used.checked_mul(u64::try_from(index + 1)?).ok_or("admission")?);
        check!(eq; total.admission_probes.used, gate.admission_probes.used.checked_mul(u64::try_from(index + 1)?).ok_or("probes")?);
    }
    check!(matches!(
        store.begin_prepared_recovery(&owner, targets.first().ok_or("target")?, &mut parent),
        Err(StoreError::Capture(CaptureError::ReadBudgetExceeded {
            resource: "recovery targets"
        }))
    ));
    Ok(())
}

#[test]
fn prepared_recovery_pending_owner_refuses_foreign_store_and_worker_before_effect() -> TestResult {
    let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    prepared(&store, &writer, &owner, 200)?;
    let (target, usage) = selected(&store, &owner)?;
    let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
    store.validate_prepared_recovery_owner(&owner, &mut parent)?;
    let before = rows(&store)?;
    let refused = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                matches!(
                    store.begin_prepared_recovery(&owner, &target, &mut parent),
                    Err(StoreError::Capture(
                        CaptureError::CaptureCapabilityForeignStore
                    ))
                )
            })
            .join()
    })
    .map_err(|_| "worker joined")?;
    check!(refused);
    check!(eq; rows(&store)?, before);
    check!(matches!(
        store.validate_prepared_recovery_owner(&owner, &mut parent),
        Err(StoreError::Capture(CaptureError::TransitionRefused))
    ));
    let (_other_dir, other, _, _) = fixture_for(CaptureProtocolVersion::V2)?;
    let mut foreign = PreparedRecoveryBudget::admit(usage, 1)?;
    store.validate_prepared_recovery_owner(&owner, &mut foreign)?;
    let other_before = rows(&other)?;
    check!(matches!(
        other.begin_prepared_recovery(&owner, &target, &mut foreign),
        Err(StoreError::Capture(
            CaptureError::CaptureCapabilityForeignStore
        ))
    ));
    check!(eq; rows(&other)?, other_before);
    Ok(())
}

#[test]
fn prepared_recovery_actual_copy_refuses_allocation_before_owner_copy_or_fs_effect() -> TestResult {
    let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    prepared(&store, &writer, &owner, 200)?;
    let (target, usage) = selected(&store, &owner)?;
    let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
    parent.reads().recovery_limit_owned(511);
    let before = rows(&store)?;
    check!(matches!(
        store.begin_prepared_recovery(&owner, &target, &mut parent),
        Err(StoreError::Capture(CaptureError::ReadBudgetExceeded {
            resource: "allocation"
        }))
    ));
    check!(eq; rows(&store)?, before);
    Ok(())
}

#[test]
fn prepared_recovery_fixed_error_tail_refuses_exact_diagnostic_n_minus_one_before_effect(
) -> TestResult {
    let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
    prepared(&store, &writer, &owner, 200)?;
    let (target, usage) = selected(&store, &owner)?;
    let mut parent = PreparedRecoveryBudget::admit(usage, 1)?;
    parent.limit_diagnostics(511);
    let before = rows(&store)?;
    check!(matches!(
        store.begin_prepared_recovery(&owner, &target, &mut parent),
        Err(StoreError::Capture(CaptureError::ReadBudgetExceeded {
            resource: "diagnostic bytes"
        }))
    ));
    check!(eq; parent.reads().recovery_totals().admission.used, 0);
    check!(eq; rows(&store)?, before);
    Ok(())
}
