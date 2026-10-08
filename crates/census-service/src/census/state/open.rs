use census_domain::model::{ReviewCase, ReviewState, COHORT_DECISION_FAMILIES};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JurisdictionStages {
    pub teams: census_crawl::CollectionDisposition,
    pub rosters: census_crawl::CollectionDisposition,
    pub meets: census_crawl::CollectionDisposition,
    pub results: census_crawl::CollectionDisposition,
    pub contacts: census_crawl::CollectionDisposition,
    pub publication: census_crawl::CollectionDisposition,
    pub refused_sources: u64,
    pub owed_rosters: u64,
    pub owed_results: u64,
}

impl JurisdictionStages {
    pub fn terminal(self) -> bool {
        [
            self.teams,
            self.rosters,
            self.meets,
            self.results,
            self.contacts,
            self.publication,
        ]
        .into_iter()
        .all(census_crawl::CollectionDisposition::is_complete)
            && self.owed_rosters == 0
            && self.owed_results == 0
            && self.refused_sources == 0
    }

    pub fn owing(self) -> Vec<&'static str> {
        let stages = [
            (self.teams, "teams"),
            (self.rosters, "rosters"),
            (self.meets, "meets_history"),
            (self.results, "results_history"),
            (self.contacts, "contact_research"),
            (self.publication, "publication"),
        ];
        stages
            .into_iter()
            .filter(|(status, name)| {
                !status.is_complete()
                    || (*name == "rosters" && self.owed_rosters > 0)
                    || (*name == "results_history" && self.owed_results > 0)
            })
            .map(|(_, name)| name)
            .chain((self.refused_sources > 0).then_some("refused_sources"))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceObject {
    pub endpoint: String,
    pub observations: u64,
    pub windows: u64,
    #[serde(default)]
    pub disposition: census_crawl::CollectionDisposition,
}

impl SourceObject {
    pub fn terminal(&self) -> bool {
        self.disposition.is_complete()
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
        .filter(|object| object.terminal() && object.observations == 0 && object.windows > 0)
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
