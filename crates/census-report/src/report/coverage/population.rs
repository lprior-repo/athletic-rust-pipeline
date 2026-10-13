use super::{ReportError, ReportResult, SourcePopulation};
use census_domain::model::{serialized_digest, CoverageRow, CoverageScope};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

pub(super) const STAGES: [&str; 7] = [
    "registered",
    "applicable",
    "attempted",
    "captured",
    "parsed",
    "accepted",
    "withheld",
];

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct StageTotals {
    sums: [usize; 7],
    declared: [usize; 7],
}

impl StageTotals {
    pub(super) fn observe(&mut self, row: &CoverageRow) {
        for (index, key) in STAGES.iter().enumerate() {
            if let Some(value) = row.metrics.get(*key) {
                let value = usize::try_from(*value).map_or(usize::MAX, std::convert::identity);
                self.sums[index] = self.sums[index].saturating_add(value);
                self.declared[index] = self.declared[index].saturating_add(1);
            }
        }
    }

    fn measured(&self, index: usize, rows: usize) -> Option<usize> {
        (rows > 0 && self.declared[index] == rows).then_some(self.sums[index])
    }

    pub(super) fn population(&self, rows: usize) -> SourcePopulation {
        SourcePopulation {
            registered: self.measured(0, rows),
            applicable: self.measured(1, rows),
            attempted: self.measured(2, rows),
            captured: self.measured(3, rows),
            parsed: self.measured(4, rows),
            accepted: self.measured(5, rows),
            withheld: self.measured(6, rows),
        }
    }
}

pub(super) fn jurisdiction_rows(
    rows: &[CoverageRow],
    jurisdiction: UsJurisdiction,
) -> Vec<&CoverageRow> {
    let code = jurisdiction.code();
    rows.iter()
        .filter(|row| row.scope == CoverageScope::Jurisdiction)
        .filter(|row| {
            row.subject == code || row.subject.starts_with(&format!("jurisdiction:{code}:"))
        })
        .collect()
}

pub(super) fn totals<'a>(rows: impl IntoIterator<Item = &'a CoverageRow>) -> StageTotals {
    let mut totals = StageTotals::default();
    for row in rows {
        totals.observe(row);
    }
    totals
}

pub(super) fn publish_coverage_rows(
    store: &Store,
    evidence: &[CoverageRow],
) -> ReportResult<usize> {
    if evidence.is_empty() {
        return Ok(0);
    }
    if evidence.len() > UsJurisdiction::CENSUS_SCOPE.len() {
        return Err(writer_error(
            "coverage evidence exceeds the jurisdiction scope",
        ));
    }
    let mut rows: Vec<&CoverageRow> = evidence.iter().collect();
    rows.sort_unstable_by(|left, right| left.id.cmp(&right.id));
    rows.iter().try_for_each(|row| validate_row(row))?;
    if rows
        .windows(2)
        .any(|pair| matches!(pair, [left, right] if left.id == right.id))
    {
        return Err(writer_error("coverage evidence repeats a jurisdiction"));
    }
    let digest = serialized_digest(&rows).map_err(|error| writer_error(error.to_string()))?;
    let operation = format!("coverage:{}:{digest}", store.derived_generation());
    let mut batch = store.write_batch();
    batch.replace_many(Table::Coverage, &rows)?;
    batch.commit_once(&operation, &digest)?;
    Ok(rows.len())
}

fn validate_row(row: &CoverageRow) -> ReportResult<()> {
    let jurisdiction = UsJurisdiction::CENSUS_SCOPE
        .iter()
        .find(|jurisdiction| jurisdiction.code() == row.subject);
    if row.scope != CoverageScope::Jurisdiction || jurisdiction.is_none() {
        return Err(writer_error(format!(
            "invalid coverage jurisdiction {}",
            row.subject
        )));
    }
    if row.id != format!("jurisdiction:{}", row.subject) {
        return Err(writer_error(format!("unstable coverage id {}", row.id)));
    }
    if row.metrics.is_empty()
        || row
            .metrics
            .keys()
            .any(|key| !STAGES.contains(&key.as_str()))
    {
        return Err(writer_error(format!(
            "unmeasured or unknown coverage stages for {}",
            row.subject
        )));
    }
    Ok(())
}

fn writer_error(detail: impl Into<String>) -> ReportError {
    ReportError::Invariant {
        detail: detail.into(),
    }
}

#[cfg(test)]
mod writer_tests {
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
}
