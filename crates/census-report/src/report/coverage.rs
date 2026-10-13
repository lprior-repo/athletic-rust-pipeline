use super::{ReportError, ReportResult, Scope};
use crate::export::ExportDataset;
use census_domain::JurisdictionBucket;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};

mod athletes;
mod classify;
mod contact;
mod gaps;
mod population;
mod reads;
mod state;

pub(crate) use state::{in_requested_year, jurisdiction_of, school_state_index};

pub use contact::ContactCounts;
pub use gaps::{CoverageGap, GapClass};

use state::Outcome;

#[cfg(test)]
mod tests;

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

impl CoverageReport {
    pub fn publish_coverage_rows(
        store: &census_store::Store,
        evidence: &[census_domain::model::CoverageRow],
    ) -> ReportResult<usize> {
        population::publish_coverage_rows(store, evidence)
    }

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
        self.reconcile_lifecycle()?;
        self.reconcile_contact()?;
        self.reconcile_totals()
    }

    fn reconcile_lifecycle(&self) -> ReportResult<()> {
        let published = self.published_totals();
        if published != self.read {
            return Err(ReportError::Invariant {
                detail: format!(
                    "coverage reconciliation failed: read {} but the rows publish {}",
                    totals_text(&self.read),
                    totals_text(&published)
                ),
            });
        }

        if self.raw_observations < self.scope_filtered {
            return Err(ReportError::Invariant {
                detail: format!(
                    "lifecycle reconciliation failed: raw_observations {} < scope_filtered {}",
                    self.raw_observations, self.scope_filtered
                ),
            });
        }
        if self.scope_filtered < self.accepted_cohort {
            return Err(ReportError::Invariant {
                detail: format!(
                    "lifecycle reconciliation failed: scope_filtered {} < accepted_cohort {}",
                    self.scope_filtered, self.accepted_cohort
                ),
            });
        }
        Ok(())
    }

    fn reconcile_contact(&self) -> ReportResult<()> {
        let contact_total = self
            .contact
            .reached
            .saturating_add(self.contact.failed)
            .saturating_add(self.contact.blocked);
        if self.contact.attempted < contact_total {
            return Err(ReportError::Invariant {
                detail: format!(
                    "contact reconciliation failed: attempted {} < reached+failed+blocked {}",
                    self.contact.attempted, contact_total
                ),
            });
        }
        if self.contact.terminal() > self.contact.attempted {
            return Err(ReportError::Invariant {
                detail: format!(
                    "contact reconciliation failed: terminal {} > attempted {}",
                    self.contact.terminal(),
                    self.contact.attempted
                ),
            });
        }
        Ok(())
    }

    fn reconcile_totals(&self) -> ReportResult<()> {
        let declared: usize = self.state_counts.values().sum();
        if declared != self.jurisdictions.len() {
            return Err(ReportError::Invariant {
                detail: format!(
                    "coverage state reconciliation failed: {} declared states for {} jurisdiction rows",
                    declared,
                    self.jurisdictions.len()
                ),
            });
        }
        for row in &self.jurisdictions {
            let state = state_slug(row.state);
            if !self.state_counts.contains_key(state) {
                return Err(ReportError::Invariant {
                    detail: format!(
                        "coverage state reconciliation failed: row {} is absent from the declared states",
                        row.jurisdiction.code()
                    ),
                });
            }
            row.source_population.reconcile()?;
        }
        self.population_totals()?;
        self.source_population.reconcile()?;
        Ok(())
    }

    fn population_totals(&self) -> ReportResult<()> {
        let global = [
            self.source_population.registered,
            self.source_population.applicable,
            self.source_population.attempted,
            self.source_population.captured,
            self.source_population.parsed,
            self.source_population.accepted,
            self.source_population.withheld,
        ];
        let mut sums = [0usize; 7];
        let mut measured = [true; 7];
        for row in &self.jurisdictions {
            let stages = [
                row.source_population.registered,
                row.source_population.applicable,
                row.source_population.attempted,
                row.source_population.captured,
                row.source_population.parsed,
                row.source_population.accepted,
                row.source_population.withheld,
            ];
            for ((sum, measured), value) in sums.iter_mut().zip(measured.iter_mut()).zip(stages) {
                match value {
                    Some(value) => *sum = sum.saturating_add(value),
                    None => *measured = false,
                }
            }
        }
        for ((total, sum), (measured, stage)) in global
            .into_iter()
            .zip(sums.iter())
            .zip(measured.iter().zip(population::STAGES.iter()))
        {
            if let Some(total) = total {
                if *measured && *sum > total {
                    return Err(ReportError::Invariant {
                        detail: format!(
                            "source population reconciliation failed: rows sum {sum} > published {total} for stage {stage}"
                        ),
                    });
                }
            }
        }
        Ok(())
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
    let outcome = classify::run(dataset, grad_year);
    let state_counts = state_histogram(&outcome.jurisdictions);
    let mut report = CoverageReport {
        grad_year,
        notes: coverage_notes(dataset, grad_year, &outcome),
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
    apply_pr_support(&mut report, dataset, grad_year);
    report.reconcile()?;
    Ok(report)
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
    append_lifecycle_notes(&mut notes, outcome);
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
             are excluded from `coverage read` and population coverage rows; retained historical \
             school/meet metadata may appear in selected athletes' results: {}",
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

fn append_lifecycle_notes(notes: &mut Vec<String>, outcome: &Outcome) {
    let population = &outcome.source_population;
    if let Some(registered) = population.registered {
        notes.push(format!(
            "source lifecycle: {registered} registered -> {} -> {} attempted -> {} captured -> {} parsed -> {} accepted -> {} withheld",
            stage_text(population.applicable),
            stage_text(population.attempted),
            stage_text(population.captured),
            stage_text(population.parsed),
            stage_text(population.accepted),
            stage_text(population.withheld)
        ));
    } else {
        notes.push(
            "source lifecycle: unmeasured — the run has published no CoverageRow population for this store"
                .to_string(),
        );
    }

    let contact = &outcome.contact;
    let contact_total = contact.attempted.saturating_add(contact.never_attempted);
    if contact_total > 0 {
        notes.push(format!(
            "contact attempts: {} attempted ({} reached, {} failed, {} blocked, {} successful-empty, {} accepted refusal, {} exhausted) + {} never attempted",
            contact.attempted,
            contact.reached,
            contact.failed,
            contact.blocked,
            contact.successful_empty,
            contact.accepted_refusal,
            contact.exhausted,
            contact.never_attempted
        ));
    }

    if outcome.raw_observations > 0 {
        notes.push(format!(
            "coverage denominators: {} raw observations -> {} scope-filtered -> {} accepted cohort",
            outcome.raw_observations, outcome.scope_filtered, outcome.accepted_cohort
        ));
    }

    if !outcome.terminal_failures.is_empty() {
        let failures: Vec<String> = outcome
            .terminal_failures
            .iter()
            .map(|(reason, count)| format!("{} ({})", reason, count))
            .collect();
        notes.push(format!("terminal failures: {}", failures.join(", ")));
    }
}

fn stage_text(stage: Option<usize>) -> String {
    stage.map_or_else(|| "unmeasured".to_string(), |value| value.to_string())
}
