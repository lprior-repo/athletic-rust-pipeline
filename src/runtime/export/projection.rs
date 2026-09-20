use super::{details::status, index::load_json};
use crate::{
    domain::{
        evidence::{BestClaim, EvidenceRef, ProfileEvidence},
        identity::{AthleteId, EvidenceDigest},
        marks::MarkValue,
        performance_evidence::{
            summarize_performances, BestClaimKind, Completeness, PerformanceContext,
            PerformanceSummary,
        },
    },
    store::ArtifactStore,
};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const MAX_EXCEL_CELL_UTF16_UNITS: usize = 32_767;

#[derive(Debug)]
pub(crate) struct Projection {
    pub(crate) report: super::RowReport,
    pub(crate) discovery: Option<Value>,
    pub(crate) identity_artifacts: Vec<Value>,
    pub(crate) assessment: Option<Value>,
    pub(crate) profile_artifacts: Vec<Value>,
    pub(crate) fields: BTreeMap<String, String>,
}
pub(crate) fn project_report(
    store: &ArtifactStore,
    digest: &crate::domain::identity::EvidenceDigest,
) -> Result<Projection> {
    let report: super::RowReport = load_json(store, digest)?;
    let discovery = report
        .discovery
        .as_ref()
        .map(|value| load_json(store, value))
        .transpose()?;
    let assessment = report
        .assessment
        .as_ref()
        .map(|value| load_json(store, value))
        .transpose()?;
    let identity_artifacts = report
        .candidates
        .iter()
        .filter_map(|candidate| candidate.probe())
        .scan(BTreeSet::new(), |seen, value| {
            seen.insert(value.as_str().to_owned()).then_some(value)
        })
        .map(|value| load_json(store, value))
        .collect::<Result<Vec<Value>>>()?;
    let profile_artifacts = report
        .candidates
        .iter()
        .filter_map(|candidate| candidate.profile())
        .map(|value| load_profile(store, value).map(|(raw, _)| raw))
        .collect::<Result<Vec<_>>>()?;
    let profiles = report
        .candidates
        .iter()
        .filter_map(|candidate| match candidate {
            super::CandidateCoverage::Complete { profile, .. } => {
                Some(load_profile(store, profile))
            }
            _ => None,
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .map(|(_, acquisition)| acquisition)
        .collect::<Vec<_>>();
    let accepted = acceptance(&report.resolution);
    let selected_profiles = accepted.map_or_else(Vec::new, |(athlete_id, _)| {
        profiles
            .iter()
            .filter(|acquisition| {
                acquisition.complete
                    && acquisition.athlete_id == athlete_id
                    && acquisition.profile.is_some()
            })
            .filter_map(|acquisition| acquisition.profile.clone())
            .collect::<Vec<_>>()
    });
    let annotations = annotations_for(&selected_profiles, accepted.map(|(id, _)| id));
    let (candidate_count, strength) = assessment
        .as_ref()
        .map_or((String::new(), String::new()), assessment_summary);
    let pr_summary = match accepted {
        Some((athlete_id, _)) => build_pr_summary(
            &selected_profiles,
            athlete_id,
            report.job.source.as_str(),
            digest,
        )?,
        None => String::new(),
    };
    let fields = fields(
        &report,
        digest,
        accepted,
        annotations,
        candidate_count,
        strength,
        pr_summary,
    );
    Ok(Projection {
        report,
        discovery,
        identity_artifacts,
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

fn annotations_for(
    profiles: &[ProfileEvidence],
    athlete_id: Option<AthleteId>,
) -> AcceptedAnnotations {
    profiles
        .iter()
        .find(|profile| Some(profile.athlete_id) == athlete_id)
        .map_or_else(AcceptedAnnotations::default, |profile| {
            AcceptedAnnotations {
                profile_url: profile.profile_url.as_str().to_owned(),
                competing_school: competing_school(profile),
                junior_evidence: junior_evidence(profile),
                junior_status: junior_status(profile),
            }
        })
}

/// Expanded-roster annotations the accepted profile already evidences: its profile URL,
/// the school the athlete competed for in the newest evidenced season, every season-bound
/// grade observation, and the roster verdict that the newest grade row supports. All empty
/// when no profile was accepted; the school and grade strings are not claims of a complete
/// history — that stays in the retained profile evidence and the JSONL sidecar, including
/// the season/result behind each observation.
#[derive(Debug, Default, PartialEq, Eq)]
struct AcceptedAnnotations {
    profile_url: String,
    competing_school: String,
    junior_evidence: String,
    junior_status: String,
}

fn competing_school(profile: &ProfileEvidence) -> String {
    let Some(newest) = profile
        .teams
        .iter()
        .filter_map(|team| team.seasons.iter().copied().max())
        .max()
    else {
        return String::new();
    };
    let mut names: Vec<&str> = profile
        .teams
        .iter()
        .filter(|team| team.seasons.contains(&newest))
        .map(|team| team.name.value.as_str())
        .collect();
    names.sort_unstable();
    names.dedup();
    names.join("; ")
}

fn junior_evidence(profile: &ProfileEvidence) -> String {
    let mut observations: Vec<(u16, u8)> = profile
        .grades
        .iter()
        .map(|grade| (grade.season, grade.grade))
        .collect();
    observations.sort_unstable_by(|left, right| right.cmp(left));
    observations.dedup();
    observations
        .into_iter()
        .map(|(season, grade)| format!("grade {grade} @ {season}"))
        .collect::<Vec<_>>()
        .join("; ")
}

/// The roster verdict, decided only by the newest season-bound grade row: the season is
/// part of the verdict, and an athlete whose grades never include a standard US high-school
/// grade stays undetermined rather than assumed. Name, age, and graduation year are never
/// consulted.
fn junior_status(profile: &ProfileEvidence) -> String {
    let Some((season, grade)) = profile
        .grades
        .iter()
        .map(|grade| (grade.season, grade.grade))
        .max()
    else {
        return String::new();
    };
    let name = match grade {
        9 => "freshman",
        10 => "sophomore",
        11 => "junior",
        12 => "senior",
        _ => return String::new(),
    };
    format!("{name} @ {season}")
}

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
    athlete_id: AthleteId,
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
    let observed_best = summary
        .observed_best_groups
        .iter()
        .map(compact_observed_best)
        .collect::<Vec<_>>();
    let source_personal_best_claims = compact_source_claims(&summary);
    let compact = PrSummary {
        observed_best,
        source_personal_best_claims,
        completeness: summary.completeness,
    };
    let encoded = serde_json::to_string(&compact)
        .context("serializing selected-athlete performance summary")?;
    if excel_cell_utf16_units(&encoded) <= MAX_EXCEL_CELL_UTF16_UNITS {
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
    if excel_cell_utf16_units(&encoded) > MAX_EXCEL_CELL_UTF16_UNITS {
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
        .collect::<BTreeSet<_>>();
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

fn excel_cell_utf16_units(value: &str) -> usize {
    value.encode_utf16().count()
}

pub(crate) fn raw_performance_values(profile: &Value) -> Vec<Value> {
    profile
        .get("profile")
        .and_then(|value| value.get("results"))
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |results| results.clone())
}
#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
