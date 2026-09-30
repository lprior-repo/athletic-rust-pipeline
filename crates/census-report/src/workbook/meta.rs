use crate::bests::SharedSelection;
use crate::report::{Census, Derivation, ReportResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool, ReviewVerdictRecord,
};
use census_domain::UsJurisdiction;
use rust_xlsxwriter::Workbook;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use super::cells::{write_sheet, Cell};
use super::millis;
use std::time::Instant;

mod coverage;
mod inventory;
mod metrics;
pub mod queues;
mod schools;

use coverage::{coverage_sheet, COVERAGE_WIDTHS};
use inventory::{meets_sheet, MEET_WIDTHS};
use metrics::{metrics_sheet, METRIC_WIDTHS};
use queues::{
    cohort_of, conflict_families, conflicts_sheet, review_families, review_sheet, CONFLICT_WIDTHS,
    REVIEW_WIDTHS,
};

pub use queues::retained_records;
use schools::{schools_sheet, SCHOOL_WIDTHS};

type Sheet = (&'static str, Vec<Vec<Cell>>, &'static [u16], bool);

pub(super) struct RunFacts<'a> {
    pub(super) population: &'a Derivation<'a>,
    pub(super) store: &'a census_store::Store,
    pub(super) core: &'a Census,
    pub(super) all_sources: &'a Census,
    pub(super) bests: &'a [SharedSelection],
    pub(super) school_year: census_domain::model::SchoolYear,
}

pub(super) fn write_meta_sheets(
    book: &mut Workbook,
    path: &Path,
    facts: RunFacts<'_>,
) -> ReportResult<()> {
    let started = Instant::now();
    let rows = StoreRows::of(facts.population, facts.school_year)?;
    let cohort = cohort_of(facts.population.dataset(), facts.population.scope());
    let names = school_name_index(&rows.schools);
    tracing::info!(
        step = "meta_rows",
        ms = millis(started),
        "workbook build step"
    );
    let started = Instant::now();
    let conflicts = conflict_families(&rows, &cohort, &names);
    tracing::info!(
        step = "conflicts",
        ms = millis(started),
        "workbook build step"
    );
    let started = Instant::now();
    let review = review_families(&rows, &cohort, &names)?;
    tracing::info!(step = "review", ms = millis(started), "workbook build step");
    let started = Instant::now();
    let metrics = metrics_sheet(&facts, &rows, &conflicts)?;
    tracing::info!(
        step = "metrics",
        ms = millis(started),
        "workbook build step"
    );
    let index = SubjectIndex::of(&rows);
    let mut sheets: Vec<Sheet> = Vec::with_capacity(6);
    let started = Instant::now();
    sheets.push((
        "Schools",
        schools_sheet(&rows.schools)?,
        &SCHOOL_WIDTHS,
        true,
    ));
    tracing::info!(
        sheet = "Schools",
        ms = millis(started),
        "workbook sheet built"
    );
    let started = Instant::now();
    sheets.push(("Meets", meets_sheet(&rows.meets), &MEET_WIDTHS, true));
    tracing::info!(
        sheet = "Meets",
        ms = millis(started),
        "workbook sheet built"
    );
    let started = Instant::now();
    sheets.push((
        "Coverage",
        coverage_sheet(facts.population.dataset())?,
        &COVERAGE_WIDTHS,
        true,
    ));
    tracing::info!(
        sheet = "Coverage",
        ms = millis(started),
        "workbook sheet built"
    );
    let started = Instant::now();
    sheets.push((
        "Conflicts",
        conflicts_sheet(&conflicts, &index),
        &CONFLICT_WIDTHS,
        true,
    ));
    tracing::info!(
        sheet = "Conflicts",
        ms = millis(started),
        "workbook sheet built"
    );
    let started = Instant::now();
    sheets.push((
        "Review",
        review_sheet(&review, &rows, &index),
        &REVIEW_WIDTHS,
        true,
    ));
    tracing::info!(
        sheet = "Review",
        ms = millis(started),
        "workbook sheet built"
    );
    let started = Instant::now();
    sheets.push(("Run Metrics", metrics, &METRIC_WIDTHS, false));
    tracing::info!(
        sheet = "Run Metrics",
        ms = millis(started),
        "workbook sheet built"
    );
    for (name, cells, widths, autofilter) in sheets {
        let count = cells.len();
        let started = Instant::now();
        write_sheet(book, path, name, cells, widths, autofilter)?;
        tracing::info!(
            sheet = name,
            rows = count,
            ms = millis(started),
            "workbook sheet written"
        );
    }
    Ok(())
}

struct StoreRows<'d> {
    schools: &'d [CanonicalSchool],
    meets: &'d [CanonicalMeet],
    athletes: &'d [CanonicalAthlete],
    identities: std::sync::Arc<census_domain::model::AthleteIdentityProjection>,
    school_year: census_domain::model::SchoolYear,
    coaches: &'d [CanonicalCoach],
    verdicts: &'d [ReviewVerdictRecord],
}
impl<'d> StoreRows<'d> {
    fn of(
        derivation: &'d Derivation<'_>,
        school_year: census_domain::model::SchoolYear,
    ) -> ReportResult<Self> {
        Ok(Self {
            schools: derivation.schools(),
            meets: derivation.meets(),
            athletes: derivation.athletes(),
            identities: derivation.dataset().identities(),
            school_year,
            coaches: derivation.coaches(),
            verdicts: &derivation.dataset().verdicts,
        })
    }
}

pub(super) struct SubjectIndex<'a> {
    schools: HashMap<&'a str, &'a CanonicalSchool>,
    meets: HashMap<&'a str, &'a CanonicalMeet>,
    athletes: HashMap<&'a str, &'a CanonicalAthlete>,
}

impl<'a> SubjectIndex<'a> {
    fn of(rows: &'a StoreRows<'a>) -> Self {
        Self {
            schools: rows
                .schools
                .iter()
                .map(|school| (school.id.as_str(), school))
                .collect(),
            meets: rows
                .meets
                .iter()
                .map(|meet| (meet.id.as_str(), meet))
                .collect(),
            athletes: rows
                .athletes
                .iter()
                .map(|athlete| (athlete.id.as_str(), athlete))
                .collect(),
        }
    }

    pub(super) fn school(&self, id: &str) -> Option<&'a CanonicalSchool> {
        self.schools.get(id).copied()
    }

    pub(super) fn meet(&self, id: &str) -> Option<&'a CanonicalMeet> {
        self.meets.get(id).copied()
    }

    pub(super) fn athlete(&self, id: &str) -> Option<&'a CanonicalAthlete> {
        self.athletes.get(id).copied()
    }

    pub(super) fn school_state(&self, id: &str) -> Option<UsJurisdiction> {
        self.school(id).and_then(|school| school.state)
    }

    pub(super) fn meet_state(&self, id: &str) -> Option<UsJurisdiction> {
        self.meet(id).and_then(|meet| meet.state)
    }

    pub(super) fn athlete_state(&self, id: &str) -> Option<UsJurisdiction> {
        self.athlete(id)
            .and_then(|athlete| self.school_state(athlete.school.as_str()))
    }

    pub(super) fn state(&self, id: &str) -> Option<UsJurisdiction> {
        self.school_state(id).or_else(|| self.athlete_state(id))
    }

    pub(super) fn subject(&self, id: &str) -> String {
        self.athlete(id).map_or_else(
            || id.to_string(),
            |athlete| {
                subject_of(
                    &athlete.canonical_name,
                    self.school(athlete.school.as_str())
                        .map(|school| school.name.as_str()),
                )
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueRow {
    pub subject_id: String,
    pub subject: String,
    pub detail: String,
}

struct Family {
    label: &'static str,
    findings: usize,
    rows: Vec<QueueRow>,
}

impl Family {
    fn new(label: &'static str) -> Self {
        Self {
            label,
            findings: 0,
            rows: Vec::new(),
        }
    }

    fn push(&mut self, row: QueueRow) {
        self.findings = self.findings.saturating_add(1);
        self.rows.push(row);
    }

    fn group(&mut self, rows: Vec<QueueRow>) {
        self.findings = self.findings.saturating_add(1);
        self.rows.extend(rows);
    }
}

fn school_name_index(schools: &[CanonicalSchool]) -> HashMap<&str, &str> {
    schools
        .iter()
        .map(|school| (school.id.as_str(), school.name.as_str()))
        .collect()
}

fn school_of<'a>(names: &HashMap<&'a str, &'a str>, school: &str) -> Option<&'a str> {
    names.get(school).copied()
}

fn subject_of(name: &str, school: Option<&str>) -> String {
    match school {
        Some(school) => format!("{name} ({school})"),
        None => name.to_string(),
    }
}

fn sorted_counts(counts: &BTreeMap<String, usize>) -> Vec<(&String, &usize)> {
    let mut ordered: Vec<(&String, &usize)> = counts.iter().collect();
    ordered.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    ordered
}

#[cfg(test)]
mod tests;
