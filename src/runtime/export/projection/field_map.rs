//! Column-key mapping: the workbook field values one row projection produces.
//!
//! Every item below was previously declared in `export/projection.rs`.

use super::super::details::status;
use super::roster::AcceptedAnnotations;
use crate::domain::identity::AthleteId;
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) fn acceptance(resolution: &super::RowResolution) -> Option<(AthleteId, &'static str)> {
    match resolution {
        super::RowResolution::Accepted { athlete_id, method } => {
            let method = match method {
                super::AcceptanceMethod::Deterministic => "deterministic",
                super::AcceptanceMethod::LocalReview => "local_review",
            };
            Some((*athlete_id, method))
        }
        super::RowResolution::CompleteSearchNoMatch | super::RowResolution::ReviewRequired => None,
    }
}

pub(super) fn assessment_summary(value: &Value) -> (String, String) {
    let candidates = value.get("candidates").and_then(Value::as_array);
    let count = candidates.map_or_else(String::new, |items| items.len().to_string());
    let strength = candidates
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("evidence_strength").and_then(Value::as_u64))
        .max()
        .map_or_else(String::new, |value| value.to_string());
    (count, strength)
}

pub(super) fn fields(
    report: &super::RowReport,
    digest: &crate::domain::identity::EvidenceDigest,
    accepted: Option<(AthleteId, &'static str)>,
    annotations: AcceptedAnnotations,
    candidate_count: String,
    strength: String,
    pr_summary: String,
) -> BTreeMap<String, String> {
    let (athlete_id, method) = match accepted {
        Some((id, method)) => (id.get().to_string(), method.to_owned()),
        None => (String::new(), String::new()),
    };
    [
        ("native.source_key", report.job.source.as_str().to_owned()),
        (
            "native.terminal_status",
            status(&report.resolution).to_owned(),
        ),
        ("native.athlete_id", athlete_id),
        ("native.profile_url", annotations.profile_url),
        ("native.competing_school", annotations.competing_school),
        ("native.school_history", annotations.school_history),
        ("native.junior_evidence", annotations.junior_evidence),
        ("native.junior_status", annotations.junior_status),
        ("native.acceptance_method", method),
        ("native.row_report_digest", digest.as_str().to_owned()),
        (
            "native.assessment_digest",
            report
                .assessment
                .as_ref()
                .map_or_else(String::new, |value| value.as_str().to_owned()),
        ),
        ("native.candidate_count", candidate_count),
        (
            "native.eligibility_basis",
            "source_workbook_membership".to_owned(),
        ),
        ("native.evidence_strength", strength),
        ("native.pr_summary", pr_summary),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), value))
    .collect()
}
