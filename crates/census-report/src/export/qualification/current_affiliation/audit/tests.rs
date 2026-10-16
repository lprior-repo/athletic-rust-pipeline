use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn audit_is_deduplicated_sorted_and_counts_exact_retained_values() -> TestResult {
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
    check!(eq; audit.reference_count, 2);
    check!(eq; audit.reason_count, 2);
    check!(eq; audit.bytes, 40);
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
fn audit_aggregate_count_and_bytes_have_typed_exact_refusals() -> TestResult {
    let id = AthleteId::mint("ath_subject", &["audit"]);
    let mut references = RetainedAudit {
        reference_count: MAX_REFERENCES,
        ..Default::default()
    };
    check!(matches!(
        references.reference(&id, "one"),
        Err(StoreError::BudgetExceeded {
            what: "current affiliation audit reference count",
            needed: 1_000_001,
            available: MAX_REFERENCES,
        })
    ));
    let mut reasons = RetainedAudit {
        reason_count: MAX_REASONS,
        ..Default::default()
    };
    check!(matches!(
        reasons.gap(id.clone(), Gap::NoRetainedRow),
        Err(StoreError::BudgetExceeded {
            what: "current affiliation audit reason count",
            needed: 1_200_001,
            available: MAX_REASONS,
        })
    ));
    let mut bytes = RetainedAudit {
        bytes: MAX_BYTES,
        ..Default::default()
    };
    check!(
        matches!(bytes.reference(&id, "x"), Err(StoreError::BudgetExceeded {
        what: "current affiliation audit bytes", needed, available: MAX_BYTES,
    }) if needed == MAX_BYTES + 1)
    );
    check!(eq; bytes.reference_count, 0);
    check!(
        matches!(bytes.gap(id, Gap::ArchiveUnavailable), Err(StoreError::BudgetExceeded {
        what: "current affiliation audit bytes", needed, available: MAX_BYTES,
    }) if needed == MAX_BYTES + 19)
    );
    check!(eq; bytes.reason_count, 0);
    Ok(())
}

#[test]
fn audit_reason_labels_are_fixed_and_complete() -> TestResult {
    check!(eq; [Gap::NoRetainedRow, Gap::IdentityUnaccepted, Gap::OutsideCaptureDay,
        Gap::ArchiveUnavailable, Gap::SchoolUnresolved, Gap::OutsideSelectedSport].map(Gap::label),
        ["no_retained_row", "identity_unaccepted", "outside_capture_day", "archive_unavailable",
        "school_unresolved", "outside_selected_sport"]);
    Ok(())
}
