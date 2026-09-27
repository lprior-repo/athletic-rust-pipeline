use std::collections::BTreeMap;

use census_domain::model::{CanonicalCoach, CoachRole, Gender, SchoolId, SchoolYear};

use super::heads::{individuals, Heads};
use super::{Named, Slot};

#[derive(Debug, Clone)]
pub(in crate::workbook::recruiting) struct SchoolContacts {
    pub(super) school: SchoolId,
    pub(in crate::workbook::recruiting) heads: Heads,
    pub(super) assistants: Vec<Named>,
}

impl SchoolContacts {
    pub(in crate::workbook::recruiting) fn director(&self) -> Option<&Named> {
        self.heads.resolve(Slot::Director, Gender::Mixed).named()
    }
}

pub(in crate::workbook::recruiting) fn contacts(
    coaches: &[CanonicalCoach], school_year: SchoolYear,
) -> BTreeMap<String, SchoolContacts> {
    let mut buckets: BTreeMap<&SchoolId, Buckets<'_>> = BTreeMap::new();
    for coach in coaches {
        buckets.entry(&coach.school).or_default().push(coach);
    }
    buckets.into_iter().map(|(school, buckets)| {
        (school.as_str().to_owned(), buckets.contacts(school, school_year))
    }).collect()
}

#[derive(Default)]
struct Buckets<'a> {
    heads: BTreeMap<(Slot, Gender), Vec<&'a CanonicalCoach>>,
    assistants: Vec<&'a CanonicalCoach>,
}

impl<'a> Buckets<'a> {
    fn push(&mut self, coach: &'a CanonicalCoach) {
        match coach.role {
            CoachRole::HeadCoach => {
                if let Some(sport) = coach.sport {
                    self.heads.entry((Slot::of(sport), coach.gender)).or_default().push(coach);
                }
            }
            CoachRole::AthleticDirector => {
                self.heads.entry((Slot::Director, Gender::Mixed)).or_default().push(coach);
            }
            CoachRole::AssistantCoach => self.assistants.push(coach),
            CoachRole::Unknown => {}
        }
    }

    fn contacts(&self, school: &SchoolId, school_year: SchoolYear) -> SchoolContacts {
        let mut heads = Heads::default();
        for ((slot, side), rows) in &self.heads {
            heads.insert(school.as_str(), *slot, *side, rows, school_year);
        }
        SchoolContacts {
            school: school.clone(), heads,
            assistants: individuals(&self.assistants, school_year),
        }
    }
}
