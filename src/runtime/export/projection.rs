use super::{details::status, index::load_json};
use crate::{
    domain::{
        evidence::{ProfileEvidence, ResultEvidence},
        identity::AthleteId,
        performance_evidence::{summarize_performances, BestClaimKind, PerformanceSummary},
    },
    store::ArtifactStore,
};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) struct Projection {
    pub(crate) report: super::RowReport,
    pub(crate) assessment: Option<Value>,
    pub(crate) profile_artifacts: Vec<Value>,
    pub(crate) fields: BTreeMap<String, String>,
}

pub(crate) fn project_report(
    store: &ArtifactStore,
    digest: &crate::domain::identity::EvidenceDigest,
) -> Result<Projection> {
    let report: super::RowReport = load_json(store, digest)?;
    let assessment = report
        .assessment
        .as_ref()
        .map(|value| load_json(store, value))
        .transpose()?;
    let loaded = report
        .profile_evidence
        .iter()
        .map(|value| load_profile(store, value))
        .collect::<Result<Vec<_>>>()?;
    let profile_artifacts = loaded
        .iter()
        .map(|(value, _)| value.clone())
        .collect::<Vec<_>>();
    let profiles = loaded
        .into_iter()
        .filter_map(|(_, acquisition)| acquisition.profile)
        .collect::<Vec<_>>();
    let accepted = acceptance(&report.resolution);
    let profile_url = accepted_profile_url(&profiles, accepted.map(|(id, _)| id));
    let (candidate_count, strength) = assessment
        .as_ref()
        .map_or((String::new(), String::new()), assessment_summary);
    let pr_summary = match accepted {
        Some((athlete_id, _)) => build_pr_summary(&profiles, athlete_id)?,
        None => String::new(),
    };
    let fields = fields(
        &report,
        digest,
        accepted,
        profile_url,
        candidate_count,
        strength,
        pr_summary,
    );
    Ok(Projection {
        report,
        assessment,
        profile_artifacts,
        fields,
    })
}

fn load_profile(
    store: &ArtifactStore,
    digest: &crate::domain::identity::EvidenceDigest,
) -> Result<(Value, super::ProfileAcquisition)> {
    let value: Value = load_json(store, digest)?;
    let acquisition =
        serde_json::from_value(value.clone()).context("decoding profile acquisition")?;
    Ok((value, acquisition))
}

fn acceptance(resolution: &super::RowResolution) -> Option<(AthleteId, &'static str)> {
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

fn assessment_summary(value: &Value) -> (String, String) {
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

fn fields(
    report: &super::RowReport,
    digest: &crate::domain::identity::EvidenceDigest,
    accepted: Option<(AthleteId, &'static str)>,
    profile_url: String,
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
        ("native.profile_url", profile_url),
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

fn accepted_profile_url(profiles: &[ProfileEvidence], athlete_id: Option<AthleteId>) -> String {
    profiles
        .iter()
        .find(|profile| Some(profile.athlete_id) == athlete_id)
        .map_or_else(String::new, |profile| {
            profile.profile_url.as_str().to_owned()
        })
}

#[derive(Debug, Serialize)]
struct PrSummary<'a> {
    observed_best: &'a [crate::domain::performance_evidence::ObservedBestGroup],
    source_personal_best_claims: Vec<crate::domain::performance_evidence::SourceBestClaim>,
    completeness: crate::domain::performance_evidence::Completeness,
}

fn build_pr_summary(profiles: &[ProfileEvidence], athlete_id: AthleteId) -> Result<String> {
    let results = profiles
        .iter()
        .filter(|profile| profile.athlete_id == athlete_id)
        .flat_map(|profile| profile.results.iter().cloned())
        .collect::<Vec<ResultEvidence>>();
    let summary: PerformanceSummary =
        summarize_performances(&results).context("summarizing selected-athlete performances")?;
    let source_personal_best_claims = summary
        .source_best_claims
        .iter()
        .filter(|claim| matches!(&claim.kind, BestClaimKind::PersonalBest))
        .cloned()
        .collect();
    serde_json::to_string(&PrSummary {
        observed_best: &summary.observed_best_groups,
        source_personal_best_claims,
        completeness: summary.completeness,
    })
    .context("serializing selected-athlete performance summary")
}

pub(crate) fn raw_performance_values(profile: &Value) -> Vec<Value> {
    profile
        .get("profile")
        .and_then(|value| value.get("results"))
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |results| results.clone())
}
