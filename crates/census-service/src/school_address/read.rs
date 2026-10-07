use anyhow::{anyhow, bail, Context, Result};
use census_crawl::directory::{ReadOutcome, RowIssue};
use census_crawl::{nces, private_assoc, state_ed, tssaa, CrawlError, CrawlResult};
use census_domain::school_directory::{
    Baseline, IdentifiedKey, ScheduleLedger, ScheduleSource, SchoolDirectoryEntry,
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
    args.association_directory
        .iter()
        .try_for_each(|pair| -> Result<()> {
            lanes.push(read_association_directory(pair)?);
            Ok(())
        })?;
    if lanes.is_empty() {
        bail!(
            "no input artifact was named: pass at least one of --ccd, --pss, --state-ed-index, \
             --state-ed-profile, --state-ed-tabular, --associations or --association-directory"
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

fn read_association_directory(pair: &str) -> Result<LaneRead> {
    let (slug, path) = pair.split_once('=').ok_or_else(|| {
        anyhow!("--association-directory requires SLUG=PATH, got {pair:?}; admitted slugs: tssaa")
    })?;
    if slug.is_empty()
        || slug.len() > 64
        || !slug.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
    {
        bail!(
            "--association-directory slug {slug:?} must be 1–64 bytes of [a-z0-9_-]; admitted slugs: tssaa"
        );
    }
    let reader = match slug {
        "tssaa" => tssaa::parse_school_list,
        _ => bail!("--association-directory unknown slug {slug:?}; admitted slugs: tssaa"),
    };
    read_lane_with_token(
        Path::new(path),
        ScheduleSource::AthleticAssociation,
        &format!("association:{slug}"),
        reader,
    )
}

fn read_lane(
    path: &Path,
    source: ScheduleSource,
    reader: fn(&str) -> CrawlResult<ReadOutcome>,
) -> Result<LaneRead> {
    read_lane_with_token(path, source, source.label(), reader)
}

fn read_lane_with_token(
    path: &Path,
    source: ScheduleSource,
    token: &str,
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
    let mut captured: Vec<IdentifiedKey> = outcome
        .entries()
        .iter()
        .filter_map(|entry| IdentifiedKey::of(entry.key()))
        .collect();
    captured.sort();
    captured.dedup();
    let report = LaneReport {
        source: token.to_string(),
        path: path.display().to_string(),
        sha256,
        entries: counts.entries,
        skipped: counts.skipped,
        notes: counts.notes,
        skipped_rows: rows(outcome.skipped()),
        note_rows: rows(outcome.notes()),
        captured,
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
