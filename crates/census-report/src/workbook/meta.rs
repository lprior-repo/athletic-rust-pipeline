use crate::bests::SharedSelection;
use crate::report::{Census, Derivation, ReportResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool, ReviewCase,
    ReviewVerdictRecord,
};
use census_domain::UsJurisdiction;
use rust_xlsxwriter::Workbook;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use super::cells::{write_sheet, Cell, SheetLayout};
use super::millis;
use std::time::Instant;

mod coverage;
mod inventory;
pub(in crate::workbook) mod metrics;
pub mod queues;
mod schools;
mod sheets;
mod sources;

use metrics::metrics_sheet;
use queues::{cohort_of, conflict_families, review_families};
use sheets::meta_sheets;

pub use queues::retained_records;

type Sheet = (&'static str, Vec<Vec<Cell>>, &'static [u16], bool);

pub(super) struct RunFacts<'a> {
    pub(super) population: &'a Derivation<'a>,
    pub(super) recruiting: &'a Derivation<'a>,
    pub(super) core: &'a Census,
    pub(super) all_sources: &'a Census,
    pub(super) bests: &'a [SharedSelection],
    pub(super) school_year: census_domain::model::SchoolYear,
}

fn step<T>(name: &'static str, build: impl FnOnce() -> T) -> T {
    let started = Instant::now();
    let value = build();
    tracing::info!(step = name, ms = millis(started), "workbook build step");
    value
}

pub(super) fn write_meta_sheets(
    book: &mut Workbook,
    path: &Path,
    facts: RunFacts<'_>,
) -> ReportResult<()> {
    let rows = step("meta_rows", || {
        StoreRows::of(facts.population, facts.school_year)
    })?;
    let cohort = cohort_of(facts.population.dataset(), facts.population.scope());
    let names = school_name_index(rows.schools);
    let conflicts = step("conflicts", || conflict_families(&rows, &cohort, &names));
    let review = step("review", || review_families(&rows, &cohort, &names))?;
    let metrics = step("metrics", || metrics_sheet(&facts, &rows, &conflicts))?;
    let index = SubjectIndex::of(&rows);
    let inputs = sheets::Inputs::of(&facts, &rows, &conflicts, &review, &index);
    let sheets = meta_sheets(&inputs, metrics)?;
    for sheet in sheets {
        write_meta_sheet(book, path, sheet)?;
    }
    write_contact_sheets(book, path, &rows)?;
    Ok(())
}

fn write_meta_sheet(book: &mut Workbook, path: &Path, sheet: Sheet) -> ReportResult<()> {
    let (name, cells, widths, autofilter) = sheet;
    let count = cells.len();
    let started = Instant::now();
    write_sheet(
        book,
        path,
        SheetLayout {
            name,
            widths,
            autofilter,
        },
        cells,
    )?;
    tracing::info!(
        sheet = name,
        rows = count,
        ms = millis(started),
        "workbook sheet written"
    );
    Ok(())
}

fn write_contact_sheets(
    book: &mut Workbook,
    path: &Path,
    facts: &StoreRows<'_>,
) -> ReportResult<()> {
    let mailboxes = crate::school_contacts::mailbox_rows(facts.schools, facts.school_year)?;
    write_contact_sheet(
        book,
        path,
        crate::school_contacts::MAILBOX_SHEET,
        &mailboxes,
        &[40, 48, 24, 18, 40, 24, 64, 68, 28, 64],
    )?;
    let research = crate::school_contacts::research_rows(facts.schools, facts.school_year)?;
    write_contact_sheet(
        book,
        path,
        crate::school_contacts::RESEARCH_SHEET,
        &research,
        &[40, 48, 32, 18, 28, 64],
    )
}

fn write_contact_sheet(
    book: &mut Workbook,
    path: &Path,
    name: &str,
    rows: &[Vec<String>],
    widths: &[u16],
) -> ReportResult<()> {
    let mut writer = super::cells::SheetWriter::start(book, path, name, widths)?;
    for (index, row) in rows.iter().enumerate() {
        writer.write_strings(index, row)?;
    }
    let last_column =
        widths
            .len()
            .checked_sub(1)
            .ok_or_else(|| crate::report::ReportError::Invariant {
                detail: "contact worksheet has no header".to_string(),
            })?;
    writer.finish(rows.len(), last_column, true)
}

pub(in crate::workbook) struct StoreRows<'d> {
    schools: &'d [CanonicalSchool],
    meets: &'d [CanonicalMeet],
    athletes: &'d [CanonicalAthlete],
    identities: std::sync::Arc<census_domain::model::AthleteIdentityProjection>,
    school_year: census_domain::model::SchoolYear,
    coaches: &'d [CanonicalCoach],
    verdicts: &'d [ReviewVerdictRecord],
    review_cases: &'d [ReviewCase],
}
impl<'d> StoreRows<'d> {
    pub(in crate::workbook) fn of(
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
            review_cases: &derivation.dataset().review_cases,
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

pub(in crate::workbook) struct Family {
    pub(in crate::workbook) label: &'static str,
    pub(in crate::workbook) findings: usize,
    pub(in crate::workbook) rows: Vec<QueueRow>,
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

pub(in crate::workbook) fn school_name_index(schools: &[CanonicalSchool]) -> HashMap<&str, &str> {
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

pub(in crate::workbook) fn sorted_counts(
    counts: &BTreeMap<String, usize>,
) -> Vec<(&String, &usize)> {
    let mut ordered: Vec<(&String, &usize)> = counts.iter().collect();
    ordered.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    ordered
}

#[cfg(test)]
mod tests;
