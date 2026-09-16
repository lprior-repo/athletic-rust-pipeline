//! Cross-artifact verification. Source fields are independently read by the field
//! verifier; the streamed XLSX annotations must also bind the verified JSONL rows.
use crate::{
    domain::identity::{EvidenceDigest, ProfileUrl, WorkbookDigest},
    model::SourceRecord,
    result_verify::{verify_results, ResultVerificationReport},
    runtime::row_protocol::{AcceptanceMethod, RowReport, RowResolution},
    workbook_ingest,
    workbook_verify::{verify_fields, VerificationReport},
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
};

const MAX_DETAIL_ROW_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleVerificationReport {
    pub fields: VerificationReport,
    pub results: ResultVerificationReport,
    pub output_sha256: String,
    pub detail_sha256: String,
}

/// Verifies retained evidence consistency, not independent real-world truth.
pub fn verify_bundle(
    original: &Path,
    output: &Path,
    expected_sha: &WorkbookDigest,
    extra_headers: &[String],
) -> Result<BundleVerificationReport> {
    let detail = output.with_extension("jsonl");
    let output_sha256 = hash_file(output)?;
    let detail_sha256 = hash_file(&detail)?;
    let fields = verify_fields(original, output, expected_sha, extra_headers)?;
    let results = verify_results(&detail)?;
    if results.total_rows != fields.matched_row_count {
        bail!("sidecar row accounting differs from preserved source rows");
    }
    bind_rows(output, &detail, expected_sha, extra_headers.len())?;
    if hash_file(output)? != output_sha256
        || hash_file(&detail)? != detail_sha256
        || hash_file(original)? != expected_sha.as_str()
    {
        bail!("bundle or original changed during verification");
    }
    Ok(BundleVerificationReport {
        fields,
        results,
        output_sha256,
        detail_sha256,
    })
}

fn hash_file(path: &Path) -> Result<String> {
    let mut reader = BufReader::new(File::open(path)?);
    let mut digest = Sha256::new();
    loop {
        let chunk = reader.fill_buf()?;
        if chunk.is_empty() {
            break;
        }
        digest.update(chunk);
        let length = chunk.len();
        reader.consume(length);
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[derive(Deserialize)]
struct DetailBinding {
    source: SourceRecord,
    report_digest: Option<EvidenceDigest>,
    report: Option<RowReport>,
}

fn bind_rows(
    output: &Path,
    detail: &Path,
    expected_sha: &WorkbookDigest,
    extra_count: usize,
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
        check_binding(&row, &bound, expected_sha, extra_count)
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
    extra_count: usize,
) -> Result<()> {
    if row.source_key != bound.source.source_key
        || row.sheet != bound.source.sheet
        || row.excel_row != bound.source.excel_row
    {
        bail!("sidecar and workbook source-row identities differ");
    }
    if bound.source.fields.len().checked_add(extra_count) != Some(row.fields.len())
        || bound
            .source
            .fields
            .iter()
            .any(|(key, value)| row.fields.get(key) != Some(value))
    {
        bail!("sidecar source fields differ from workbook fields");
    }
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
            check_resolution(row, &report.resolution)?;
        }
        _ => bail!("sidecar row report and digest presence differ"),
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

fn check_field(row: &SourceRecord, name: &str, expected: &str) -> Result<()> {
    if row.fields.get(name).map(String::as_str) != Some(expected) {
        bail!("workbook annotation differs from sidecar evidence: {name}");
    }
    Ok(())
}
