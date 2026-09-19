//! Independent verification of exported result-detail JSONL.
//!
//! Reconstructs planned discovery, complete profiles and source-bound exclusions
//! from captured requests and raw responses, then recomputes canonical assessments.
//! `bundle_verify` separately binds these results to preserved source/XLSX fields.

mod assessment;
mod checks;
mod complete;
mod coverage;
mod discovery;
mod exclusions;
mod raw_profiles;
mod source_receipts;
mod rankings;

use crate::store::ArtifactStore;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

const MAX_DETAIL_LINE_BYTES: u64 = 128 * 1024 * 1024;

/// Counts independently verified terminal states in an exported detail artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResultVerificationReport {
    pub total_rows: u64,
    pub accepted_rows: u64,
    pub review_rows: u64,
    pub no_match_rows: u64,
    pub pending_rows: u64,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DetailRow {
    pub(super) source: crate::model::SourceRecord,
    pub(super) discovery: Option<serde_json::Value>,
    pub(super) identity_artifacts: Vec<serde_json::Value>,
    pub(super) report_digest: Option<crate::domain::identity::EvidenceDigest>,
    pub(super) report: Option<crate::runtime::row_protocol::RowReport>,
    pub(super) assessment: Option<serde_json::Value>,
    pub(super) profile_artifacts: Vec<serde_json::Value>,
    pub(super) performance_evidence: Vec<serde_json::Value>,
}

/// Independently verifies positive decisions, retained coverage, and detail-row integrity.
///
/// The verifier reads the JSONL detail artifact and the existing stopped
/// ArtifactStore. It checks source/report binding, captured HTTP request/response
/// operations, planned discovery, raw complete-profile and exclusion evidence,
/// canonical assessments, and (when present) local-review selections. Incomplete
/// acquisitions remain review evidence, not proof of a complete upstream corpus.
/// It does not establish workbook membership, verify exported XLSX fields, or
/// authenticate upstream documents; `bundle_verify` checks workbook provenance.
pub fn verify_results(detail: &Path, store: &ArtifactStore) -> Result<ResultVerificationReport> {
    let file = File::open(detail)
        .with_context(|| format!("opening detail artifact {}", detail.display()))?;
    let mut reader = BufReader::new(file);
    let mut seen_sources = HashSet::new();
    let mut report = ResultVerificationReport {
        total_rows: 0,
        accepted_rows: 0,
        review_rows: 0,
        no_match_rows: 0,
        pending_rows: 0,
    };
    let mut line = Vec::new();
    let mut line_number = 0_u64;
    loop {
        line.clear();
        let length = reader
            .by_ref()
            .take(MAX_DETAIL_LINE_BYTES.saturating_add(1))
            .read_until(b'\n', &mut line)
            .with_context(|| format!("reading detail line {}", line_number.saturating_add(1)))?;
        if length == 0 {
            break;
        }
        line_number = line_number
            .checked_add(1)
            .context("detail line count overflow")?;
        validate_line(&line, length, detail, line_number)?;
        let row: DetailRow = serde_json::from_slice(&line)
            .with_context(|| format!("decoding detail line {line_number}"))?;
        let raw: Value = serde_json::from_slice(&line)
            .with_context(|| format!("decoding detail line {line_number}"))?;
        if !seen_sources.insert(row.source.source_key.clone()) {
            bail!(
                "duplicate source row key at detail line {line_number}: {}",
                row.source.source_key
            );
        }
        checks::verify_embedded_artifacts(&row, &raw, store).with_context(|| {
            format!("verifying embedded artifact serialization at detail line {line_number}")
        })?;
        let state = checks::verify_row(&row)
            .with_context(|| format!("verifying detail line {line_number}"))?;
        report.total_rows = report
            .total_rows
            .checked_add(1)
            .context("detail row count overflow")?;
        state.add_to(&mut report)?;
    }
    Ok(report)
}
fn validate_line(line: &[u8], length: usize, detail: &Path, number: u64) -> Result<()> {
    if u64::try_from(length)? > MAX_DETAIL_LINE_BYTES {
        bail!(
            "detail line {number} exceeds 128 MiB in {}",
            detail.display()
        );
    }
    if line.last() != Some(&b'\n') {
        bail!("detail line {number} is not newline-terminated");
    }
    if line[..line.len().saturating_sub(1)]
        .iter()
        .all(u8::is_ascii_whitespace)
    {
        bail!("detail line {number} is blank");
    }
    Ok(())
}
