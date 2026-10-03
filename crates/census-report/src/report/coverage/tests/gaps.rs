use super::*;

#[test]
fn an_empty_store_publishes_every_jurisdiction_with_a_gap() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;

    let report = coverage_of(&store, None)?;

    check!(eq; report.jurisdictions.len(),
    UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1));
    check!(eq; report
        .jurisdictions
        .last()
        .ok_or("missing unplaced row")?
        .jurisdiction
        .code(),
    "UNKNOWN");
    check!(report
        .jurisdictions
        .iter()
        .all(|row| row.athletes == 0 && row.schools == 0 && row.core_share_pct == 0));
    check!(report
        .jurisdictions
        .iter()
        .any(|row| row.jurisdiction.code() == "WI" && row.schools == 0));

    check!(eq; report.gaps.len(),
    UsJurisdiction::CENSUS_SCOPE.len().saturating_add(1));
    check!(report.gaps.iter().all(|gap| gap
        .jurisdiction
        .jurisdiction()
        .is_none_or(|jurisdiction| jurisdiction.is_in_census_scope())));
    check!(report.gaps.iter().all(|gap| {
        gap.class == GapClass::EmptyJurisdiction && gap.count == 0 && gap.unit == "schools"
    }));

    check!(eq; report.read, CoverageTotals::default());
    check!(eq; report.off_cohort_athletes, 0);
    check!(report
        .notes
        .iter()
        .all(|note| !note.contains("outside the census run scope")));
    report.reconcile()?;
    Ok(())
}

#[test]
fn gap_classes_carry_the_count_that_produced_them() -> TestResult {
    let (_dir, store) = fixture_store()?;

    let report = coverage_of(&store, Some(2027))?;

    check!(eq; gap(&report, "WI", GapClass::MissingGraduationEvidence),
    Some(1));
    check!(eq; gap(&report, "WI", GapClass::MissingProfile), Some(1));
    check!(eq; gap(&report, "WI", GapClass::MissingPrSupport), Some(1));
    check!(eq; gap(&report, "WI", GapClass::MissingEventContext), Some(1));
    check!(eq; gap(&report, "WI", GapClass::UnmappedEvent), Some(1));
    let unmapped = report
        .gaps
        .iter()
        .find(|gap| gap.class == GapClass::UnmappedEvent)
        .ok_or("missing unmapped-event gap")?;
    check!(eq; unmapped.unit, "performances");
    check!(eq; gap(&report, "WI", GapClass::MissingPerformanceHistory),
    None);
    let event_context = report
        .gaps
        .iter()
        .find(|gap| gap.class == GapClass::MissingEventContext)
        .ok_or("missing event-context gap")?;
    check!(eq; event_context.unit, "performances");

    check!(eq; row(&report, "IL")?.identity_conflicts, 1);
    check!(eq; gap(&report, "IL", GapClass::ConflictingIdentity), Some(1));
    check!(eq; gap(&report, "IL", GapClass::MissingGraduationEvidence),
    Some(1));

    check!(eq; gap(&report, "MN", GapClass::MissingCoach), Some(1));

    check!(eq; gap(&report, "UNKNOWN", GapClass::MissingSchool), Some(1));
    check!(eq; gap(&report, "UNKNOWN", GapClass::MissingCoach), Some(1));
    check!(eq; gap(&report, "UNKNOWN", GapClass::UnknownJurisdiction),
    Some(1));
    check!(eq; gap(&report, "UNKNOWN", GapClass::MissingProfile), Some(1));
    check!(eq; gap(&report, "UNKNOWN", GapClass::MissingEventContext),
    Some(1));

    check!(eq; gap(&report, "MN", GapClass::EmptyJurisdiction), None);
    check!(eq; gap(&report, "WY", GapClass::EmptyJurisdiction), Some(0));
    Ok(())
}
