use super::{pr_summary::build_pr_summary, MAX_DETAIL_ROW_BYTES};
use crate::{
    domain::identity::{EvidenceDigest, ProfileUrl, WorkbookDigest},
    model::SourceRecord,
    runtime::{
        acquisition::ProfileAcquisition,
        row_protocol::{AcceptanceMethod, RowReport, RowResolution},
    },
    store::ArtifactStore,
    workbook_ingest,
};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

#[derive(Deserialize)]
struct DetailBinding {
    source: SourceRecord,
    report_digest: Option<EvidenceDigest>,
    report: Option<RowReport>,
    assessment: Option<serde_json::Value>,
    profile_artifacts: Vec<serde_json::Value>,
}

pub(super) fn bind_rows(
    output: &Path,
    detail: &Path,
    expected_sha: &WorkbookDigest,
    extra_headers: &[String],
    store: &ArtifactStore,
) -> Result<()> {
    let mut reader = BufReader::new(File::open(detail).context("opening result sidecar")?);
    let mut buffer = Vec::new();
    workbook_ingest::visit_records(output, |row| {
        buffer.clear();
        let bytes = reader
            .by_ref()
            .take(MAX_DETAIL_ROW_BYTES + 1)
            .read_until(b'\n', &mut buffer)?;
        if bytes == 0 {
            bail!("sidecar is missing a workbook row");
        }
        if u64::try_from(bytes)? > MAX_DETAIL_ROW_BYTES {
            bail!("sidecar row exceeds verification bound");
        }
        let bound: DetailBinding =
            serde_json::from_slice(&buffer).context("decoding sidecar row binding")?;
        check_binding(&row, &bound, expected_sha, extra_headers, store)
    })?;
    if !reader.fill_buf()?.is_empty() {
        bail!("sidecar has rows after workbook EOF");
    }
    Ok(())
}

fn check_binding(
    row: &SourceRecord,
    bound: &DetailBinding,
    expected_sha: &WorkbookDigest,
    extra_headers: &[String],
    _store: &ArtifactStore,
) -> Result<()> {
    if row.source_key != bound.source.source_key
        || row.sheet != bound.source.sheet
        || row.excel_row != bound.source.excel_row
    {
        bail!("sidecar and workbook source-row identities differ");
    }
    check_source_fields(row, bound, extra_headers)?;
    check_field(row, "native.source_key", &bound.source.source_key)?;
    check_field(
        row,
        "native.eligibility_basis",
        "source_workbook_membership",
    )?;
    match (&bound.report, &bound.report_digest) {
        (None, None) => {
            check_field(row, "native.terminal_status", "PENDING")?;
            for field in [
                "native.athlete_id",
                "native.profile_url",
                "native.acceptance_method",
                "native.row_report_digest",
                "native.assessment_digest",
                "native.pr_summary",
            ] {
                check_field(row, field, "")?;
            }
        }
        (Some(report), Some(digest)) => {
            if report.job.workbook != *expected_sha || report.job.source.as_str() != row.source_key
            {
                bail!("row report does not bind original workbook and source key");
            }
            check_field(row, "native.row_report_digest", digest.as_str())?;
            check_field(
                row,
                "native.assessment_digest",
                report
                    .assessment
                    .as_ref()
                    .map_or("", EvidenceDigest::as_str),
            )?;

            check_terminal_annotations(row, bound, report)?;
            check_resolution(row, &report.resolution)?;
        }
        _ => bail!("sidecar row report and digest presence differ"),
    }
    Ok(())
}

fn check_source_fields(
    row: &SourceRecord,
    bound: &DetailBinding,
    extra_headers: &[String],
) -> Result<()> {
    let original_count = row
        .fields
        .keys()
        .filter(|key| !extra_headers.iter().any(|extra| extra == *key))
        .count();
    if bound.source.fields.len() != original_count
        || bound.source.fields.iter().any(|(key, value)| {
            extra_headers.iter().any(|extra| extra == key) || row.fields.get(key) != Some(value)
        })
        || row.fields.iter().any(|(key, value)| {
            !extra_headers.iter().any(|extra| extra == key)
                && bound.source.fields.get(key) != Some(value)
        })
    {
        bail!("sidecar source fields differ from workbook fields");
    }
    Ok(())
}

fn check_resolution(row: &SourceRecord, resolution: &RowResolution) -> Result<()> {
    let status = match resolution {
        RowResolution::Accepted { athlete_id, method } => {
            check_field(row, "native.terminal_status", "ACCEPTED")?;
            check_field(row, "native.athlete_id", &athlete_id.get().to_string())?;
            let method = match method {
                AcceptanceMethod::Deterministic => "deterministic",
                AcceptanceMethod::LocalReview => "local_review",
            };
            check_field(row, "native.acceptance_method", method)?;
            let url = row
                .fields
                .get("native.profile_url")
                .context("workbook lacks profile URL annotation")?;
            if ProfileUrl::parse(url)?.athlete_id() != *athlete_id {
                bail!("workbook profile URL does not bind accepted athlete");
            }
            return Ok(());
        }
        RowResolution::CompleteSearchNoMatch => "COMPLETE_SEARCH_NO_MATCH",
        RowResolution::ReviewRequired => "REVIEW_REQUIRED",
    };
    check_field(row, "native.terminal_status", status)?;
    for field in [
        "native.athlete_id",
        "native.profile_url",
        "native.acceptance_method",
        "native.pr_summary",
    ] {
        check_field(row, field, "")?;
    }
    Ok(())
}

fn check_terminal_annotations(
    row: &SourceRecord,
    bound: &DetailBinding,
    report: &RowReport,
) -> Result<()> {
    let (candidate_count, strength) = bound
        .assessment
        .as_ref()
        .map_or((String::new(), String::new()), assessment_summary);
    check_field(row, "native.candidate_count", &candidate_count)?;
    check_field(row, "native.evidence_strength", &strength)?;
    let pr_summary = match &report.resolution {
        RowResolution::Accepted { athlete_id, .. } => {
            let acquisitions = bound
                .profile_artifacts
                .iter()
                .map(|value| {
                    serde_json::from_value::<ProfileAcquisition>(value.clone())
                        .context("decoding profile acquisition for PR summary")
                })
                .collect::<Result<Vec<_>>>()?;
            let profiles = acquisitions
                .into_iter()
                .filter_map(|acquisition| acquisition.profile)
                .collect::<Vec<_>>();
            build_pr_summary(
                &profiles,
                *athlete_id,
                &bound.source.source_key,
                bound
                    .report_digest
                    .as_ref()
                    .context("accepted row is missing report digest")?,
            )?
        }
        RowResolution::CompleteSearchNoMatch | RowResolution::ReviewRequired => String::new(),
    };
    check_field(row, "native.pr_summary", &pr_summary)
}

fn assessment_summary(value: &serde_json::Value) -> (String, String) {
    let candidates = value
        .get("candidates")
        .and_then(serde_json::Value::as_array);
    let count = candidates.map_or_else(String::new, |items| items.len().to_string());
    let strength = candidates
        .into_iter()
        .flatten()
        .filter_map(|item| {
            item.get("evidence_strength")
                .and_then(serde_json::Value::as_u64)
        })
        .max()
        .map_or_else(String::new, |value| value.to_string());
    (count, strength)
}

fn check_field(row: &SourceRecord, name: &str, expected: &str) -> Result<()> {
    if row.fields.get(name).map(String::as_str) != Some(expected) {
        bail!("workbook annotation differs from sidecar evidence: {name}");
    }
    Ok(())
}
