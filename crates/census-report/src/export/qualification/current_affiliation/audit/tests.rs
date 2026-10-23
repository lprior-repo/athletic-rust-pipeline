use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn audit_deduplicates_and_orders_retained_references_and_reasons() -> TestResult {
    let id = AthleteId::mint("ath_subject", &["audit"]);
    let mut audit = RetainedAudit::default();
    for reference in ["b", "a", "b"] {
        audit.reference(&id, reference)?;
    }
    for reason in [
        Gap::OutsideCaptureDay,
        Gap::ArchiveUnavailable,
        Gap::OutsideCaptureDay,
    ] {
        audit.gap(id.clone(), reason)?;
    }
    let verified = audit.into_verified();
    let retained = verified.get(&id).ok_or("audit")?;
    check!(eq; retained.references(), &["a".to_string(), "b".to_string()]);
    check!(eq; retained.unavailable(), &["archive_unavailable".to_string(), "outside_capture_day".to_string()]);
    Ok(())
}

#[test]
fn audit_refuses_exact_per_subject_count_overflow_without_truncation() -> TestResult {
    let id = AthleteId::mint("ath_subject", &["audit"]);
    let mut audit = RetainedAudit::default();
    for index in 0..MAX_REFERENCES_PER_SUBJECT {
        audit.reference(&id, &format!("reference-{index}"))?;
    }
    audit.reference(&id, "reference-0")?;
    check!(matches!(
        audit.reference(&id, "extra"),
        Err(StoreError::BudgetExceeded {
            what: "current affiliation audit references per subject",
            needed: 257,
            available: 256,
        })
    ));
    check!(eq; audit.reference_count, MAX_REFERENCES_PER_SUBJECT);
    check!(eq; audit.into_verified().get(&id).ok_or("audit")?.references().len(), 256);
    Ok(())
}

#[test]
fn aggregate_admission_failure_preserves_all_existing_audit_state() -> TestResult {
    let id = AthleteId::mint("ath_subject", &["audit"]);
    let mut references = RetainedAudit {
        reference_count: OBSERVATION_BUDGET.rows,
        ..Default::default()
    };
    check!(matches!(
        references.reference(&id, "one"),
        Err(StoreError::BudgetExceeded { needed, available, .. })
            if available == OBSERVATION_BUDGET.rows
                && needed.checked_sub(available) == Some(1)
    ));
    check!(references.references.is_empty());
    check!(eq; references.reference_count, OBSERVATION_BUDGET.rows);
    let mut reasons = RetainedAudit {
        reason_count: OBSERVATION_BUDGET.rows,
        ..Default::default()
    };
    check!(matches!(
        reasons.gap(id.clone(), Gap::NoRetainedRow),
        Err(StoreError::BudgetExceeded { needed, available, .. })
            if available == OBSERVATION_BUDGET.rows
                && needed.checked_sub(available) == Some(1)
    ));
    check!(reasons.unavailable.is_empty());
    check!(eq; reasons.reason_count, OBSERVATION_BUDGET.rows);
    let mut bytes = RetainedAudit {
        bytes: MAX_OBJECT_BYTES,
        ..Default::default()
    };
    check!(matches!(
        bytes.reference(&id, "x"),
        Err(StoreError::BudgetExceeded { needed, available, .. })
            if available == 4 * 1024 * 1024 * 1024
                && needed.checked_sub(available) == Some(1)
    ));
    check!(bytes.references.is_empty());
    check!(eq; bytes.reference_count, 0);
    check!(matches!(
        bytes.gap(id, Gap::ArchiveUnavailable),
        Err(StoreError::BudgetExceeded { needed, available, .. })
            if available == 4 * 1024 * 1024 * 1024 && needed > available
    ));
    check!(bytes.unavailable.is_empty());
    check!(eq; bytes.reason_count, 0);
    Ok(())
}
