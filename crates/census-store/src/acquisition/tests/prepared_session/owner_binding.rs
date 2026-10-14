use super::*;

#[test]
fn prepared_recovery_parent_keeps_original_worker_store_and_scope_after_first_commit() -> TestResult
{
    for foreign in ["worker", "store", "scope"] {
        let (_dir, store, writer, owner) = fixture_for(CaptureProtocolVersion::V2)?;
        prepared(&store, &writer, &owner, 200)?;
        let second = super::owner::second_prepared(&store, &writer, &owner)?;
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
        let mut parent = PreparedRecoveryBudget::admit(selection.selection_usage(), 2)?;
        store.validate_prepared_recovery_owner(&owner, &mut parent)?;
        let mut first = unresolved(
            store
                .begin_prepared_recovery(&owner, targets.first().ok_or("first")?, &mut parent)?
                .read_backing()?,
        )?;
        let decision = first.missing_decision()?;
        first.close_missing(&decision)?;
        let before = rows(&store)?;
        let reads = parent.reads().recovery_totals();
        let second = targets.get(1).ok_or("second")?;
        let refused = match foreign {
            "worker" => std::thread::scope(|scope| {
                scope
                    .spawn(|| {
                        matches!(
                            store.begin_prepared_recovery(&owner, second, &mut parent),
                            Err(StoreError::Capture(
                                CaptureError::CaptureCapabilityForeignStore
                            ))
                        )
                    })
                    .join()
            })
            .map_err(|_| "worker joined")?,
            "store" => {
                let (_other_dir, other, other_writer, other_owner) =
                    fixture_for(CaptureProtocolVersion::V2)?;
                prepared(&other, &other_writer, &other_owner, 200)?;
                let (other_target, _) = selected(&other, &other_owner)?;
                let other_before = rows(&other)?;
                let refused = matches!(
                    other.begin_prepared_recovery(&other_owner, &other_target, &mut parent),
                    Err(StoreError::Capture(
                        CaptureError::CaptureCapabilityForeignStore
                    ))
                );
                check!(eq; rows(&other)?, other_before);
                refused
            }
            "scope" => {
                let foreign_owner = super::super::support::owner(
                    &store,
                    &writer,
                    2025,
                    CaptureCacheIntent::ReuseEligible,
                )?;
                matches!(
                    store.begin_prepared_recovery(&foreign_owner, second, &mut parent),
                    Err(StoreError::Capture(
                        CaptureError::CaptureCapabilityForeignStore
                    ))
                )
            }
            _ => return Err("foreign scenario".into()),
        };
        check!(refused);
        let after = parent.reads().recovery_totals();
        check!(eq; after.admission.used, reads.admission.used);
        check!(eq; after.admission_probes.used, reads.admission_probes.used);
        check!(after.allocations.owned.used >= reads.allocations.owned.used);
        check!(eq; rows(&store)?, before);
        check!(matches!(
            store.begin_prepared_recovery(&owner, second, &mut parent),
            Err(StoreError::Capture(CaptureError::ReadBudgetExceeded {
                resource: "recovery targets"
            })) | Err(StoreError::Capture(CaptureError::TransitionRefused))
        ));
    }
    Ok(())
}
