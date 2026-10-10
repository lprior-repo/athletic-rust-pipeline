use crate::bests::{self, SharedSelection};
use crate::export::ExportDataset;
use crate::report::{Derivation, ReportResult};
use crate::workbook::Options;
use census_domain::model::SchoolYear;
use std::collections::{BTreeMap, BTreeSet};

use super::labels;
use crate::workbook::recruiting::contact::{self, attach_research, SchoolContacts};

pub(super) const DATA_ROWS_PER_SHEET: usize = 1_000_000;

#[derive(Debug, Default)]
pub(super) struct Tally {
    pub(super) performances: usize,
    pub(super) meets: BTreeSet<String>,
}

#[derive(Debug, Default)]
pub(super) struct School {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) state: String,
    pub(super) city: String,
    pub(super) athletics_url: Option<String>,
}

pub(super) struct Expectations<'a> {
    pub(super) dataset: &'a ExportDataset,
    pub(super) school_year: SchoolYear,
    pub(super) derivation: Derivation<'a>,
    pub(super) bests: Vec<SharedSelection>,
    pub(super) contacts: BTreeMap<String, SchoolContacts>,
    pub(super) school_address: BTreeMap<String, String>,
    schools: BTreeMap<String, School>,
    coach_spellings: BTreeMap<String, String>,
    pr_index: BTreeMap<String, Vec<usize>>,
    tallies: BTreeMap<String, Tally>,
}

impl<'a> Expectations<'a> {
    pub(super) fn of(dataset: &'a ExportDataset, options: &Options) -> ReportResult<Self> {
        let school_year = options.school_year;
        let derivation = Derivation::of(dataset, options.scope, options.grad_year);
        let school_address = super::postal::athlete_address_index(dataset, derivation.athletes())?;
        let bests = selected_bests(dataset, options);
        let schools = schools(derivation.schools());
        let pr_index = index(&bests);
        let tallies = tallies(&derivation);
        let mut contacts = contact::contacts(derivation.coach_observations(), school_year);
        attach_research(&mut contacts, derivation.schools(), school_year);
        let coach_spellings =
            crate::workbook::recruiting::coach_spelling::spellings(derivation.coach_observations());
        Ok(Self {
            dataset,
            school_year,
            derivation,
            bests,
            contacts,
            school_address,
            schools,
            coach_spellings,
            pr_index,
            tallies,
        })
    }

    pub(super) fn school(&self, id: &str) -> Option<&School> {
        self.schools.get(id)
    }

    pub(super) fn school_state(&self, id: &str) -> &str {
        self.school(id).map_or("", |school| school.state.as_str())
    }

    pub(super) fn school_name(&self, id: &str) -> &str {
        self.school(id).map_or("", |school| school.name.as_str())
    }

    pub(super) fn school_city(&self, id: &str) -> &str {
        self.school(id).map_or("", |school| school.city.as_str())
    }

    pub(super) fn school_id<'b>(&'b self, id: &'b str) -> &'b str {
        self.school(id).map_or(id, |school| school.id.as_str())
    }

    pub(super) fn published_coach_name<'b>(&'b self, id: &str, fallback: &'b str) -> &'b str {
        self.coach_spellings
            .get(id)
            .map_or(fallback, String::as_str)
    }

    pub(super) fn athletics_url(&self, id: &str) -> Option<&str> {
        self.school(id)
            .and_then(|school| school.athletics_url.as_deref())
    }

    pub(super) fn tally(&self, athlete: &str) -> Option<&Tally> {
        self.tallies.get(athlete)
    }

    pub(super) fn prs_of(&self, athlete: &str) -> impl Iterator<Item = &SharedSelection> {
        self.pr_index
            .get(athlete)
            .into_iter()
            .flatten()
            .filter_map(|index| self.bests.get(*index))
    }

    pub(super) fn sheet_names(&self) -> Vec<String> {
        let mut names = vec![labels::ATHLETES.to_string(), labels::PRS.to_string()];
        names.extend(performance_sheet_names(
            self.derivation.performances().len(),
        ));
        names.push(labels::COACHES.to_string());
        names.extend(labels::META_SHEETS.iter().map(|name| (*name).to_string()));
        names
    }
}

fn selected_bests(dataset: &ExportDataset, options: &Options) -> Vec<SharedSelection> {
    bests::build_from_dataset(
        dataset,
        &bests::Options {
            scope: options.scope,
            grad_year: options.grad_year,
            limit: options.limit,
        },
    )
}

pub(super) fn performance_sheet_names(count: usize) -> Vec<String> {
    let sheets = count.div_ceil(DATA_ROWS_PER_SHEET).max(1);
    (0..sheets)
        .map(|index| format!("Performances_{:03}", index.saturating_add(1)))
        .collect()
}

fn schools(rows: &[census_domain::model::CanonicalSchool]) -> BTreeMap<String, School> {
    let mut schools: BTreeMap<String, School> = BTreeMap::new();
    for school in rows {
        schools.insert(
            school.id.as_str().to_string(),
            School {
                id: school.id.as_str().to_string(),
                name: school.name.clone(),
                state: school
                    .state
                    .map(|state| state.to_string())
                    .map_or(Default::default(), core::convert::identity),
                city: school
                    .city
                    .clone()
                    .map_or(Default::default(), core::convert::identity),
                athletics_url: school.athletics_website.clone(),
            },
        );
    }
    schools
}

fn index(prs: &[SharedSelection]) -> BTreeMap<String, Vec<usize>> {
    let mut index: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (position, pr) in prs.iter().enumerate() {
        index
            .entry(pr.athlete_id().as_str().to_string())
            .or_default()
            .push(position);
    }
    index
}

fn tallies(derivation: &Derivation<'_>) -> BTreeMap<String, Tally> {
    let cohort: BTreeSet<&str> = derivation
        .athletes()
        .iter()
        .map(|athlete| athlete.id.as_str())
        .collect();
    let mut tallies: BTreeMap<String, Tally> = cohort
        .iter()
        .map(|athlete| ((*athlete).to_string(), Tally::default()))
        .collect();
    tally_performances(&mut tallies, &cohort, derivation);
    tallies
}

fn tally_performances(
    tallies: &mut BTreeMap<String, Tally>,
    cohort: &BTreeSet<&str>,
    derivation: &Derivation<'_>,
) {
    for performance in derivation.performances() {
        let subject = performance.athlete.as_str();
        let athlete = derivation
            .athlete_aliases()
            .get(subject)
            .map_or(subject, String::as_str);
        if !cohort.contains(athlete) {
            continue;
        }
        let Some(tally) = tallies.get_mut(athlete) else {
            continue;
        };
        tally.performances = tally.performances.saturating_add(1);
        tally.meets.insert(performance.meet.as_str().to_string());
    }
}
