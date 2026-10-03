use super::*;

#[test]
fn jurisdiction_rows_sum_to_the_store_totals() -> TestResult {
    let (_dir, store) = fixture_store()?;

    let report = coverage_of(&store, Some(2027))?;
    let published = report.published_totals();

    let stored_athletes = store.scan::<CanonicalAthlete>(Table::Athletes)?.len();
    check!(eq; stored_athletes, 7);
    check!(eq; report.off_cohort_athletes, 2);
    check!(eq; published
        .athletes
        .saturating_add(report.off_cohort_athletes),
    stored_athletes);
    check!(eq; published.schools,
    store.scan::<CanonicalSchool>(Table::Schools)?.len());
    check!(eq; published.coaches,
    store.scan::<CanonicalCoach>(Table::Coaches)?.len());
    check!(eq; published.meets,
    store.scan::<CanonicalMeet>(Table::Meets)?.len());
    check!(eq; published.performances,
    store
        .scan::<CanonicalPerformance>(Table::Performances)?
        .len());

    check!(eq; published, report.read);
    report.reconcile()?;
    let lines = report.reconciliation_lines();
    check!(eq; lines.len(), 4);
    check!(lines
        .iter()
        .any(|line| line.contains("coverage reconciliation: matches")));
    check!(
        lines.iter().any(
            |line| line.contains("read: schools=4 athletes=5 coaches=2 meets=1 performances=4")
        ),
        "{lines:?}"
    );
    Ok(())
}

#[test]
fn reconcile_refuses_a_report_whose_rows_lost_or_invented_a_row() -> TestResult {
    let (_dir, store) = fixture_store()?;
    let report = coverage_of(&store, Some(2027))?;

    let mut lost = report.clone();
    lost.jurisdictions.pop();
    let error = match lost.reconcile() {
        Err(error) => error.to_string(),
        Ok(()) => return Err("lost jurisdiction reconciled".into()),
    };
    check!(error.contains("coverage reconciliation failed"), "{error}");
    check!(
        error.contains("rows publish schools=4 athletes=4"),
        "{error}"
    );

    let mut invented = report.clone();
    invented
        .jurisdictions
        .push(row(&report, "UNKNOWN")?.clone());
    let error = match invented.reconcile() {
        Err(error) => error.to_string(),
        Ok(()) => return Err("invented jurisdiction reconciled".into()),
    };
    check!(error.contains("coverage reconciliation failed"), "{error}");
    check!(
        error.contains("rows publish schools=4 athletes=6"),
        "{error}"
    );
    Ok(())
}

#[test]
fn reconcile_refuses_a_read_count_the_rows_do_not_publish() -> TestResult {
    let (_dir, store) = fixture_store()?;
    let mut report = coverage_of(&store, Some(2027))?;

    report.read.athletes = report.read.athletes.saturating_add(1);
    let error = match report.reconcile() {
        Err(error) => error,
        Ok(()) => return Err("unpublished read count reconciled".into()),
    };
    let text = error.to_string();
    check!(text.contains("coverage reconciliation failed"), "{text}");
    check!(text.contains("read schools=4 athletes=6"), "{text}");
    check!(text.contains("rows publish schools=4 athletes=5"), "{text}");
    Ok(())
}
