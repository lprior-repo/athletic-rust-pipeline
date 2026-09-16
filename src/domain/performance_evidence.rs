//! Evidence-backed performance normalization and context-safe observed-best projection.
//!
//! This module never turns a source claim into a computed best.  Source claims
//! and the best marks observed in the supplied (possibly incomplete) sample are
//! separate projections.

use super::error::DomainError;
use super::evidence::{BestClaim, EvidenceRef, ResultEvidence, Sport};
use super::marks::{EventName, Performance};

mod context;
#[cfg(test)]
mod tests;
use context::{
    context, observed_bests, parse_mark, source_claims, source_event, source_unit_for, timing_for,
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MarkObservation {
    Parsed(Performance),
    Unsupported { raw: String, reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BestClaimKind {
    PersonalBest,
    SeasonBest,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceBestClaim {
    pub result_id: u64,
    pub kind: BestClaimKind,
    pub raw: BestClaim,
    pub claimed: Option<bool>,
    pub evidence: EvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SurfaceContext {
    Indoor,
    Outdoor,
    NotApplicable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TimingBasis {
    Fat,
    Hand,
    Other(String),
    NotApplicable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WindLegality {
    Legal,
    Illegal,
    NotApplicable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EquipmentContext {
    Hurdles(String),
    Implement(String),
    NotApplicable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SourceUnit {
    Seconds,
    Meters,
    Centimeters,
    Millimeters,
    FeetInches,
    Points,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AttributionContext {
    Individual,
    Relay(u64),
    Unknown,
}

/// Every field that can make two marks incomparable is intentionally explicit.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PerformanceContext {
    pub sport: Sport,
    pub event: EventName,
    pub distance: Option<String>,
    pub surface: SurfaceContext,
    pub event_type: Option<String>,
    pub timing: TimingBasis,
    pub units: SourceUnit,
    pub equipment: EquipmentContext,
    pub wind: WindLegality,
    pub attribution: AttributionContext,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PerformanceObservation {
    pub result_id: u64,
    pub displayed_mark: String,
    pub mark: MarkObservation,
    pub context: PerformanceContext,
    pub evidence: EvidenceRef,
    pub source: ResultEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ObservedBestGroup {
    pub context: PerformanceContext,
    pub best: PerformanceObservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Completeness {
    RetrievedSampleOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PerformanceSummary {
    pub observations: Vec<PerformanceObservation>,
    pub source_best_claims: Vec<SourceBestClaim>,
    pub observed_best_groups: Vec<ObservedBestGroup>,
    pub completeness: Completeness,
}

pub fn summarize_performances(
    results: &[ResultEvidence],
) -> Result<PerformanceSummary, DomainError> {
    let observations: Vec<_> = results.iter().map(observe).collect();
    let source_best_claims = results.iter().flat_map(source_claims).collect();
    let observed_best_groups = observed_bests(&observations);
    Ok(PerformanceSummary {
        observations,
        source_best_claims,
        observed_best_groups,
        completeness: Completeness::RetrievedSampleOnly,
    })
}

fn observe(source: &ResultEvidence) -> PerformanceObservation {
    let event = source_event(source);
    let timing = timing_for(source, &event);
    let units = source_unit_for(source, &event);
    let context = context(source, event.clone(), timing.clone(), units.clone());
    let mark = parse_mark(source, &event, &timing, &units);
    PerformanceObservation {
        result_id: source.result_id,
        displayed_mark: source.mark.clone(),
        mark,
        context,
        evidence: source.evidence.clone(),
        source: source.clone(),
    }
}
