use serde::{Deserialize, Serialize};

use super::super::{Gender, ObservedGrade, SourceNamespace};
use crate::UsJurisdiction;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSchoolObservation {
    pub id: String,
    pub namespace: SourceNamespace,
    pub source_school_id: String,
    pub source_row_key: String,
    pub observed_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<UsJurisdiction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub association_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub district: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub official_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub athletics_net_team_id: Option<String>,
    pub observed_on: String,
}

impl SourceSchoolObservation {
    pub fn new(
        namespace: SourceNamespace,
        source_school_id: impl Into<String>,
        source_row_key: impl Into<String>,
        observed_name: impl Into<String>,
        observed_on: impl Into<String>,
    ) -> Self {
        let source_school_id = source_school_id.into();
        Self {
            id: format!("{namespace}:{source_school_id}"),
            namespace,
            source_school_id,
            source_row_key: source_row_key.into(),
            observed_name: observed_name.into(),
            city: None,
            state: None,
            association_id: None,
            district: None,
            official_url: None,
            athletics_net_team_id: None,
            observed_on: observed_on.into(),
        }
    }

    pub fn with_state(mut self, state: Option<UsJurisdiction>) -> Self {
        self.state = state;
        self
    }

    pub fn with_city(mut self, city: Option<String>) -> Self {
        self.city = city;
        self
    }

    pub fn with_association_id(mut self, association_id: Option<String>) -> Self {
        self.association_id = association_id;
        self
    }

    pub fn with_district(mut self, district: Option<String>) -> Self {
        self.district = district;
        self
    }

    pub fn with_urls(
        mut self,
        official_url: Option<String>,
        athletics_net_team_id: Option<String>,
    ) -> Self {
        self.official_url = official_url;
        self.athletics_net_team_id = athletics_net_team_id;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAthleteObservation {
    pub id: String,
    pub namespace: SourceNamespace,
    pub source_athlete_id: String,
    pub source_row_key: String,
    pub observed_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_school: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_grade: Option<ObservedGrade>,
    pub gender: Gender,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_url: Option<String>,
    pub observed_on: String,
}

impl SourceAthleteObservation {
    pub fn new(
        namespace: SourceNamespace,
        source_athlete_id: impl Into<String>,
        source_row_key: impl Into<String>,
        observed_name: impl Into<String>,
        observed_on: impl Into<String>,
    ) -> Self {
        let source_athlete_id = source_athlete_id.into();
        Self {
            id: format!("{namespace}:{source_athlete_id}"),
            namespace,
            source_athlete_id,
            source_row_key: source_row_key.into(),
            observed_name: observed_name.into(),
            observed_school: None,
            observed_grade: None,
            gender: Gender::Unknown,
            profile_url: None,
            observed_on: observed_on.into(),
        }
    }

    pub fn with_school(mut self, school: Option<String>) -> Self {
        self.observed_school = school;
        self
    }

    pub fn with_grade(mut self, grade: Option<ObservedGrade>) -> Self {
        self.observed_grade = grade;
        self
    }

    pub fn with_gender(mut self, gender: Gender) -> Self {
        self.gender = gender;
        self
    }

    pub fn with_profile_url(mut self, url: Option<String>) -> Self {
        self.profile_url = url;
        self
    }
}
