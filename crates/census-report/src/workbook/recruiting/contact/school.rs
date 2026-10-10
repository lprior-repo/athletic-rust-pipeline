use std::collections::BTreeMap;

use census_domain::model::{
    CanonicalCoach, CanonicalSchool, CoachRole, ContactResearchOutcome, ContactResearchSubject,
    Gender, SchoolId, SchoolYear,
};

use super::heads::{individuals, HeadScope, Heads};
use super::{Named, Slot};

#[derive(Debug, Clone)]
pub(in crate::workbook) struct SchoolContacts {
    pub(super) school: SchoolId,
    pub(in crate::workbook::recruiting) heads: Heads,
    pub(super) assistants: Vec<Named>,
    pub(super) research: ContactResearchOutcome,
}

impl SchoolContacts {
    pub(in crate::workbook) fn director(&self) -> Option<&Named> {
        self.heads.resolve(Slot::Director, Gender::Mixed).named()
    }

    pub(in crate::workbook) fn coach_admitted(&self, coach: &CanonicalCoach) -> bool {
        match coach.role {
            CoachRole::AssistantCoach => self.assistants.iter().any(|named| {
                named
                    .source()
                    .is_some_and(|source| source.coach_id == coach.id)
            }),
            CoachRole::HeadCoach | CoachRole::AthleticDirector => role_slot(coach)
                .and_then(|(slot, side)| self.heads.resolve(slot, side).named())
                .is_some_and(|named| {
                    named
                        .source()
                        .is_some_and(|source| source.coach_id == coach.id)
                }),
            CoachRole::Unknown => false,
        }
    }
}

pub(in crate::workbook) fn contacts(
    coaches: &[CanonicalCoach],
    school_year: SchoolYear,
) -> BTreeMap<String, SchoolContacts> {
    let buckets = coaches.iter().fold(
        BTreeMap::<&SchoolId, Buckets<'_>>::new(),
        |mut buckets, coach| {
            buckets.entry(&coach.school).or_default().push(coach);
            buckets
        },
    );
    buckets
        .into_iter()
        .map(|(school, buckets)| {
            (
                school.as_str().to_owned(),
                buckets.contacts(school, school_year),
            )
        })
        .collect()
}

#[derive(Default)]
struct Buckets<'a> {
    heads: BTreeMap<(Slot, Gender), Vec<&'a CanonicalCoach>>,
    assistants: Vec<&'a CanonicalCoach>,
}

impl<'a> Buckets<'a> {
    fn push(&mut self, coach: &'a CanonicalCoach) {
        if coach.role == CoachRole::AssistantCoach {
            self.assistants.push(coach);
        } else if let Some(slot) = role_slot(coach) {
            self.heads.entry(slot).or_default().push(coach);
        }
    }

    fn contacts(&self, school: &SchoolId, school_year: SchoolYear) -> SchoolContacts {
        let heads = self
            .heads
            .iter()
            .fold(Heads::default(), |mut heads, ((slot, side), rows)| {
                let scope = HeadScope {
                    school: school.as_str(),
                    slot: *slot,
                    side: *side,
                    school_year,
                };
                heads.insert(scope, rows);
                heads
            });
        SchoolContacts {
            school: school.clone(),
            heads,
            assistants: individuals(&self.assistants, school_year),
            research: ContactResearchOutcome::Unattempted,
        }
    }
}

pub(in crate::workbook) fn attach_research(
    contacts: &mut BTreeMap<String, SchoolContacts>,
    schools: &[CanonicalSchool],
    school_year: SchoolYear,
) {
    for school in schools {
        let outcome = research_outcome(school, school_year);
        match contacts.entry(school.id.as_str().to_owned()) {
            std::collections::btree_map::Entry::Occupied(mut slot) => {
                slot.get_mut().research = outcome;
            }
            std::collections::btree_map::Entry::Vacant(slot)
                if outcome != ContactResearchOutcome::Unattempted =>
            {
                slot.insert(SchoolContacts {
                    school: school.id.clone(),
                    heads: Heads::default(),
                    assistants: Vec::new(),
                    research: outcome,
                });
            }
            std::collections::btree_map::Entry::Vacant(_) => {}
        }
    }
}

fn research_outcome(school: &CanonicalSchool, school_year: SchoolYear) -> ContactResearchOutcome {
    school
        .contact_research
        .iter()
        .filter(|research| research.school_year == school_year)
        .filter_map(|research| match research.subject {
            ContactResearchSubject::Program(_) => Some(research.outcome.clone()),
            ContactResearchSubject::SchoolMailbox(_) => None,
        })
        .reduce(ContactResearchOutcome::combine)
        .map_or(ContactResearchOutcome::Unattempted, core::convert::identity)
}

fn role_slot(coach: &CanonicalCoach) -> Option<(Slot, Gender)> {
    match coach.role {
        CoachRole::HeadCoach => coach
            .sport
            .and_then(|sport| Slot::of(sport).map(|slot| (slot, coach.gender))),
        CoachRole::AthleticDirector => Some((Slot::Director, Gender::Mixed)),
        CoachRole::AssistantCoach | CoachRole::Unknown => None,
    }
}
