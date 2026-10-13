use super::*;
use census_domain::model::{CoverageRow, CoverageScope};

fn row(scope: CoverageScope, subject: &str, metrics: &[(&str, u64)]) -> CoverageRow {
    let mut row = CoverageRow::new(scope, subject);
    for (key, value) in metrics {
        row = row.with(key, *value);
    }
    row
}

#[test]
fn a_stage_declared_by_every_row_is_measured_and_a_partial_stage_is_unmeasured() -> TestResult {
    let rows = vec![
        row(
            CoverageScope::Jurisdiction,
            "WI",
            &[("registered", 4), ("accepted", 3)],
        ),
        row(
            CoverageScope::Jurisdiction,
            "MN",
            &[("registered", 6), ("accepted", 1)],
        ),
    ];
    let totals = population::totals(&rows);
    let population = totals.population(rows.len());
    check!(eq; population.registered, Some(10));
    check!(eq; population.accepted, Some(4));
    check!(eq; population.attempted, None);
    Ok(())
}

#[test]
fn an_empty_plane_is_unmeasured_rather_than_zero() -> TestResult {
    let rows: Vec<CoverageRow> = Vec::new();
    let population = population::totals(&rows).population(rows.len());
    check!(eq; population.accepted, None);
    check!(eq; population.withheld, None);
    Ok(())
}

#[test]
fn jurisdiction_rows_match_bare_codes_and_canonical_identities() -> TestResult {
    let rows = vec![
        row(CoverageScope::Jurisdiction, "WI", &[]),
        row(
            CoverageScope::Jurisdiction,
            "jurisdiction:WI:2026-27:1",
            &[],
        ),
        row(CoverageScope::Jurisdiction, "MN", &[]),
        row(CoverageScope::Source, "milesplit_rosters", &[]),
    ];
    let matched = population::jurisdiction_rows(&rows, UsJurisdiction::Wisconsin);
    check!(eq; matched.len(), 2);
    Ok(())
}

#[test]
fn scoped_stage_totals_reject_cross_jurisdiction_and_same_jurisdiction_overflow() -> TestResult {
    for subjects in [["WI", "MN"], ["WI", "jurisdiction:WI:2026-27:1"]] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        for subject in subjects {
            store.replace(
                Table::Coverage,
                &row(
                    CoverageScope::Jurisdiction,
                    subject,
                    &[("registered", u64::MAX)],
                ),
            )?;
        }
        let dataset = crate::export::ExportDataset::load(&store)?;
        let derivation = crate::report::Derivation::of(
            &dataset,
            Scope::AllSources,
            None,
            &[UsJurisdiction::Wisconsin, UsJurisdiction::Minnesota],
        );
        let error = coverage_report_for_derivation(&derivation)
            .err()
            .ok_or("overflowing stage total was accepted")?;
        check!(error
            .to_string()
            .contains("coverage source stage total exceeds representable capacity"));
    }
    Ok(())
}

#[test]
fn scoped_stage_totals_accept_exact_capacity_without_saturation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let maximum = u64::try_from(usize::MAX)?;
    store.replace(
        Table::Coverage,
        &row(
            CoverageScope::Jurisdiction,
            "WI",
            &[("registered", maximum.checked_sub(1).ok_or("capacity")?)],
        ),
    )?;
    store.replace(
        Table::Coverage,
        &row(CoverageScope::Jurisdiction, "MN", &[("registered", 1)]),
    )?;
    let dataset = crate::export::ExportDataset::load(&store)?;
    let derivation = crate::report::Derivation::of(
        &dataset,
        Scope::AllSources,
        None,
        &[UsJurisdiction::Wisconsin, UsJurisdiction::Minnesota],
    );
    let report = coverage_report_for_derivation(&derivation)?;
    check!(eq; report.source_population.registered, Some(usize::MAX));
    Ok(())
}

#[test]
fn scoped_stages_distinguish_measured_zero_and_partial_measurement() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.replace(
        Table::Coverage,
        &row(
            CoverageScope::Jurisdiction,
            "WI",
            &[("registered", 7), ("attempted", 4), ("captured", 0)],
        ),
    )?;
    store.replace(
        Table::Coverage,
        &row(
            CoverageScope::Jurisdiction,
            "MN",
            &[("registered", 9), ("attempted", 5), ("captured", 0)],
        ),
    )?;
    let dataset = crate::export::ExportDataset::load(&store)?;
    let selected = [UsJurisdiction::Wisconsin, UsJurisdiction::Minnesota];
    let derivation = crate::report::Derivation::of(&dataset, Scope::AllSources, None, &selected);
    let report = coverage_report_for_derivation(&derivation)?;
    check!(eq; report.source_population.registered, Some(16));
    check!(eq; report.source_population.attempted, Some(9));
    check!(eq; report.source_population.captured, Some(0));
    check!(eq; report.source_population.parsed, None);
    check!(report
        .jurisdictions
        .iter()
        .any(|row| row.jurisdiction == JurisdictionBucket::Unplaced));
    store.replace(
        Table::Coverage,
        &row(
            CoverageScope::Jurisdiction,
            "MN",
            &[("registered", 9), ("captured", 0)],
        ),
    )?;
    let dataset = crate::export::ExportDataset::load(&store)?;
    let derivation = crate::report::Derivation::of(&dataset, Scope::AllSources, None, &selected);
    let report = coverage_report_for_derivation(&derivation)?;
    check!(eq; report.source_population.registered, Some(16));
    check!(eq; report.source_population.attempted, None);
    check!(eq; report.source_population.captured, Some(0));
    let unsupported =
        crate::report::Derivation::of(&dataset, Scope::AllSources, None, &[UsJurisdiction::Alaska]);
    let report = coverage_report_for_derivation(&unsupported)?;
    check!(eq; report.jurisdictions.len(), 1);
    check!(eq; report.source_population, SourcePopulation::default());
    Ok(())
}
