use super::{ExportCoverage, SourceManifest};
use crate::{
    domain::identity::{EvidenceDigest, SourceRowKey},
    store::ArtifactStore,
};
use anyhow::{bail, Context, Result};
use serde::de::DeserializeOwned;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub(super) struct ReportIndex {
    pub(super) reports: BTreeMap<String, EvidenceDigest>,
    pub(super) completed: u64,
}

pub(super) fn validate_progress(
    progress: &super::RunProgress,
    pages: &[EvidenceDigest],
    manifest: &SourceManifest,
) -> Result<()> {
    if manifest.ingestion_revision != super::super::import::INGESTION_REVISION {
        bail!("source manifest ingestion revision is unsupported");
    }
    let page_count = usize::try_from(progress.pages).context("page count conversion overflow")?;
    if pages.len() != page_count {
        bail!("sealed page count differs from run progress");
    }
    if progress.coverage.selected > manifest.stats.actual_data_rows
        || progress.coverage.completed > progress.coverage.selected
    {
        bail!("run coverage exceeds source manifest");
    }
    let counters = progress
        .coverage
        .deterministic
        .checked_add(progress.coverage.local_review)
        .and_then(|value| value.checked_add(progress.coverage.no_match))
        .and_then(|value| value.checked_add(progress.coverage.review_required))
        .context("coverage count overflow")?;
    if counters != progress.coverage.completed {
        bail!("run coverage counters do not sum to completed rows");
    }
    if progress.complete
        && (!progress.pending_rows.is_empty()
            || progress.coverage.completed != progress.coverage.selected)
    {
        bail!("complete run has unfinished progress");
    }
    let mut seen = BTreeSet::new();
    pages.iter().try_for_each(|digest| {
        if seen.insert(digest.as_str().to_owned()) {
            Ok(())
        } else {
            bail!("duplicate sealed result page digest")
        }
    })
}

pub(super) fn build_report_index(
    store: &ArtifactStore,
    progress: &super::RunProgress,
    pages: &[EvidenceDigest],
    manifest: &SourceManifest,
) -> Result<ReportIndex> {
    let mut reports = BTreeMap::new();
    pages.iter().try_for_each(|digest| {
        let page: super::RunPage =
            load_json(store, digest).context("loading sealed result page")?;
        if page.rows.is_empty() || page.rows.len() > super::RESULT_PAGE_ROWS {
            bail!("sealed result page has invalid row count");
        }
        page.rows.iter().try_for_each(|row| {
            add_report(
                store,
                &mut reports,
                row.source.clone(),
                row.report.clone(),
                progress,
                manifest,
            )
        })
    })?;
    progress.pending_rows.iter().try_for_each(|row| {
        add_report(
            store,
            &mut reports,
            row.source.clone(),
            row.report.clone(),
            progress,
            manifest,
        )
    })?;
    let completed = u64::try_from(reports.len()).context("report count conversion overflow")?;
    if completed != progress.coverage.completed {
        bail!("report count differs from run coverage");
    }
    Ok(ReportIndex { reports, completed })
}

fn add_report(
    store: &ArtifactStore,
    reports: &mut BTreeMap<String, EvidenceDigest>,
    source: SourceRowKey,
    digest: EvidenceDigest,
    progress: &super::RunProgress,
    manifest: &SourceManifest,
) -> Result<()> {
    let key = source.as_str().to_owned();
    if reports.insert(key.clone(), digest.clone()).is_some() {
        bail!("duplicate report source key {key:?}");
    }
    let report: super::RowReport = load_json(store, &digest).context("loading row report")?;
    validate_report(&report, &source, progress, manifest)?;
    SourceRowKey::parse(&key).context("parsing report source key")?;
    Ok(())
}

fn validate_report(
    report: &super::RowReport,
    source: &SourceRowKey,
    progress: &super::RunProgress,
    manifest: &SourceManifest,
) -> Result<()> {
    let bound = report.revision == super::super::row_protocol::ROW_PROTOCOL_REVISION
        && report.job.snapshot == progress.request.snapshot
        && report.job.workbook == manifest.workbook
        && report.job.source.as_str() == source.as_str();
    if !bound {
        bail!("row report does not bind run source");
    }
    Ok(())
}

pub(super) fn validate_export_coverage(
    actual: &ExportCoverage,
    expected: &super::super::run_protocol::Coverage,
    indexed: u64,
    source_rows: u64,
) -> Result<()> {
    if actual.source_rows != source_rows
        || actual.completed_rows != indexed
        || actual.completed_rows != expected.completed
    {
        bail!("export coverage differs from sealed run reports");
    }
    if actual.completed_rows.checked_add(actual.pending_rows) != Some(actual.source_rows) {
        bail!("export rows are neither completed nor pending");
    }
    let accepted = expected
        .deterministic
        .checked_add(expected.local_review)
        .context("accepted coverage overflow")?;
    if actual.accepted_rows != accepted
        || actual.no_match_rows != expected.no_match
        || actual.review_rows != expected.review_required
    {
        bail!("export resolution coverage differs from run progress");
    }
    Ok(())
}

pub(super) fn load_json<T: DeserializeOwned>(
    store: &ArtifactStore,
    digest: &EvidenceDigest,
) -> Result<T> {
    let bytes = store.get_bytes(digest)?;
    serde_json::from_slice(&bytes).context("decoding stored JSON artifact")
}

pub(super) fn count_resolution(
    coverage: &mut ExportCoverage,
    digest: &EvidenceDigest,
    store: &ArtifactStore,
) -> Result<()> {
    let report: super::RowReport = load_json(store, digest)?;
    let counter = match report.resolution {
        super::RowResolution::Accepted { .. } => &mut coverage.accepted_rows,
        super::RowResolution::CompleteSearchNoMatch => &mut coverage.no_match_rows,
        super::RowResolution::ReviewRequired => &mut coverage.review_rows,
    };
    *counter = counter
        .checked_add(1)
        .context("resolution count overflow")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_export_coverage;
    use crate::runtime::{export::ExportCoverage, run_protocol::Coverage};

    fn exported(
        completed: u64,
        pending: u64,
        accepted: u64,
        no_match: u64,
        review: u64,
    ) -> ExportCoverage {
        ExportCoverage {
            source_rows: completed + pending,
            selected_rows: completed + pending,
            completed_rows: completed,
            pending_rows: pending,
            accepted_rows: accepted,
            no_match_rows: no_match,
            review_rows: review,
        }
    }

    fn sealed(
        completed: u64,
        deterministic: u64,
        local_review: u64,
        no_match: u64,
        review: u64,
    ) -> Coverage {
        Coverage {
            selected: completed,
            completed,
            deterministic,
            local_review,
            no_match,
            review_required: review,
        }
    }

    fn rejection(actual: &ExportCoverage, expected: &Coverage, indexed: u64) -> String {
        validate_export_coverage(actual, expected, indexed, actual.source_rows)
            .expect_err("inconsistent coverage must be rejected")
            .to_string()
    }

    #[test]
    fn coverage_reconciling_with_the_sealed_run_is_accepted() {
        let actual = exported(90, 10, 84, 3, 3);
        let expected = sealed(90, 80, 4, 3, 3);
        assert!(validate_export_coverage(&actual, &expected, 90, 100).is_ok());
    }

    #[test]
    fn an_export_that_drops_pending_rows_is_rejected() {
        let mut actual = exported(90, 9, 84, 3, 3);
        actual.source_rows = 100;
        let expected = sealed(90, 80, 4, 3, 3);
        assert!(rejection(&actual, &expected, 90).contains("neither completed nor pending"));
    }

    #[test]
    fn resolutions_that_do_not_reconcile_with_run_progress_are_rejected() {
        let actual = exported(90, 10, 83, 3, 3);
        let expected = sealed(90, 80, 4, 3, 3);
        assert!(rejection(&actual, &expected, 90).contains("resolution coverage differs"));
    }

    #[test]
    fn totals_that_differ_from_sealed_reports_are_rejected() {
        let actual = exported(90, 10, 84, 3, 3);
        let expected = sealed(89, 79, 4, 3, 3);
        assert!(rejection(&actual, &expected, 89).contains("differs from sealed run reports"));
    }
}
