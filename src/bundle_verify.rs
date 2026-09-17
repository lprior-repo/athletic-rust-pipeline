//! Cross-artifact verification. Source fields are independently read by the field
//! verifier; the streamed XLSX annotations must also bind the verified JSONL rows.
use crate::{
    domain::{
        evidence::{BestClaim, EvidenceRef, ProfileEvidence},
        identity::{EvidenceDigest, ProfileUrl, WorkbookDigest},
        marks::MarkValue,
        performance_evidence::{
            summarize_performances, BestClaimKind, Completeness, PerformanceContext,
            PerformanceSummary,
        },
    },
    model::SourceRecord,
    result_verify::{verify_results, ResultVerificationReport},
    runtime::{
        acquisition::ProfileAcquisition,
        row_protocol::{AcceptanceMethod, RowReport, RowResolution},
    },
    store::ArtifactStore,
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
/// Verifies retained evidence consistency and revalidates raw exclusion receipts.
pub fn verify_bundle(
    original: &Path,
    output: &Path,
    expected_sha: &WorkbookDigest,
    extra_headers: &[String],
    store: &ArtifactStore,
) -> Result<BundleVerificationReport> {
    let detail = output.with_extension("jsonl");
    let output_sha256 = hash_file(output)?;
    let detail_sha256 = hash_file(&detail)?;
    let fields = verify_fields(original, output, expected_sha, extra_headers)?;
    let results = verify_results(&detail, store)?;
    if results.total_rows != fields.matched_row_count {
        bail!("sidecar row accounting differs from preserved source rows");
    }
    bind_rows(output, &detail, expected_sha, extra_headers, store)?;
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
    assessment: Option<serde_json::Value>,
    profile_artifacts: Vec<serde_json::Value>,
}
fn bind_rows(
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
const MAX_EXCEL_CELL_UTF16_UNITS: usize = 32_767;

#[derive(Debug, Serialize)]
struct PrSummary<'a> {
    observed_best: Vec<CompactObservedBest<'a>>,
    source_personal_best_claims: Vec<CompactSourcePersonalBestClaim<'a>>,
    completeness: Completeness,
}

#[derive(Debug, Serialize)]
struct CompactObservedBest<'a> {
    context: &'a PerformanceContext,
    result_id: u64,
    displayed_mark: &'a str,
    comparable_mark: Option<MarkValue>,
    evidence: &'a EvidenceRef,
}

#[derive(Debug, Serialize)]
struct CompactSourcePersonalBestClaim<'a> {
    kind: &'a BestClaimKind,
    result_id: u64,
    raw: &'a BestClaim,
    claimed: Option<bool>,
    evidence: &'a EvidenceRef,
}

#[derive(Debug, Serialize)]
struct PrSummaryOverflow<'a> {
    storage: &'static str,
    source_key: &'a str,
    report_digest: &'a EvidenceDigest,
    selected_athlete_id: u64,
    observed_best_group_count: usize,
    explanation: &'static str,
}

fn build_pr_summary(
    profiles: &[ProfileEvidence],
    athlete_id: crate::domain::identity::AthleteId,
    source_key: &str,
    report_digest: &EvidenceDigest,
) -> Result<String> {
    let results = profiles
        .iter()
        .filter(|profile| profile.athlete_id == athlete_id)
        .flat_map(|profile| profile.results.iter().cloned())
        .collect::<Vec<_>>();
    let summary: PerformanceSummary =
        summarize_performances(&results).context("summarizing selected-athlete performances")?;
    let compact = PrSummary {
        observed_best: summary
            .observed_best_groups
            .iter()
            .map(compact_observed_best)
            .collect(),
        source_personal_best_claims: compact_source_claims(&summary),
        completeness: summary.completeness,
    };
    let encoded = serde_json::to_string(&compact)
        .context("serializing selected-athlete performance summary")?;
    if encoded.encode_utf16().count() <= MAX_EXCEL_CELL_UTF16_UNITS {
        return Ok(encoded);
    }
    let overflow = PrSummaryOverflow {
        storage: "detail_sidecar",
        source_key,
        report_digest,
        selected_athlete_id: athlete_id.get(),
        observed_best_group_count: summary.observed_best_groups.len(),
        explanation:
            "full selected-athlete observations and source claims are retained in row JSONL",
    };
    let encoded = serde_json::to_string(&overflow)
        .context("serializing selected-athlete summary sidecar reference")?;
    if encoded.encode_utf16().count() > MAX_EXCEL_CELL_UTF16_UNITS {
        bail!("selected-athlete summary sidecar reference exceeds Excel cell limit");
    }
    Ok(encoded)
}

fn compact_observed_best<'a>(
    group: &'a crate::domain::performance_evidence::ObservedBestGroup,
) -> CompactObservedBest<'a> {
    let comparable_mark = match &group.best.mark {
        crate::domain::performance_evidence::MarkObservation::Parsed(mark) => mark.comparable(),
        crate::domain::performance_evidence::MarkObservation::Unsupported { .. } => None,
    };
    CompactObservedBest {
        context: &group.context,
        result_id: group.best.result_id,
        displayed_mark: &group.best.displayed_mark,
        comparable_mark,
        evidence: &group.best.evidence,
    }
}

fn compact_source_claims<'a>(
    summary: &'a PerformanceSummary,
) -> Vec<CompactSourcePersonalBestClaim<'a>> {
    let selected_results = summary
        .observed_best_groups
        .iter()
        .map(|group| {
            (
                group.best.result_id,
                group.best.evidence.document.as_str(),
                group.best.evidence.locator.as_str(),
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    summary
        .source_best_claims
        .iter()
        .filter(|claim| {
            matches!(&claim.kind, BestClaimKind::PersonalBest)
                && selected_results.contains(&(
                    claim.result_id,
                    claim.evidence.document.as_str(),
                    claim.evidence.locator.as_str(),
                ))
        })
        .map(|claim| CompactSourcePersonalBestClaim {
            kind: &claim.kind,
            result_id: claim.result_id,
            raw: &claim.raw,
            claimed: claim.claimed,
            evidence: &claim.evidence,
        })
        .collect()
}

fn check_field(row: &SourceRecord, name: &str, expected: &str) -> Result<()> {
    if row.fields.get(name).map(String::as_str) != Some(expected) {
        bail!("workbook annotation differs from sidecar evidence: {name}");
    }
    Ok(())
}
