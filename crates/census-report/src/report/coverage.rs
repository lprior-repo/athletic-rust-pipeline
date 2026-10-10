use super::{ReportError, ReportResult, Scope};
use crate::export::ExportDataset;
use census_domain::JurisdictionBucket;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};

mod athletes;
mod classify;
mod gaps;
mod reads;
mod state;

pub(crate) use state::{in_requested_year, jurisdiction_of, school_state_index};

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

pub fn coverage_report(
    dataset: &ExportDataset,
    grad_year: Option<i16>,
) -> ReportResult<CoverageReport> {
    let outcome = classify::run(dataset, grad_year);
    let mut report = CoverageReport {
        grad_year,
        notes: coverage_notes(dataset, grad_year, &outcome),
        jurisdictions: outcome.jurisdictions,
        gaps: outcome.gaps,
        read: outcome.read,
        off_cohort_athletes: outcome.off_cohort_athletes,
    };
    apply_pr_support(&mut report, dataset, grad_year);
    report.reconcile()?;
    Ok(report)
}

fn apply_pr_support(report: &mut CoverageReport, dataset: &ExportDataset, grad_year: Option<i16>) {
    let bests = crate::bests::build_from_dataset(
        dataset,
        &crate::bests::Options {
            scope: Scope::AllSources,
            grad_year,
            limit: None,
        },
    );
    let supported: HashSet<&str> = bests
        .iter()
        .map(|selection| selection.athlete_id().as_str())
        .collect();
    tally_pr_support(report, dataset, grad_year, &supported);
    write_pr_gaps(report);
}

fn tally_pr_support(
    report: &mut CoverageReport,
    dataset: &ExportDataset,
    grad_year: Option<i16>,
    supported: &HashSet<&str>,
) {
    let school_state = state::school_state_index(dataset.schools.values());
    let athletes =
        super::derivation::collapse_athletes(&dataset.athletes, &dataset.canonical_aliases);
    for athlete in &athletes {
        if !state::in_requested_year(athlete, grad_year) {
            continue;
        }
        if !supported.contains(athlete.id.as_str()) {
            continue;
        }
        let bucket = state::jurisdiction_of(&school_state, athlete.school.as_str());
        bump_pr_support(report, bucket);
    }
}

fn bump_pr_support(report: &mut CoverageReport, bucket: JurisdictionBucket) {
    if let Some(row) = report
        .jurisdictions
        .iter_mut()
        .find(|row| row.jurisdiction == bucket)
    {
        row.with_pr_support = row.with_pr_support.saturating_add(1);
    }
}

fn write_pr_gaps(report: &mut CoverageReport) {
    for row in &report.jurisdictions {
        let count = row.with_performance.saturating_sub(row.with_pr_support);
        match report.gaps.iter_mut().find(|gap| {
            gap.jurisdiction == row.jurisdiction && gap.class == GapClass::MissingPrSupport
        }) {
            Some(gap) => gap.count = count,
            None if count > 0 => report.gaps.push(CoverageGap {
                jurisdiction: row.jurisdiction,
                class: GapClass::MissingPrSupport,
                unit: GapClass::MissingPrSupport.unit(),
                count,
            }),
            None => {}
        }
    }
    report
        .gaps
        .retain(|gap| gap.class != GapClass::MissingPrSupport || gap.count > 0);
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

fn coverage_notes(
    dataset: &ExportDataset,
    grad_year: Option<i16>,
    outcome: &Outcome,
) -> Vec<String> {
    let cohort = grad_year.map_or_else(|| "all".to_string(), |year| year.to_string());
    let mut notes = vec![format!(
        "coverage grad_year={cohort} athletes={} jurisdictions={} from {}",
        outcome.read.athletes,
        outcome.jurisdictions.len(),
        dataset.lineage.store_root
    )];
    append_population_notes(&mut notes, outcome, &cohort);
    append_placement_notes(&mut notes, outcome);
    notes
}

fn append_population_notes(notes: &mut Vec<String>, outcome: &Outcome, cohort: &str) {
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
}

fn append_placement_notes(notes: &mut Vec<String>, outcome: &Outcome) {
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
}
