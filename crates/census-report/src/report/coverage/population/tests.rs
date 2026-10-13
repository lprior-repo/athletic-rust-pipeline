use super::*;
use crate::report::CoverageReport;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn coverage_writer_rows_reconcile_through_the_public_report() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let rows: Vec<CoverageRow> = UsJurisdiction::CENSUS_SCOPE
        .iter()
        .map(|jurisdiction| {
            CoverageRow::new(CoverageScope::Jurisdiction, jurisdiction.code())
                .with("registered", 1)
                .with("applicable", 1)
                .with("attempted", 1)
        })
        .collect();
    check!(eq; CoverageReport::publish_coverage_rows(&store, &rows)?, 49);
    let dataset = crate::export::ExportDataset::load(&store)?;
    let report = crate::report::coverage_report(&dataset, None)?;
    check!(eq; report.source_population.registered, Some(49));
    check!(eq; report.source_population.applicable, Some(49));
    check!(eq; report.source_population.attempted, Some(49));
    check!(eq; report.source_population.captured, None);
    check!(eq; report.source_population.parsed, None);
    check!(eq; report.source_population.accepted, None);
    check!(eq; report.source_population.withheld, None);
    for jurisdiction in &UsJurisdiction::CENSUS_SCOPE {
        let row = report
            .jurisdictions
            .iter()
            .find(|row| row.jurisdiction == (*jurisdiction).into())
            .ok_or("report lost a measured jurisdiction")?;
        check!(eq; row.source_population.registered, Some(1));
        check!(eq; row.source_population.applicable, Some(1));
        check!(eq; row.source_population.attempted, Some(1));
        check!(eq; row.source_population.captured, None);
        check!(eq; row.source_population.parsed, None);
        check!(eq; row.source_population.accepted, None);
        check!(eq; row.source_population.withheld, None);
    }
    report.reconcile()?;
    Ok(())
}

#[test]
fn coverage_writer_replay_is_durable_and_has_no_new_effects() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let rows = [
        CoverageRow::new(CoverageScope::Jurisdiction, "WI")
            .with("registered", 1)
            .with("applicable", 1)
            .with("attempted", 1),
        CoverageRow::new(CoverageScope::Jurisdiction, "IA")
            .with("registered", 1)
            .with("applicable", 1),
    ];
    check!(eq; CoverageReport::publish_coverage_rows(&store, &rows)?, 2);
    let generation = store.evidence_generation();
    let receipts = store.receipt_count()?;
    drop(store);
    let store = Store::open(directory.path())?;
    let reversed: Vec<CoverageRow> = rows.into_iter().rev().collect();
    check!(eq; CoverageReport::publish_coverage_rows(&store, &reversed)?, 2);
    check!(eq; store.evidence_generation(), generation);
    check!(eq; store.receipt_count()?, receipts);
    let stored: Vec<CoverageRow> = store.scan(Table::Coverage)?;
    check!(eq; stored, reversed);
    Ok(())
}

#[test]
fn coverage_writer_updates_a_subject_without_duplicate_rows() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let registered = CoverageRow::new(CoverageScope::Jurisdiction, "WI").with("registered", 1);
    let attempted = registered.clone().with("attempted", 1);
    check!(eq; CoverageReport::publish_coverage_rows(&store, &[registered])?, 1);
    check!(eq; CoverageReport::publish_coverage_rows(&store, std::slice::from_ref(&attempted))?, 1);
    check!(eq; store.scan::<CoverageRow>(Table::Coverage)?, vec![attempted]);
    Ok(())
}

#[test]
fn coverage_writer_empty_evidence_has_no_effects() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let generation = store.evidence_generation();
    let receipts = store.receipt_count()?;
    check!(eq; CoverageReport::publish_coverage_rows(&store, &[])?, 0);
    check!(eq; store.evidence_generation(), generation);
    check!(eq; store.receipt_count()?, receipts);
    check!(eq; store.scan::<CoverageRow>(Table::Coverage)?, Vec::<CoverageRow>::new());
    Ok(())
}

#[test]
fn coverage_writer_retains_measured_zero_for_each_stage() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let row = STAGES.iter().fold(
        CoverageRow::new(CoverageScope::Jurisdiction, "WI"),
        |row, stage| row.with(stage, 0),
    );
    check!(eq; CoverageReport::publish_coverage_rows(&store, std::slice::from_ref(&row))?, 1);
    check!(eq; store.scan::<CoverageRow>(Table::Coverage)?, vec![row]);
    Ok(())
}

#[test]
fn coverage_writer_refuses_invalid_evidence_before_any_write() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let valid = CoverageRow::new(CoverageScope::Jurisdiction, "WI").with("registered", 1);
    let mut unstable = valid.clone();
    unstable.id = "random-id".to_string();
    let cases = [
        (
            vec![valid.clone(), valid.clone()],
            "coverage evidence repeats a jurisdiction",
        ),
        (
            vec![valid.clone(); 50],
            "coverage evidence exceeds the jurisdiction scope",
        ),
        (
            vec![CoverageRow::new(CoverageScope::Source, "WI").with("registered", 1)],
            "invalid coverage jurisdiction WI",
        ),
        (
            vec![CoverageRow::new(CoverageScope::Jurisdiction, "AK").with("registered", 1)],
            "invalid coverage jurisdiction AK",
        ),
        (vec![unstable], "unstable coverage id random-id"),
        (
            vec![CoverageRow::new(CoverageScope::Jurisdiction, "WI")],
            "unmeasured or unknown coverage stages for WI",
        ),
        (
            vec![valid.with("schools", 12)],
            "unmeasured or unknown coverage stages for WI",
        ),
    ];
    let generation = store.evidence_generation();
    for (rows, detail) in cases {
        let Err(ReportError::Invariant { detail: observed }) =
            CoverageReport::publish_coverage_rows(&store, &rows)
        else {
            return Err(format!("coverage accepted invalid evidence: {detail}").into());
        };
        check!(eq; observed, detail);
        check!(eq; store.evidence_generation(), generation);
        check!(eq; store.receipt_count()?, 0);
        check!(eq; store.scan::<CoverageRow>(Table::Coverage)?, Vec::<CoverageRow>::new());
    }
    Ok(())
}
