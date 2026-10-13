use std::sync::Arc;

use census_domain::model::{CoverageRow, CoverageScope};
use census_domain::UsJurisdiction;
use census_report::report::CoverageReport;
use restate_sdk::prelude::*;

use super::super::publish::Jobs;
use super::super::wire::NationalReport;
use super::super::{blocking, job_error};

#[tracing::instrument(skip_all)]
pub(super) async fn publish(
    jobs: &Jobs,
    ctx: &WorkflowContext<'_>,
    targets: &[(UsJurisdiction, String)],
    report: &NationalReport,
) -> Result<(), HandlerError> {
    let rows = coverage_rows(targets, report);
    let store = Arc::clone(jobs.store());
    let region = Arc::clone(jobs.region());
    let permit = jobs.permit_control().await?;
    ctx.run(move || async move {
        blocking(region, move || {
            let _permit = permit;
            CoverageReport::publish_coverage_rows(&store, &rows).map(|_| ())
        })
        .await
        .map_err(job_error)
    })
    .retry_policy(RunRetryPolicy::new().max_attempts(1))
    .await
    .map_err(HandlerError::from)
}

fn coverage_rows(
    targets: &[(UsJurisdiction, String)],
    report: &NationalReport,
) -> Vec<CoverageRow> {
    targets
        .iter()
        .map(|(jurisdiction, _)| {
            let row = CoverageRow::new(CoverageScope::Jurisdiction, jurisdiction.code())
                .with("registered", 1)
                .with("applicable", 1);
            if report
                .jurisdictions
                .iter()
                .any(|summary| summary.jurisdiction == *jurisdiction)
                || report
                    .failures
                    .iter()
                    .any(|failure| failure.jurisdiction == *jurisdiction)
            {
                row.with("attempted", 1)
            } else {
                row
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::SchoolYear;
    use census_reconcile::identity::Revision;
    use census_store::{Store, Table};

    use super::super::super::wire::{JurisdictionSummary, NationalFailure};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn report() -> Result<NationalReport, Box<dyn std::error::Error>> {
        Ok(NationalReport {
            season: SchoolYear::new(2026).ok_or("invalid test season")?,
            revision: Revision(1),
            jurisdictions: vec![JurisdictionSummary {
                jurisdiction: UsJurisdiction::Wisconsin,
                identity: "jurisdiction:WI:2026-27:1".to_string(),
                rosters_total: 12,
                rosters_committed: 8,
                rosters_remaining: 4,
                rosters_skipped: 2,
                blocked: true,
                athletes: 90,
                class_of_2027: 30,
            }],
            failures: vec![NationalFailure {
                jurisdiction: UsJurisdiction::Iowa,
                identity: "jurisdiction:IA:2026-27:1".to_string(),
                error: "source refused".to_string(),
            }],
            owed: Vec::new(),
            rosters_total: 12,
            athletes_total: 90,
            class_of_2027_total: 30,
            school_address: None,
            today: "2026-10-12".to_string(),
        })
    }

    fn targets() -> Vec<(UsJurisdiction, String)> {
        [
            UsJurisdiction::Wisconsin,
            UsJurisdiction::Iowa,
            UsJurisdiction::Illinois,
        ]
        .into_iter()
        .map(|jurisdiction| (jurisdiction, jurisdiction.code().to_string()))
        .collect()
    }

    #[test]
    fn coverage_publication_preserves_answered_failed_and_unmeasured_sweeps() -> TestResult {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let evidence = coverage_rows(&targets(), &report()?);
        check!(eq; CoverageReport::publish_coverage_rows(&store, &evidence)?, 3);
        drop(store);
        let store = Store::open(directory.path())?;
        let rows: Vec<CoverageRow> = store.scan(Table::Coverage)?;
        check!(eq; rows.len(), 3);
        for row in &rows {
            check!(eq; row.id, format!("jurisdiction:{}", row.subject));
            check!(eq; row.metrics.get("registered"), Some(&1));
            check!(eq; row.metrics.get("applicable"), Some(&1));
            let attempted = if row.subject == "IL" { None } else { Some(&1) };
            check!(eq; row.metrics.get("attempted"), attempted);
            let count = if row.subject == "IL" { 2 } else { 3 };
            check!(eq; row.metrics.len(), count);
            for stage in ["captured", "parsed", "accepted", "withheld"] {
                check!(eq; row.metrics.get(stage), None);
            }
        }
        Ok(())
    }

    #[test]
    fn coverage_failed_sweep_does_not_use_a_foreign_jurisdictions_outcome() -> TestResult {
        let targets = vec![(UsJurisdiction::Minnesota, "MN".to_string())];
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let evidence = coverage_rows(&targets, &report()?);
        check!(eq; CoverageReport::publish_coverage_rows(&store, &evidence)?, 1);
        let rows: Vec<CoverageRow> = store.scan(Table::Coverage)?;
        check!(eq; rows, vec![CoverageRow::new(CoverageScope::Jurisdiction, "MN")
            .with("registered", 1).with("applicable", 1)]);
        Ok(())
    }
}
