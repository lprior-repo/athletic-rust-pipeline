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
