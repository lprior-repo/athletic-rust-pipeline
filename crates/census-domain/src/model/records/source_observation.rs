//! One source's own observation of one object, as one store row.
//!
//! [`SourceSchoolObservation`](super::observation::SourceSchoolObservation) and
//! [`SourceAthleteObservation`](super::observation::SourceAthleteObservation) say what a single
//! source published about one of its own objects. This module is what makes them rows: the store keys
//! an observation by the `id` field of its serialized form, and either object's id is
//! `{namespace}:{provider id}`, so the two shapes are carried by one enum whose tag rides *inside*
//! that map rather than wrapping it — `{"object":"school","id":…}` — which keeps `id` where the key
//! encoder reads it while letting a single table hold what every source said about every object.
//!
//! The mint reads the source's own object id off the row the adapter just built, because that row is
//! the adapter's observation of the source's page: it is appended as evidence before any merge sees
//! it, so the identity it carries for the provider — and the name, city, site and team id beside that
//! identity — is exactly what the source published.

use serde::{Deserialize, Serialize};

use super::observation::{SourceAthleteObservation, SourceSchoolObservation};
use crate::model::{CanonicalSchool, Gender, SourceNamespace};

/// One source's own observation of one object, as one store row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "object", rename_all = "snake_case")]
pub enum SourceObservation {
    School(SourceSchoolObservation),
    Athlete(SourceAthleteObservation),
}

impl SourceObservation {
    /// The store row id: `{namespace}:{provider id}`.
    pub fn id(&self) -> &str {
        match self {
            Self::School(row) => &row.id,
            Self::Athlete(row) => &row.id,
        }
    }

    /// The source that published the observation.
    pub fn namespace(&self) -> &SourceNamespace {
        match self {
            Self::School(row) => &row.namespace,
            Self::Athlete(row) => &row.namespace,
        }
    }

    /// The day the observation was read (`yyyy-mm-dd`).
    pub fn observed_on(&self) -> &str {
        match self {
            Self::School(row) => &row.observed_on,
            Self::Athlete(row) => &row.observed_on,
        }
    }

    /// Which object the row observes, as the wire tag spells it.
    pub fn object(&self) -> &'static str {
        match self {
            Self::School(_) => "school",
            Self::Athlete(_) => "athlete",
        }
    }

    /// Fold a second sighting of the same object into this one.
    ///
    /// Two kinds under one id cannot be told apart, and the namespaced id makes the pair unreachable;
    /// the row is then left as the first sighting wrote it rather than guessing which object the store
    /// meant.
    pub fn absorb(&mut self, other: Self) {
        match (self, other) {
            (Self::School(mine), Self::School(theirs)) => mine.absorb(theirs),
            (Self::Athlete(mine), Self::Athlete(theirs)) => mine.absorb(theirs),
            _ => {}
        }
    }
}

impl SourceSchoolObservation {
    /// The observation of one school: what the source published, read off the row the adapter minted
    /// for it together with the identity that row carries for the source's own object.
    ///
    /// `None` when the row carries no identity in `namespace`. Without the provider's own object id
    /// there is no key to file the observation under, and minting one from the canonical id would hand
    /// the row the very thing it exists to outlive — the merge it makes reversible.
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
                identity.url.clone().unwrap_or_default(),
                school.name.clone(),
                observed_on,
            )
            .with_state(school.state)
            .with_city(school.city.clone())
            .with_association_id(association_id)
            .with_urls(school.school_website.clone(), athletic_net_team_id(school)),
        )
    }

    /// Keep the first sighting's identity, let a later one complete it, and hold the earliest day the
    /// object was seen — the rule the meet census's own index rows merge under.
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
    /// Keep the first sighting's identity, let a later one complete it, and hold the earliest day the
    /// object was seen. A later roster fills a blank grade; it never overwrites the grade this row
    /// already carries, which is the one the reversal argument rests on.
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

/// The Athletic.net team id a school row carries, when one of its identities is the team page.
fn athletic_net_team_id(school: &CanonicalSchool) -> Option<String> {
    school
        .source_identities
        .iter()
        .find(|identity| {
            matches!(&identity.namespace, SourceNamespace::AthleticNet { kind } if kind == "team")
        })
        .map(|identity| identity.id.clone())
}

/// Take `source`'s value only where this row holds none.
fn fill<T>(target: &mut Option<T>, source: Option<T>) {
    if target.is_none() {
        *target = source;
    }
}
