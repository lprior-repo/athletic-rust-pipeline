use serde::{Deserialize, Serialize};

use super::observation::{SourceAthleteObservation, SourceSchoolObservation};
use crate::model::{CanonicalAthlete, CanonicalSchool, Gender, SourceNamespace};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "object", rename_all = "snake_case")]
pub enum SourceObservation {
    School(SourceSchoolObservation),
    Athlete(SourceAthleteObservation),
}

impl SourceObservation {
    pub fn id(&self) -> &str {
        match self {
            Self::School(row) => &row.id,
            Self::Athlete(row) => &row.id,
        }
    }

    pub fn namespace(&self) -> &SourceNamespace {
        match self {
            Self::School(row) => &row.namespace,
            Self::Athlete(row) => &row.namespace,
        }
    }

    pub fn observed_on(&self) -> &str {
        match self {
            Self::School(row) => &row.observed_on,
            Self::Athlete(row) => &row.observed_on,
        }
    }

    pub fn object(&self) -> &'static str {
        match self {
            Self::School(_) => "school",
            Self::Athlete(_) => "athlete",
        }
    }

    pub fn absorb(&mut self, other: Self) {
        match (self, other) {
            (Self::School(mine), Self::School(theirs)) => mine.absorb(theirs),
            (Self::Athlete(mine), Self::Athlete(theirs)) => mine.absorb(theirs),
            _ => {}
        }
    }
}

impl SourceSchoolObservation {
    pub fn of_school(
        namespace: &SourceNamespace,
        school: &CanonicalSchool,
        observed_on: &str,
    ) -> Option<Self> {
        let identity = school
            .source_identities
            .iter()
            .find(|identity| &identity.namespace == namespace)?;
        let association_id = match namespace {
            SourceNamespace::AssociationSchool { .. } => Some(identity.id.clone()),
            _ => None,
        };
        Some(
            Self::new(
                namespace.clone(),
                identity.id.clone(),
                identity
                    .url
                    .clone()
                    .map_or(Default::default(), core::convert::identity),
                school.name.clone(),
                observed_on,
            )
            .with_state(school.state)
            .with_city(school.city.clone())
            .with_association_id(association_id)
            .with_urls(school.school_website.clone(), athletic_net_team_id(school)),
        )
    }

    fn absorb(&mut self, other: Self) {
        if other.observed_on < self.observed_on {
            self.observed_on = other.observed_on;
        }
        fill(&mut self.city, other.city);
        fill(&mut self.state, other.state);
        fill(&mut self.association_id, other.association_id);
        fill(&mut self.district, other.district);
        fill(&mut self.official_url, other.official_url);
        fill(&mut self.athletics_net_team_id, other.athletics_net_team_id);
    }
}

impl SourceAthleteObservation {
    pub fn of_athlete(
        namespace: &SourceNamespace,
        athlete: &CanonicalAthlete,
        observed_school: Option<String>,
        observed_on: &str,
    ) -> Option<Self> {
        let identity = athlete
            .identities()
            .find(|identity| &identity.namespace == namespace)?;
        let page = identity
            .url
            .clone()
            .or_else(|| athlete.public_profile_urls.first().cloned());
        Some(
            Self::new(
                namespace.clone(),
                identity.id.clone(),
                page.map_or(Default::default(), core::convert::identity),
                athlete.canonical_name.clone(),
                observed_on,
            )
            .with_school(observed_school)
            .with_grade(athlete.observed_grades.last().cloned())
            .with_gender(athlete.gender)
            .with_profile_url(athlete.public_profile_urls.first().cloned()),
        )
    }

    fn absorb(&mut self, other: Self) {
        if other.observed_on < self.observed_on {
            self.observed_on = other.observed_on;
        }
        fill(&mut self.observed_school, other.observed_school);
        fill(&mut self.observed_grade, other.observed_grade);
        fill(&mut self.profile_url, other.profile_url);
        if self.gender == Gender::Unknown {
            self.gender = other.gender;
        }
    }
}

fn athletic_net_team_id(school: &CanonicalSchool) -> Option<String> {
    school
        .source_identities
        .iter()
        .find(|identity| {
            matches!(&identity.namespace, SourceNamespace::AthleticNet { kind } if kind == "team")
        })
        .map(|identity| identity.id.clone())
}

fn fill<T>(target: &mut Option<T>, source: Option<T>) {
    if target.is_none() {
        *target = source;
    }
}
