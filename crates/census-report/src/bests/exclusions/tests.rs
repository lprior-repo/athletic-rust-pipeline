use super::*;
use census_domain::model::Id;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn exclusion_row_admission_accepts_exact_budget_then_refuses_without_growth() -> TestResult {
    let mut exclusions = Exclusions::new();
    exclusions.reserve(crate::grouping::STAGE_BYTES)?;
    let capacity = exclusions.rows.capacity();
    check!(matches!(
        exclusions.reserve(1),
        Err(ReportError::Grouping(GroupingError::BudgetExceeded { .. }))
    ));
    check!(eq; exclusions.rows.capacity(), capacity);
    check!(exclusions.rows.is_empty());
    Ok(())
}

#[test]
fn exclusion_allocation_bound_checks_addition_and_multiplication_overflow() -> TestResult {
    check!(eq; exclusion_bytes([2, 3, 5])?,
        (std::mem::size_of::<CombinedExclusion>() + 10) * 4);
    for lengths in [[usize::MAX, 0, 0], [usize::MAX / 4, 0, 0]] {
        check!(matches!(
            exclusion_bytes(lengths),
            Err(GroupingError::Overflow { .. })
        ));
    }
    let mut exclusions = Exclusions::new();
    check!(matches!(
        exclusions.reserve(crate::grouping::STAGE_BYTES + 1),
        Err(ReportError::Grouping(GroupingError::BudgetExceeded { .. }))
    ));
    check!(eq; exclusions.rows.capacity(), 0);
    Ok(())
}

#[test]
fn exclusion_disposition_admission_charges_new_ids_and_preserves_replay() -> TestResult {
    let first: PerformanceId = Id::mint("perf", &["disposition-first"]);
    let second: PerformanceId = Id::mint("perf", &["disposition-second"]);
    let mut exclusions = Exclusions::new();
    exclusions
        .budget
        .charge(crate::grouping::STAGE_BYTES - DISPOSITION_BYTES, 0)?;
    exclusions.track(&first, Disposition::Eligible)?;
    exclusions.track(&first, Disposition::Eligible)?;
    check!(matches!(
        exclusions.track(&second, Disposition::Excluded),
        Err(ReportError::Grouping(GroupingError::BudgetExceeded { .. }))
    ));
    check!(eq; exclusions.dispositions.len(), 1);
    Ok(())
}

#[test]
fn exclusion_dispositions_refuse_mixed_eligible_and_excluded_facts_in_both_orders() -> TestResult {
    let id: PerformanceId = Id::mint("perf", &["mixed-disposition"]);
    for (first, second) in [
        (Disposition::Eligible, Disposition::Excluded),
        (Disposition::Excluded, Disposition::Eligible),
    ] {
        let mut exclusions = Exclusions::new();
        exclusions.track(&id, first)?;
        check!(matches!(
            exclusions.track(&id, second),
            Err(ReportError::Invariant { .. })
        ));
        check!(eq; exclusions.dispositions.len(), 1);
    }
    Ok(())
}
