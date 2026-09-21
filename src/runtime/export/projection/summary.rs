//! Emission of `native.pr_summary`: the compact observed-best and source-claim JSON, and the
//! detail-sidecar reference that replaces it when the cell would exceed the Excel limit.
//!
//! Every item below was previously declared in `export/projection.rs`.

use crate::domain::{
    evidence::{BestClaim, EvidenceRef, ProfileEvidence},
    identity::{AthleteId, EvidenceDigest},
    marks::MarkValue,
    performance_evidence::{
        summarize_performances, BestClaimKind, Completeness, PerformanceContext, PerformanceSummary,
    },
};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::collections::BTreeSet;

pub(super) const MAX_EXCEL_CELL_UTF16_UNITS: usize = 32_767;

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

pub(super) fn build_pr_summary(
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
