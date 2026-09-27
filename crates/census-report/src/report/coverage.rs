use super::{ReportError, ReportResult};
use census_domain::JurisdictionBucket;
use census_store::Store;
use serde::Serialize;
use std::collections::BTreeMap;

mod athletes;
mod classify;
mod gaps;
mod reads;
mod state;

pub(crate) use state::{jurisdiction_of, school_state_index};

pub use gaps::{CoverageGap, GapClass};

use state::Outcome;

#[cfg(test)]
mod tests;

pub const UNKNOWN_JURISDICTION: &str = JurisdictionBucket::UNPLACED_CODE;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct CoverageTotals {
    pub schools: usize,
    pub athletes: usize,
    pub coaches: usize,
    pub meets: usize,
    pub performances: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct JurisdictionCoverage {
    pub jurisdiction: JurisdictionBucket,
    pub schools: usize,
    pub schools_with_athletes: usize,
    pub athletes: usize,
    pub athletes_core: usize,
    pub core_share_pct: u32,
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
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageReport {
    pub grad_year: Option<i16>,
    pub jurisdictions: Vec<JurisdictionCoverage>,
    pub gaps: Vec<CoverageGap>,
    pub read: CoverageTotals,
    pub off_cohort_athletes: usize,
    pub notes: Vec<String>,
}

impl CoverageReport {
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

fn add(total: &mut usize, value: usize) {
    *total = total.saturating_add(value);
}

fn totals_text(totals: &CoverageTotals) -> String {
    format!(
        "schools={} athletes={} coaches={} meets={} performances={}",
        totals.schools, totals.athletes, totals.coaches, totals.meets, totals.performances
    )
}

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
    if outcome.outside_scope != CoverageTotals::default() {
        notes.push(format!(
            "stored rows outside the census run scope (the 48 continental states plus DC, ADR-009) \
             are excluded from `coverage read` and published in no row: {}",
            totals_text(&outcome.outside_scope)
        ));
    }
    let unplaceable = outcome
        .jurisdictions
        .iter()
        .find(|row| row.jurisdiction == JurisdictionBucket::Unplaced)
        .map_or(0, |row| row.athletes);
    if unplaceable > 0 {
        notes.push(format!(
            "{unplaceable} cohort athletes carry no placeable jurisdiction and publish in the {UNKNOWN_JURISDICTION} row"
        ));
    }
    notes
}
