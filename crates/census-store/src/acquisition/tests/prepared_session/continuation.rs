use super::*;

#[test]
fn prepared_retention_failure_continuation_preflight_refuses_corruption_without_changing_receipt(
) -> TestResult {
    for corruption in [false, true] {
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
        let receipt = failed.receipt().clone();
        if corruption {
            let (body_path, _) = publish(&store, &prepared, &body)?;
            std::fs::write(body_path, b"occupied wrong body")?;
        }
        let held = registered(&store, &writer, &owner)?;
        let before = rows(&store)?;
        let next = store.reserve_acquisition_attempt(held);
        if corruption {
            check!(matches!(
                next,
                Err(StoreError::Capture(CaptureError::PreparedRepairRequired))
            ));
            check!(eq; rows(&store)?, before);
        } else {
            check!(eq; next?.attempt_id().ordinal().get(), 2);
        }
        check!(eq; store.receipt(&receipt.operation)?.ok_or("receipt")?, receipt);
    }
    Ok(())
}
