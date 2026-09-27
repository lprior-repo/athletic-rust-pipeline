use crate::bests::SharedSelection;
use crate::report::{
    exclude_out_of_scope, in_run_scope, jurisdiction_of, retain_core, school_state_index, Census,
    ReportResult, Scope,
};
use census_domain::{
    model::{
        CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool, ReviewVerdictRecord,
    },
    JurisdictionBucket,
};
use census_store::{Store, Table};
use rust_xlsxwriter::Workbook;
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use super::cells::{write_sheet, Cell};
use super::performances::PerformanceSheetPopulation;

mod coverage;
mod inventory;
mod metrics;
pub mod queues;
mod schools;
mod sources;

use coverage::{coverage_sheet, COVERAGE_WIDTHS};
use inventory::{meets_sheet, MEET_WIDTHS};
use metrics::{metrics_sheet, METRIC_WIDTHS};
use queues::{
    conflict_families, conflicts_sheet, review_families, review_sheet, CONFLICT_WIDTHS,
    REVIEW_WIDTHS,
};

pub use queues::retained_records;
use schools::{schools_sheet, SCHOOL_WIDTHS};
use sources::{sources_sheet, SOURCE_WIDTHS};

type Sheet = (&'static str, Vec<Vec<Cell>>, &'static [u16], bool);

pub(super) struct RunFacts<'a> {
    pub(super) store: &'a Store,
    pub(super) core: &'a Census,
    pub(super) all_sources: &'a Census,
    pub(super) bests: &'a [SharedSelection],
    pub(super) scope: Scope,
    pub(super) school_year: census_domain::model::SchoolYear,
    pub(super) perf_population: PerformanceSheetPopulation,
}

pub(super) fn write_meta_sheets(
    book: &mut Workbook,
    path: &Path,
    facts: RunFacts<'_>,
) -> ReportResult<()> {
    let rows = StoreRows::read(facts.store, facts.scope, facts.school_year)?;
    let names = school_name_index(&rows.schools);
    let conflicts = conflict_families(&rows, &names);
    let review = review_families(&rows, &names)?;
    let metrics = metrics_sheet(&facts, &rows, &conflicts)?;
    let sheets: [Sheet; 7] = [
        (
            "Schools",
            schools_sheet(&rows.schools)?,
            &SCHOOL_WIDTHS,
            true,
        ),
        ("Meets", meets_sheet(&rows.meets), &MEET_WIDTHS, true),
        (
            "Sources",
            sources_sheet(facts.all_sources)?,
            &SOURCE_WIDTHS,
            true,
        ),
        (
            "Coverage",
            coverage_sheet(facts.store)?,
            &COVERAGE_WIDTHS,
            true,
        ),
        (
            "Conflicts",
            conflicts_sheet(&conflicts, &rows),
            &CONFLICT_WIDTHS,
            true,
        ),
        ("Review", review_sheet(&review, &rows), &REVIEW_WIDTHS, true),
        ("Run Metrics", metrics, &METRIC_WIDTHS, false),
    ];
    for (name, cells, widths, autofilter) in sheets {
        write_sheet(book, path, name, cells, widths, autofilter)?;
    }
    Ok(())
}

struct StoreRows {
    schools: Vec<CanonicalSchool>,
    meets: Vec<CanonicalMeet>,
    athletes: Vec<CanonicalAthlete>,
    identities: census_domain::model::AthleteIdentityProjection,
    school_year: census_domain::model::SchoolYear,
    coaches: Vec<CanonicalCoach>,
    verdicts: Vec<ReviewVerdictRecord>,
}
impl StoreRows {
    fn read(
        store: &Store,
        scope: Scope,
        school_year: census_domain::model::SchoolYear,
    ) -> ReportResult<Self> {
        let snapshot = store.snapshot();
        let mut schools: Vec<CanonicalSchool> = snapshot.scan(Table::Schools)?;
        let outside_schools = exclude_out_of_scope(&mut schools, |school| school.state.into());
        let mut meets: Vec<CanonicalMeet> = snapshot.scan(Table::Meets)?;
        meets.retain(|m| in_run_scope(JurisdictionBucket::from(m.state)));
        let mut athletes: Vec<CanonicalAthlete> = snapshot.scan(Table::Athletes)?;
        let mut index = census_domain::model::AthleteIdentityIndex::default();
        for athlete in &athletes {
            index
                .observe(athlete)
                .map_err(census_store::StoreError::from)?;
        }
        let identities = snapshot.project_athlete_identities(index)?;
        let mut school_state = school_state_index(&schools);
        school_state.extend(school_state_index(&outside_schools));
        athletes.retain(|a| in_run_scope(jurisdiction_of(&school_state, a.school.as_str())));
        if scope == Scope::Core {
            retain_core(&mut meets);
            retain_core(&mut athletes);
        }
        let mut coaches = super::recruiting::coach_observations(&snapshot)?;
        coaches.retain(|c| in_run_scope(jurisdiction_of(&school_state, c.school.as_str())));
        let verdicts: Vec<ReviewVerdictRecord> = snapshot.scan(Table::IdentityVerdicts)?;
        Ok(Self {
            schools,
            meets,
            athletes,
            identities,
            school_year,
            coaches,
            verdicts,
        })
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
