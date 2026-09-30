use anyhow::{bail, Context, Result};
use census_crawl::directory::{ReadOutcome, RowIssue};
use census_crawl::{nces, private_assoc, state_ed, CrawlError, CrawlResult};
use census_domain::school_directory::{
    Baseline, ScheduleLedger, ScheduleSource, SchoolDirectoryEntry,
};
use std::path::Path;

use super::export;
use super::report::{LaneReport, LedgerRow};
use super::SchoolAddressArgs;

pub(super) struct LaneRead {
    pub(super) source: ScheduleSource,
    pub(super) report: LaneReport,
    pub(super) entries: Vec<SchoolDirectoryEntry>,
}

pub(super) fn lanes(args: &SchoolAddressArgs) -> Result<Vec<LaneRead>> {
    let mut lanes = Vec::new();
    if let Some(path) = &args.ccd {
        lanes.push(read_lane(path, ScheduleSource::Ccd, nces::parse_ccd)?);
    }
    if let Some(path) = &args.pss {
        lanes.push(read_lane(path, ScheduleSource::Pss, nces::parse_pss)?);
    }
    for path in &args.state_ed_index {
        lanes.push(read_lane(
            path,
            ScheduleSource::StateEducationAgency,
            state_ed::parse_index,
        )?);
    }
    for path in &args.state_ed_profile {
        lanes.push(read_lane(
            path,
            ScheduleSource::StateEducationAgency,
            state_ed::parse_profile,
        )?);
    }
    for path in &args.state_ed_tabular {
        lanes.push(read_lane(
            path,
            ScheduleSource::StateEducationAgency,
            state_ed::parse_tabular,
        )?);
    }
    for path in &args.associations {
        lanes.push(read_lane(
            path,
            ScheduleSource::PrivateAssociation,
            private_assoc::parse_listing,
        )?);
    }
    if lanes.is_empty() {
        bail!(
            "no input artifact was named: pass at least one of --ccd, --pss, --state-ed-index, \
             --state-ed-profile, --state-ed-tabular or --associations"
        );
    }
    Ok(lanes)
}

pub(super) fn read_baseline(path: &Path) -> Result<Option<Baseline>> {
    if !path
        .try_exists()
        .with_context(|| format!("reading the baseline {}", path.display()))?
    {
        return Ok(None);
    }
    let bytes = super::manifest::verified_artifact(path)?;
    Ok(Some(serde_json::from_slice(&bytes).with_context(|| {
        format!("parsing the baseline {}", path.display())
    })?))
}

pub(super) fn read_ledger(path: &Path) -> Result<ScheduleLedger> {
    if !path
        .try_exists()
        .with_context(|| format!("reading the update ledger {}", path.display()))?
    {
        return Ok(ScheduleLedger::new());
    }
    let bytes = super::manifest::verified_artifact(path)?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing the update ledger {}", path.display()))
}

fn read_lane(
    path: &Path,
    source: ScheduleSource,
    reader: fn(&str) -> CrawlResult<ReadOutcome>,
) -> Result<LaneRead> {
    let text = std::fs::read_to_string(path).map_err(|source| CrawlError::DirectoryArtifact {
        path: path.to_path_buf(),
        detail: format!("the file could not be read: {source}"),
    })?;
    let sha256 = export::sha256_hex(text.as_bytes());
    let outcome = reader(&text).map_err(|error| match error {
        CrawlError::Invariant { detail } => CrawlError::DirectoryArtifact {
            path: path.to_path_buf(),
            detail,
        },
        other => other,
    })?;
    let counts = outcome.counts();
    let report = LaneReport {
        source: source.label().to_string(),
        path: path.display().to_string(),
        sha256,
        entries: counts.entries,
        skipped: counts.skipped,
        notes: counts.notes,
        skipped_rows: rows(outcome.skipped()),
        note_rows: rows(outcome.notes()),
    };
    Ok(LaneRead {
        source,
        report,
        entries: outcome.entries().to_vec(),
    })
}

fn rows(issues: &[RowIssue]) -> Vec<LedgerRow> {
    issues
        .iter()
        .map(|issue| LedgerRow {
            line: issue.line,
            field: issue.field.to_string(),
            detail: issue.detail.clone(),
        })
        .collect()
}
