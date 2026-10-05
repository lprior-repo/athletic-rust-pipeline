use crate::workbook::meta::Family;
use census_domain::model::{CanonicalAthlete, CanonicalMeet, CanonicalSchool, ReviewVerdictRecord};
use census_domain::UsJurisdiction;
use census_review::ReviewFamily;
use std::collections::HashMap;

use super::{header, Expect, Sheet};

const CONFLICT_HEADERS: [&str; 5] = ["Family", "State", "Subject ID", "Subject", "Detail"];

const REVIEW_HEADERS: [&str; 7] = [
    "Family",
    "State",
    "Subject ID",
    "Subject",
    "Answer",
    "Confidence",
    "Detail",
];

pub(super) struct Lookup<'a> {
    schools: HashMap<&'a str, &'a CanonicalSchool>,
    meets: HashMap<&'a str, &'a CanonicalMeet>,
    athletes: HashMap<&'a str, &'a CanonicalAthlete>,
}

impl<'a> Lookup<'a> {
    pub(super) fn of(
        schools: &'a [CanonicalSchool],
        meets: &'a [CanonicalMeet],
        athletes: &'a [CanonicalAthlete],
    ) -> Self {
        Self {
            schools: schools
                .iter()
                .map(|school| (school.id.as_str(), school))
                .collect(),
            meets: meets.iter().map(|meet| (meet.id.as_str(), meet)).collect(),
            athletes: athletes
                .iter()
                .map(|athlete| (athlete.id.as_str(), athlete))
                .collect(),
        }
    }

    fn school(&self, id: &str) -> Option<&'a CanonicalSchool> {
        self.schools.get(id).copied()
    }

    fn meet(&self, id: &str) -> Option<&'a CanonicalMeet> {
        self.meets.get(id).copied()
    }

    fn athlete(&self, id: &str) -> Option<&'a CanonicalAthlete> {
        self.athletes.get(id).copied()
    }

    fn school_state(&self, id: &str) -> Option<UsJurisdiction> {
        self.school(id).and_then(|school| school.state)
    }

    fn meet_state(&self, id: &str) -> Option<UsJurisdiction> {
        self.meet(id).and_then(|meet| meet.state)
    }

    fn athlete_state(&self, id: &str) -> Option<UsJurisdiction> {
        self.athlete(id)
            .and_then(|athlete| self.school_state(athlete.school.as_str()))
    }

    fn state(&self, id: &str) -> Option<UsJurisdiction> {
        self.school_state(id).or_else(|| self.athlete_state(id))
    }

    fn subject(&self, id: &str) -> String {
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

pub(super) fn conflicts(families: &[Family], index: &Lookup<'_>) -> Sheet {
    let mut rows = vec![header(&CONFLICT_HEADERS)];
    for family in families {
        for retained in &family.rows {
            rows.push(vec![
                Expect::text(family.label),
                state_cell(index.state(&retained.subject_id)),
                Expect::text(&retained.subject_id),
                Expect::text(&retained.subject),
                Expect::text(&retained.detail),
            ]);
        }
    }
    ("Conflicts", rows)
}

pub(super) fn review(
    families: &[Family],
    verdicts: &[ReviewVerdictRecord],
    index: &Lookup<'_>,
) -> Sheet {
    let mut rows = vec![header(&REVIEW_HEADERS)];
    for family in families {
        for retained in &family.rows {
            rows.push(vec![
                Expect::text(family.label),
                family_state(family.label, &retained.subject_id, index),
                Expect::text(&retained.subject_id),
                Expect::text(&retained.subject),
                Expect::Empty,
                Expect::Empty,
                Expect::text(&retained.detail),
            ]);
        }
    }
    rows.extend(verdicts.iter().map(|verdict| verdict_row(verdict, index)));
    ("Review", rows)
}

fn verdict_row(verdict: &ReviewVerdictRecord, index: &Lookup<'_>) -> Vec<Expect> {
    let family = ReviewFamily::parse(&verdict.family).map_or_else(
        || verdict.family.clone(),
        |family| family.label().to_string(),
    );
    let answer = match (verdict.field.is_empty(), verdict.value.is_empty()) {
        (false, false) => format!("{}={}", verdict.field, verdict.value),
        _ => String::new(),
    };
    vec![
        Expect::text(family),
        family_state(&verdict.family, &verdict.subject_id, index),
        Expect::text(&verdict.subject_id),
        Expect::text(verdict_subject(verdict, index)),
        Expect::text(answer),
        Expect::Number(f64::from(verdict.confidence)),
        Expect::text(&verdict.rationale),
    ]
}

fn verdict_subject(verdict: &ReviewVerdictRecord, index: &Lookup<'_>) -> String {
    match ReviewFamily::parse(&verdict.family) {
        Some(ReviewFamily::SchoolJurisdiction | ReviewFamily::SchoolLink) => index
            .school(&verdict.subject_id)
            .map_or_else(|| verdict.subject_id.clone(), |school| school.name.clone()),
        Some(ReviewFamily::MeetJurisdiction) => index
            .meet(&verdict.subject_id)
            .map_or_else(|| verdict.subject_id.clone(), |meet| meet.name.clone()),
        Some(ReviewFamily::AthleteIdentity) => index.subject(&verdict.subject_id),
        None => verdict.subject_id.clone(),
    }
}

fn family_state(family: &str, subject_id: &str, index: &Lookup<'_>) -> Expect {
    let state = match ReviewFamily::parse(family) {
        Some(ReviewFamily::SchoolJurisdiction | ReviewFamily::SchoolLink) => {
            index.school_state(subject_id)
        }
        Some(ReviewFamily::MeetJurisdiction) => index.meet_state(subject_id),
        Some(ReviewFamily::AthleteIdentity) => index.athlete_state(subject_id),
        None => None,
    };
    state_cell(state)
}

fn state_cell(state: Option<UsJurisdiction>) -> Expect {
    state.map_or(Expect::Empty, |state| Expect::text(state.code()))
}

fn subject_of(name: &str, school: Option<&str>) -> String {
    match school {
        Some(school) => format!("{name} ({school})"),
        None => name.to_string(),
    }
}
