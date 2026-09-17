use super::{
    acquisition::ProfileAcquisition,
    import::SourceManifest,
    row_protocol::{AcceptanceMethod, CandidateCoverage, RowReport, RowResolution},
    run_protocol::{RunPage, RunProgress, RESULT_PAGE_ROWS},
};
use crate::{
    domain::identity::EvidenceDigest,
    store::ArtifactStore,
    workbook_export::{ExportStats, WorkbookExport},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

mod details;
mod index;
mod projection;
mod publish;
mod stream;

pub const EXPORT_HEADERS: &[&str] = &[
    "native.source_key",
    "native.terminal_status",
    "native.athlete_id",
    "native.profile_url",
    "native.acceptance_method",
    "native.row_report_digest",
    "native.assessment_digest",
    "native.candidate_count",
    "native.eligibility_basis",
    "native.evidence_strength",
    "native.pr_summary",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompletenessState {
    Complete,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportCoverage {
    pub source_rows: u64,
    pub selected_rows: u64,
    pub completed_rows: u64,
    pub pending_rows: u64,
    pub accepted_rows: u64,
    pub no_match_rows: u64,
    pub review_rows: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportReport {
    pub workbook: String,
    pub xlsx_path: PathBuf,
    pub xlsx_sha256: String,
    pub detail_path: PathBuf,
    pub detail_sha256: String,
    pub stats: ExportStats,
    pub coverage: ExportCoverage,
    pub completeness: CompletenessState,
}

pub fn export_to(
    store: &ArtifactStore,
    progress: &RunProgress,
    page_digests: &[EvidenceDigest],
    destination: &Path,
) -> Result<ExportReport> {
    let manifest: SourceManifest = index::load_json(store, &progress.request.manifest)
        .context("loading export source manifest")?;
    index::validate_progress(progress, page_digests, &manifest)?;
    let reports = index::build_report_index(store, progress, page_digests, &manifest)?;
    let detail_path = publish::detail_path(destination);
    publish::reject_detail_destination(destination, &detail_path, &manifest)?;
    let headers = EXPORT_HEADERS
        .iter()
        .map(|header| (*header).to_owned())
        .collect::<Vec<_>>();
    let mut workbook = WorkbookExport::new(&manifest, &headers)?;
    let parent = publish::destination_parent(destination)?;
    let mut temporary =
        NamedTempFile::new_in(&parent).context("creating detail artifact temporary")?;
    let mut coverage = initial_coverage(progress);
    write_artifacts(
        store,
        &manifest,
        &reports,
        &mut workbook,
        &mut temporary,
        &mut coverage,
    )?;
    index::validate_export_coverage(
        &coverage,
        &progress.coverage,
        reports.completed,
        manifest.stats.actual_data_rows,
    )?;
    temporary
        .as_file()
        .sync_all()
        .context("syncing detail artifact")?;
    let detail_sha256 = publish::sha256_file(temporary.path())?;
    publish::persist_detail(temporary, &detail_path, parent)?;
    let stats = workbook
        .finish(destination)
        .context("publishing workbook export")?;
    let xlsx_sha256 = publish::sha256_file(destination)?;
    let completeness = match coverage.pending_rows {
        0 => CompletenessState::Complete,
        _ => CompletenessState::Partial,
    };
    Ok(ExportReport {
        workbook: manifest.workbook.as_str().to_owned(),
        xlsx_path: destination.to_owned(),
        xlsx_sha256,
        detail_path,
        detail_sha256,
        stats,
        coverage,
        completeness,
    })
}

fn initial_coverage(progress: &RunProgress) -> ExportCoverage {
    ExportCoverage {
        source_rows: 0,
        selected_rows: progress.coverage.selected,
        completed_rows: 0,
        pending_rows: 0,
        accepted_rows: 0,
        no_match_rows: 0,
        review_rows: 0,
    }
}

fn write_artifacts(
    store: &ArtifactStore,
    manifest: &SourceManifest,
    reports: &index::ReportIndex,
    workbook: &mut WorkbookExport,
    temporary: &mut NamedTempFile,
    coverage: &mut ExportCoverage,
) -> Result<()> {
    let mut detail = std::io::BufWriter::new(temporary.as_file_mut());
    stream::sources(
        store,
        manifest,
        &reports.reports,
        workbook,
        &mut detail,
        coverage,
    )?;
    detail.flush().context("flushing detail artifact")
}
