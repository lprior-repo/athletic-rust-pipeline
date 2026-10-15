use super::{ReportError, ReportResult, Scope};
use crate::export::ExportDataset;
use census_domain::JurisdictionBucket;
use serde::Serialize;
use std::collections::BTreeMap;

mod athletes;
mod classify;
mod contact;
mod gaps;
mod notes;
mod population;
mod pr_support;
mod reads;
mod report_impl;
mod retained;
mod scan;
mod scoped;
mod state;

pub(crate) use state::{in_requested_year, jurisdiction_of, school_state_index};

pub use contact::ContactCounts;
pub use gaps::{CoverageGap, GapClass};

use state::Outcome;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use retained::tests::correction::split as split_identity_fixture;

pub use census_domain::model::AccessBlockKind;

pub const UNKNOWN_JURISDICTION: &str = JurisdictionBucket::UNPLACED_CODE;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct CoverageTotals {
    pub schools: usize,
    pub athletes: usize,
    pub coaches: usize,
    pub meets: usize,
    pub performances: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct SourcePopulation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registered: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applicable: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attempted: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub withheld: Option<usize>,
}

impl SourcePopulation {
    pub fn is_measured(&self) -> bool {
        self.registered.is_some()
            && self.applicable.is_some()
            && self.attempted.is_some()
            && self.captured.is_some()
            && self.parsed.is_some()
            && self.accepted.is_some()
            && self.withheld.is_some()
    }

    pub fn reconcile(&self) -> ReportResult<()> {
        let ordered = [
            ("registered", self.registered),
            ("attempted", self.attempted),
            ("captured", self.captured),
            ("parsed", self.parsed),
            ("accepted", self.accepted),
        ];
        for pair in ordered.windows(2) {
            let [(previous_name, previous), (next_name, next)] = pair else {
                continue;
            };
            if let (Some(previous), Some(next)) = (previous, next) {
                if previous < next {
                    return Err(ReportError::Invariant {
                        detail: format!(
                            "source population {previous_name} {previous} < {next_name} {next}"
                        ),
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JurisdictionCoverageState {
    #[default]
    Unresearched,
    Pending,
    Complete,
    Blocked,
    RetainedGap,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct JurisdictionCoverage {
    pub jurisdiction: JurisdictionBucket,
    pub schools: usize,
    pub schools_with_athletes: usize,
    pub athletes: usize,
    pub athletes_core: usize,
    pub core_share_pct: Option<u32>,
    pub boys: usize,
    pub girls: usize,
    pub unknown_gender: usize,
    pub grad_verified: usize,
    pub grad_unresolved: usize,
    pub outdoor_track: usize,
    pub indoor_track: usize,
    pub cross_country: usize,
    pub with_performance: usize,
    pub with_comparable_mark: usize,
    pub with_pr_support: usize,
    pub multisource: usize,
    pub identity_conflicts: usize,
    pub with_profile_url: usize,
    pub with_athletic_net_url: usize,
    pub with_milesplit_url: usize,
    pub coaches: usize,
    pub coaches_with_email: usize,
    pub schools_with_tf_coach: usize,
    pub schools_with_xc_coach: usize,
    pub schools_with_coach_email: usize,
    pub meets: usize,
    pub performances: usize,
    pub sources: BTreeMap<String, usize>,
    pub state: JurisdictionCoverageState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_kind: Option<AccessBlockKind>,
    pub source_population: SourcePopulation,
    pub contact: ContactCounts,
    pub terminal_failures: BTreeMap<String, usize>,
    pub raw_observations: usize,
    pub scope_filtered: usize,
    pub accepted_cohort: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageReport {
    pub grad_year: Option<i16>,
    pub event_catalog: crate::report::EventCounts,
    pub jurisdictions: Vec<JurisdictionCoverage>,
    pub gaps: Vec<CoverageGap>,
    pub read: CoverageTotals,
    pub off_cohort_athletes: usize,
    pub notes: Vec<String>,
    pub source_population: SourcePopulation,
    pub state_counts: BTreeMap<String, usize>,
    pub contact: ContactCounts,
    pub terminal_failures: BTreeMap<String, usize>,
    pub raw_observations: usize,
    pub scope_filtered: usize,
    pub accepted_cohort: usize,
}

pub(super) fn state_slug(state: JurisdictionCoverageState) -> &'static str {
    match state {
        JurisdictionCoverageState::Unresearched => "unresearched",
        JurisdictionCoverageState::Pending => "pending",
        JurisdictionCoverageState::Complete => "complete",
        JurisdictionCoverageState::Blocked => "blocked",
        JurisdictionCoverageState::RetainedGap => "retained_gap",
    }
}

pub fn coverage_report(
    dataset: &ExportDataset,
    grad_year: Option<i16>,
) -> ReportResult<CoverageReport> {
    if dataset.selected_profile_id().is_some() {
        let derivation = super::Derivation::of(dataset, Scope::AllSources, grad_year, &[])?;
        return scoped::report(&derivation);
    }
    let admission = crate::report::EventAdmission::of(dataset)?;
    let outcome = classify::run(dataset, grad_year)?;
    let state_counts = state_histogram(&outcome.jurisdictions);
    let mut report = CoverageReport {
        grad_year,
        event_catalog: admission.counts().clone(),
        notes: notes::coverage_notes(dataset, grad_year, &outcome),
        jurisdictions: outcome.jurisdictions,
        gaps: outcome.gaps,
        read: outcome.read,
        off_cohort_athletes: outcome.off_cohort_athletes,
        source_population: outcome.source_population,
        state_counts,
        contact: outcome.contact,
        terminal_failures: outcome.terminal_failures,
        raw_observations: outcome.raw_observations,
        scope_filtered: outcome.scope_filtered,
        accepted_cohort: outcome.accepted_cohort,
    };
    pr_support::apply_pr_support(&mut report, dataset, grad_year)?;
    report.reconcile()?;
    Ok(report)
}

pub fn coverage_report_for_derivation(
    derivation: &super::Derivation<'_>,
) -> ReportResult<CoverageReport> {
    scoped::report(derivation)
}

fn state_histogram(rows: &[JurisdictionCoverage]) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for row in rows {
        let key = state_slug(row.state).to_string();
        let entry = counts.entry(key).or_insert(0);
        *entry = entry.saturating_add(1);
    }
    counts
}

pub(super) fn add(total: &mut usize, value: usize) {
    *total = total.saturating_add(value);
}

pub(super) fn totals_text(totals: &CoverageTotals) -> String {
    format!(
        "schools={} athletes={} coaches={} meets={} performances={}",
        totals.schools, totals.athletes, totals.coaches, totals.meets, totals.performances
    )
}
