//! What a source said about a school or an athlete, kept as that source published it.
//!
//! These rows are the reversal point for a canonical decision. The merge can be wrong about which
//! schools are one school, or which athlete rows are one athlete, and the answer to that is not to
//! download the results again but to read what each source actually said: so the row holds the
//! provider's own object id and row key beside the values that source published — the name, the city,
//! the graduating evidence, the profile URL — and no canonical id at all. Which canonical row an
//! observation resolved to is the [`SourceObjectIdentity`](super::source::SourceObjectIdentity)
//! join's answer, and that join is exactly what may change.
//!
//! Only the two objects the review audit asked for are modelled here: a school observation and an
//! athlete observation. A meet or a coach that needs the same treatment gets its own row when it
//! needs one, with the fields that object has.

use serde::{Deserialize, Serialize};

use super::super::{Gender, ObservedGrade, SourceNamespace};
use crate::UsJurisdiction;

/// One source's own observation of a school, as that source published it.
///
/// `(state, observed_name)` is the natural-key material a canonical school is minted from, held here
/// as the source wrote it: the name unnormalized, the state as the source placed it. Two rows whose
/// observations disagree are two rows whose canonical merge can be re-decided without another fetch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSchoolObservation {
    /// The store row id: `{namespace}:{source_school_id}`.
    pub id: String,
    pub namespace: SourceNamespace,
    /// The school's own id at the source.
    pub source_school_id: String,
    /// The row the values were read from, as the source keys it (a directory row, a page slug).
    pub source_row_key: String,
    /// The school's name as the source spelled it.
    pub observed_name: String,
    /// The city the source published, when it published one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    /// The state the source placed the school in, when it named one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<UsJurisdiction>,
    /// The association's own id for the school (a WIAA member id, an MSHSL id, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub association_id: Option<String>,
    /// The conference or district the source filed the school under.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub district: Option<String>,
    /// The school's own site, as the source linked it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub official_url: Option<String>,
    /// The Athletic.net team id the source published for the school.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub athletics_net_team_id: Option<String>,
    /// The day this observation was read (`yyyy-mm-dd`).
    pub observed_on: String,
}

impl SourceSchoolObservation {
    /// Mint the observation of one source school object.
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

    /// Carry the state the source placed the school in.
    pub fn with_state(mut self, state: Option<UsJurisdiction>) -> Self {
        self.state = state;
        self
    }

    /// Carry the city the source published.
    pub fn with_city(mut self, city: Option<String>) -> Self {
        self.city = city;
        self
    }

    /// Carry the association's own id for the school.
    pub fn with_association_id(mut self, association_id: Option<String>) -> Self {
        self.association_id = association_id;
        self
    }

    /// Carry the conference or district the source filed the school under.
    pub fn with_district(mut self, district: Option<String>) -> Self {
        self.district = district;
        self
    }

    /// Carry the school's own site and the team id the source published beside it.
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

/// One source's own observation of an athlete, as that source published it.
///
/// The provider's athlete id and row key are the reversal key. Two rows a canonical merge collapsed
/// are told apart again from what each source said about the athlete — its name, its school, the class
/// the grade observation implies, its gender and its profile URL — without reading the provider again.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAthleteObservation {
    /// The store row id: `{namespace}:{source_athlete_id}`.
    pub id: String,
    pub namespace: SourceNamespace,
    /// The athlete's own id at the source.
    pub source_athlete_id: String,
    /// The row the values were read from, as the source keys it (a roster row, a result row).
    pub source_row_key: String,
    /// The athlete's name as the source spelled it.
    pub observed_name: String,
    /// The school the source placed the athlete at, in the source's own spelling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_school: Option<String>,
    /// The grade and school year the source published, which imply the class it claims.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_grade: Option<ObservedGrade>,
    /// The gender the source published; `Unknown` when the source stated none.
    pub gender: Gender,
    /// The profile page the source published for this athlete.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_url: Option<String>,
    /// The day this observation was read (`yyyy-mm-dd`).
    pub observed_on: String,
}

impl SourceAthleteObservation {
    /// Mint the observation of one source athlete object, with nothing published but the name.
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

    /// Carry the school the source placed the athlete at.
    pub fn with_school(mut self, school: Option<String>) -> Self {
        self.observed_school = school;
        self
    }

    /// Carry the grade and school year the source published.
    pub fn with_grade(mut self, grade: Option<ObservedGrade>) -> Self {
        self.observed_grade = grade;
        self
    }

    /// Carry the gender the source published.
    pub fn with_gender(mut self, gender: Gender) -> Self {
        self.gender = gender;
        self
    }

    /// Carry the profile page the source published.
    pub fn with_profile_url(mut self, url: Option<String>) -> Self {
        self.profile_url = url;
        self
    }
}
