//! §49 coverage reporting: exact per-jurisdiction denominators, read from the store's merged entity
//! tables.
//!
//! One row per configured jurisdiction — every state and DC in `UsJurisdiction::ALL` order, with the
//! row for everything no school placed ([`UNKNOWN_JURISDICTION`]) last — plus one row per gap class
//! per jurisdiction ([`CoverageGap`], §47). Three rules shape the report:
//!
//! * **Denominators, never claims.** Every column is a row count from a table the census already
//!   consolidated (`schools`, `athletes`, `coaches`, `meets`, `performances`), so "how many Class of
//!   2027 athletes does this state hold" is answerable from the store alone.
//! * **Emptiness is reported, not omitted.** A jurisdiction with no data publishes a row of zeros
//!   *and* an `EmptyJurisdiction` gap: a missing state is the one finding coverage may not swallow.
//! * **Core and all-sources are both measured.** `core_share_pct` prints how much of a jurisdiction
//!   the platform's own [`Core`](super::Scope) evidence reaches, which is the number §49's "never
//!   describe this as complete coverage" rule needs before anyone calls a state finished.
//!
//! The report reconciles itself: [`CoverageReport::read`] is what the scans produced,
//! [`CoverageReport::published_totals`] is what the rows sum to, and [`coverage_report`] returns an
//! error instead of a report when the two disagree — the §70 items 11-12 duty, kept here rather than
//! in the caller so no path can publish an unreconciled coverage report.
//!
//! The passes live in the sibling parts: [`classify`] reads the store and fills the jurisdiction
//! rows, [`athletes`] owns the athlete and performance columns, [`gaps`] turns the measurements into
//! §47 classes, and `state` holds the accumulators they share.

use super::{ReportError, ReportResult};
use crate::store::Store;
use serde::Serialize;
use std::collections::BTreeMap;

mod athletes;
mod classify;
mod gaps;
mod state;

pub use gaps::{CoverageGap, GapClass};

use state::Outcome;

#[cfg(test)]
mod tests;

/// The label of the row no school placed: the label the census report's state buckets already use.
pub const UNKNOWN_JURISDICTION: &str = "UNKNOWN";

/// Row counts the report read from the store, and the totals its rows must sum back to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct CoverageTotals {
    pub schools: usize,
    /// Cohort athletes: the rows the store held inside the requested graduation year(s).
    pub athletes: usize,
    pub coaches: usize,
    pub meets: usize,
    pub performances: usize,
}

/// §49 coverage for one jurisdiction. `UNKNOWN` is a row like any other, so unplaceable entities
/// stay in the denominator instead of vanishing from it.
#[derive(Debug, Clone, Default, Serialize)]
pub struct JurisdictionCoverage {
    /// The USPS code, or [`UNKNOWN_JURISDICTION`].
    pub jurisdiction: String,
    /// School rows whose own `state` is this jurisdiction: the school universe.
    pub schools: usize,
    /// Schools holding at least one published cohort athlete.
    pub schools_with_athletes: usize,
    /// Cohort athletes placed here by their school's jurisdiction.
    pub athletes: usize,
    /// Those athletes the platform's own core evidence can reach (see `core_share_pct`).
    pub athletes_core: usize,
    /// `athletes_core` as a floored integer percentage of `athletes`; 0 with no athletes.
    pub core_share_pct: u32,
    pub boys: usize,
    pub girls: usize,
    /// `Gender::Mixed` and `Gender::Unknown`: counted, never guessed into a side.
    pub unknown_gender: usize,
    /// Athletes with at least one grade observation backing their cohort.
    pub grad_verified: usize,
    /// Athletes whose cohort rests on the stored grad year alone.
    pub grad_unresolved: usize,
    pub outdoor_track: usize,
    pub indoor_track: usize,
    pub cross_country: usize,
    pub with_performance: usize,
    /// Athletes with at least one mark on a real scale (time, distance, points): the precondition
    /// for a PR. The reduction itself — per-event bests, relays excluded — belongs to `bests`.
    pub with_comparable_mark: usize,
    /// Athletes reachable through two or more distinct source namespaces.
    pub multisource: usize,
    /// Athletes holding a grade observation that disagrees with their stored cohort.
    pub identity_conflicts: usize,
    pub with_profile_url: usize,
    pub with_athletic_net_url: usize,
    pub with_milesplit_url: usize,
    pub coaches: usize,
    pub coaches_with_email: usize,
    /// Schools with a head coach for a track sport (`sport` is outdoor or indoor track).
    pub schools_with_tf_coach: usize,
    /// Schools with a head coach whose sport is cross country.
    pub schools_with_xc_coach: usize,
    /// Schools with any coach row carrying a professional email.
    pub schools_with_coach_email: usize,
    /// Meets the meet table places here. Meets are not cohort-scoped, so a filter never narrows them.
    pub meets: usize,
    pub performances: usize,
    /// Adapter id to the athletes it reached here. One athlete can be reached by several adapters, so
    /// this map deliberately does not sum to `athletes`.
    pub sources: BTreeMap<String, usize>,
}

/// §49 coverage: one row per configured jurisdiction, the gap rows they produced, and the store
/// totals the rows have to sum to.
#[derive(Debug, Clone, Serialize)]
pub struct CoverageReport {
    /// The cohort filter this report was read under; `None` reads every stored athlete.
    pub grad_year: Option<i16>,
    pub jurisdictions: Vec<JurisdictionCoverage>,
    pub gaps: Vec<CoverageGap>,
    /// What the store held for the rows this report classified.
    pub read: CoverageTotals,
    /// Stored athletes the cohort filter left out: read, and never published as if in cohort.
    pub off_cohort_athletes: usize,
    pub notes: Vec<String>,
}

impl CoverageReport {
    /// The totals the published rows sum to, recomputed from them every time so a printed total can
    /// never drift from the rows next to it.
    pub fn published_totals(&self) -> CoverageTotals {
        let mut totals = CoverageTotals::default();
        for row in &self.jurisdictions {
            add(&mut totals.schools, row.schools);
            add(&mut totals.athletes, row.athletes);
            add(&mut totals.coaches, row.coaches);
            add(&mut totals.meets, row.meets);
            add(&mut totals.performances, row.performances);
        }
        totals
    }

    /// Fail loudly when the published rows do not sum to what the store was read for: a report whose
    /// rows lose, double-count or invent a row is a bug, not a finding.
    pub fn reconcile(&self) -> ReportResult<()> {
        let published = self.published_totals();
        if published == self.read {
            return Ok(());
        }
        Err(ReportError::Invariant {
            detail: format!(
                "coverage reconciliation failed: read {} but the rows publish {}",
                totals_text(&self.read),
                totals_text(&published)
            ),
        })
    }

    /// The reconciliation as printed lines: the cohort, what was read, what the rows publish, and
    /// whether the two agree. A sheet or a log prints these instead of assuming the totals match.
    pub fn reconciliation_lines(&self) -> Vec<String> {
        let published = self.published_totals();
        let verdict = if published == self.read {
            "matches"
        } else {
            "MISMATCH"
        };
        let cohort = self
            .grad_year
            .map_or_else(|| "all".to_string(), |year| year.to_string());
        vec![
            format!(
                "coverage cohort: grad_year={cohort} off_cohort_athletes={}",
                self.off_cohort_athletes
            ),
            format!("coverage read: {}", totals_text(&self.read)),
            format!("coverage published: {}", totals_text(&published)),
            format!("coverage reconciliation: {verdict}"),
        ]
    }
}

/// §49 coverage for the stored census, optionally restricted to one graduation year.
///
/// Every row comes from [`Store::scan`], which merges the append-only observations of a table into
/// one entity per id, so the report needs no consolidated snapshot. The cohort filter narrows which
/// athletes, performances and schools the rows count; jurisdictions, coaches and meets are read
/// whole, and the athletes a filter excludes are published as
/// [`CoverageReport::off_cohort_athletes`] rather than silently dropped.
pub fn coverage_report(store: &Store, grad_year: Option<i16>) -> ReportResult<CoverageReport> {
    let outcome = classify::run(store, grad_year)?;
    let report = CoverageReport {
        grad_year,
        notes: coverage_notes(store, grad_year, &outcome),
        jurisdictions: outcome.jurisdictions,
        gaps: outcome.gaps,
        read: outcome.read,
        off_cohort_athletes: outcome.off_cohort_athletes,
    };
    report.reconcile()?;
    Ok(report)
}

/// Saturating accumulation of one published row's counter into the totals.
fn add(total: &mut usize, value: usize) {
    *total = total.saturating_add(value);
}

/// One totals line, in declaration order, so the reconciliation prints the same shape everywhere.
fn totals_text(totals: &CoverageTotals) -> String {
    format!(
        "schools={} athletes={} coaches={} meets={} performances={}",
        totals.schools, totals.athletes, totals.coaches, totals.meets, totals.performances
    )
}

/// Provenance notes for `report.json`, in the shape the census notes already use.
fn coverage_notes(store: &Store, grad_year: Option<i16>, outcome: &Outcome) -> Vec<String> {
    let cohort = grad_year.map_or_else(|| "all".to_string(), |year| year.to_string());
    let mut notes = vec![format!(
        "coverage grad_year={cohort} athletes={} jurisdictions={} from {}",
        outcome.read.athletes,
        outcome.jurisdictions.len(),
        store.root().display()
    )];
    if outcome.read.athletes == 0 {
        notes.push(
            "no cohort athletes stored yet — run `collect` then `consolidate` before `report`"
                .to_string(),
        );
    }
    if outcome.off_cohort_athletes > 0 {
        notes.push(format!(
            "{} stored athletes are outside grad_year={cohort} and publish as off_cohort_athletes",
            outcome.off_cohort_athletes
        ));
    }
    let unplaceable = outcome
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction == UNKNOWN_JURISDICTION)
        .map_or(0, |row| row.athletes);
    if unplaceable > 0 {
        notes.push(format!(
            "{unplaceable} cohort athletes carry no placeable jurisdiction and publish in the {UNKNOWN_JURISDICTION} row"
        ));
    }
    notes
}
