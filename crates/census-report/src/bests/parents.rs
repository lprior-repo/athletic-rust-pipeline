use crate::report::Derivation;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalSchool, CanonicalTeam, EventKind,
};
use std::collections::HashMap;

pub(crate) struct Parents<'a> {
    athletes: HashMap<&'a str, &'a CanonicalAthlete>,
    aliases: &'a HashMap<String, String>,
    meets: HashMap<&'a str, &'a CanonicalMeet>,
    events: HashMap<&'a str, &'a CanonicalEvent>,
    teams: HashMap<&'a str, &'a CanonicalTeam>,
    schools: HashMap<&'a str, &'a CanonicalSchool>,
}

impl<'a> Parents<'a> {
    pub(crate) fn of(derivation: &'a Derivation<'_>) -> Self {
        Self {
            athletes: index(derivation.athletes(), |athlete| athlete.id.as_str()),
            aliases: derivation.athlete_aliases(),
            meets: index(derivation.meets(), |meet| meet.id.as_str()),
            events: derivation
                .events()
                .iter()
                .map(|event| (event.id.as_str(), event))
                .collect(),
            teams: derivation
                .dataset()
                .teams
                .values()
                .map(|team| (team.id.as_str(), team))
                .collect(),
            schools: index(derivation.schools(), |school| school.id.as_str()),
        }
    }

    pub(crate) fn athlete(&self, id: &str) -> Option<&'a CanonicalAthlete> {
        let canonical = self.aliases.get(id).map_or(id, String::as_str);
        self.athletes.get(canonical).copied()
    }

    pub(crate) fn meet(&self, id: &str) -> Option<&'a CanonicalMeet> {
        self.meets.get(id).copied()
    }

    pub(crate) fn event(&self, id: &str) -> Option<&'a EventKind> {
        self.events.get(id).map(|event| &event.kind)
    }

    pub(crate) fn canonical_event(&self, id: &str) -> Option<&'a CanonicalEvent> {
        self.events.get(id).copied()
    }

    pub(crate) fn team(&self, id: &str) -> Option<&'a CanonicalTeam> {
        self.teams.get(id).copied()
    }

    pub(crate) fn school(&self, id: &str) -> Option<&'a CanonicalSchool> {
        self.schools.get(id).copied()
    }
}

fn index<'a, T>(rows: &'a [T], id: impl Fn(&'a T) -> &'a str) -> HashMap<&'a str, &'a T> {
    rows.iter().map(|row| (id(row), row)).collect()
}
