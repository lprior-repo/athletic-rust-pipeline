use census_domain::model::{ReviewCase, ReviewState, COHORT_DECISION_FAMILIES};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JurisdictionStages {
    pub teams: bool,
    pub rosters: bool,
    pub meets: bool,
    pub owed_rosters: u64,
}

impl JurisdictionStages {
    pub fn terminal(self) -> bool {
        self.teams && self.rosters && self.meets && self.owed_rosters == 0
    }

    pub fn owing(self) -> Vec<&'static str> {
        let mut owing = Vec::new();
        if !self.teams {
            owing.push("teams");
        }
        if !self.rosters || self.owed_rosters > 0 {
            owing.push("rosters");
        }
        if !self.meets {
            owing.push("meets");
        }
        owing
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceObject {
    pub endpoint: String,
    pub observations: u64,
    pub windows: u64,
}

impl SourceObject {
    pub fn terminal(&self) -> bool {
        self.observations > 0 || self.windows > 0
    }
}

pub fn owed_jurisdictions(stages: &[JurisdictionStages]) -> u64 {
    count(stages.iter().filter(|stage| !stage.terminal()).count())
}

pub fn owed_source_objects(objects: &[SourceObject]) -> u64 {
    count(objects.iter().filter(|object| !object.terminal()).count())
}

pub fn silent_source_objects(objects: &[SourceObject]) -> Vec<String> {
    let mut names = objects
        .iter()
        .filter(|object| object.observations == 0 && object.windows > 0)
        .map(|object| object.endpoint.clone())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

pub fn owed_cohort_decisions(cases: &[ReviewCase]) -> u64 {
    count(
        cases
            .iter()
            .filter(|case| case.state == ReviewState::Pending)
            .filter(|case| COHORT_DECISION_FAMILIES.contains(&case.family.as_str()))
            .count(),
    )
}

pub fn owed_identity_candidates(cases: &[ReviewCase]) -> u64 {
    count(
        cases
            .iter()
            .filter(|case| case.state == ReviewState::Pending)
            .count(),
    )
}

fn count(value: usize) -> u64 {
    u64::try_from(value).map_or(u64::MAX, core::convert::identity)
}
