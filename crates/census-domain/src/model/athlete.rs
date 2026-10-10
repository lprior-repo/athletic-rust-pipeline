use super::*;
use std::cmp::Ordering;

mod persisted;
use persisted::PersistedAthlete;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AthleteCandidateKey {
    pub school: SchoolId,
    pub name: String,
    pub grad_year: GradYear,
    pub gender: Gender,
}

impl AthleteCandidateKey {
    pub fn new(school: &SchoolId, name: &str, grad_year: GradYear, gender: Gender) -> Self {
        Self {
            school: school.clone(),
            name: normalize_name(name),
            grad_year,
            gender,
        }
    }

    pub fn index_id(&self) -> AthleteIndexId {
        self.mint()
    }

    const fn gender_letter(gender: Gender) -> &'static str {
        match gender {
            Gender::Boys => "m",
            Gender::Girls => "f",
            Gender::Mixed | Gender::Unknown => "u",
        }
    }

    fn mint<T>(&self) -> Id<T> {
        let year = self.grad_year.get().to_string();
        Id::mint(
            "ath",
            &[
                self.school.as_str(),
                &self.name,
                &year,
                Self::gender_letter(self.gender),
            ],
        )
    }
}

impl PartialOrd for AthleteCandidateKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AthleteCandidateKey {
    fn cmp(&self, other: &Self) -> Ordering {
        self.school
            .as_str()
            .cmp(other.school.as_str())
            .then_with(|| self.name.cmp(&other.name))
            .then_with(|| self.grad_year.cmp(&other.grad_year))
            .then_with(|| Self::gender_letter(self.gender).cmp(Self::gender_letter(other.gender)))
            .then_with(|| self.gender.stable_key().cmp(other.gender.stable_key()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "PersistedAthlete")]
pub struct CanonicalAthlete {
    pub id: AthleteId,
    pub canonical_name: String,
    pub known_names: Vec<String>,
    pub grad_year: GradYear,
    pub school: SchoolId,
    pub gender: Gender,
    pub sports: Vec<Sport>,
    pub observed_grades: Vec<ObservedGrade>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub published_graduations: Vec<PublishedGraduation>,
    pub public_profile_urls: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceIdentity>,
    pub source_links: Vec<SourceIdentity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub identity_attestations: Vec<IdentityAttestation>,
    pub evidence: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_conflicts: Vec<RetainedConflict>,
}

impl CanonicalAthlete {
    pub fn identity_in(&self, namespace: &SourceNamespace) -> Option<&SourceIdentity> {
        self.identities()
            .find(|identity| &identity.namespace == namespace)
    }

    pub fn identities(&self) -> impl Iterator<Item = &SourceIdentity> {
        self.source.iter().chain(self.source_links.iter())
    }

    pub fn add_identity(&mut self, identity: SourceIdentity) {
        let same_owner = self
            .source
            .as_ref()
            .is_some_and(|source| {
                source.namespace == identity.namespace && source.id == identity.id
            });
        if !same_owner {
            if self.source.is_none() {
                self.source = Some(identity);
            } else if !self.source_links.contains(&identity) {
                self.source_links.push(identity);
            }
            return;
        }
        let fills_missing_locator = self
            .source
            .as_ref()
            .is_some_and(|source| source.url.is_none() && identity.url.is_some());
        if fills_missing_locator {
            if let Some(source) = self.source.as_mut() {
                source.url = identity.url;
            }
            return;
        }
        let alternate_locator = identity.url.is_some()
            && self
                .source
                .as_ref()
                .is_some_and(|source| source.url != identity.url);
        if alternate_locator && !self.source_links.contains(&identity) {
            self.source_links.push(identity);
        }
    }

    pub fn mint(
        school: &SchoolId,
        name: &str,
        grad_year: GradYear,
        gender: Gender,
        source: &SourceIdentity,
    ) -> AthleteId {
        let index = AthleteCandidateKey::new(school, name, grad_year, gender).index_id();
        Id::mint(
            "ath_subject",
            &[
                index.as_str(),
                &source.namespace.to_string(),
                &source.id,
                gender.stable_key(),
            ],
        )
    }

    pub fn candidate_key(&self) -> AthleteCandidateKey {
        AthleteCandidateKey::new(
            &self.school,
            &self.canonical_name,
            self.grad_year,
            self.gender,
        )
    }

    pub fn new(
        school: &SchoolId,
        name: impl Into<String>,
        grad_year: GradYear,
        gender: Gender,
        source: SourceIdentity,
    ) -> Self {
        let canonical_name: String = name.into();
        debug_assert!(!canonical_name.is_empty(), "athlete name must not be empty");
        debug_assert!(
            school.validate("sch"),
            "athlete school id {school} does not validate as a school identifier"
        );
        let id = CanonicalAthlete::mint(school, &canonical_name, grad_year, gender, &source);
        Self {
            id,
            canonical_name: canonical_name.clone(),
            known_names: vec![canonical_name],
            grad_year,
            school: school.clone(),
            gender,
            sports: Vec::new(),
            observed_grades: Vec::new(),
            published_graduations: Vec::new(),
            public_profile_urls: Vec::new(),
            source: Some(source),
            source_links: Vec::new(),
            identity_attestations: Vec::new(),
            evidence: Vec::new(),
            retained_conflicts: Vec::new(),
        }
    }

    pub fn new_checked(
        school: &SchoolId,
        name: impl Into<String>,
        grad_year: GradYear,
        gender: Gender,
        source: SourceIdentity,
    ) -> Result<Self, String> {
        let canonical_name: String = name.into();
        if canonical_name.is_empty() {
            return Err("athlete name must not be empty".to_string());
        }
        if !school.validate("sch") {
            return Err(format!(
                "athlete school id {school} does not validate as a school identifier"
            ));
        }
        let id = CanonicalAthlete::mint(school, &canonical_name, grad_year, gender, &source);
        Ok(Self {
            id,
            canonical_name: canonical_name.clone(),
            known_names: vec![canonical_name],
            grad_year,
            school: school.clone(),
            gender,
            sports: Vec::new(),
            observed_grades: Vec::new(),
            published_graduations: Vec::new(),
            public_profile_urls: Vec::new(),
            source: Some(source),
            source_links: Vec::new(),
            identity_attestations: Vec::new(),
            evidence: Vec::new(),
            retained_conflicts: Vec::new(),
        })
    }

    pub fn has_cohort_conflict(&self) -> bool {
        self.published_graduations
            .iter()
            .any(|observation| observation.grad_year != self.grad_year)
            || self
                .observed_grades
                .iter()
                .any(|observation| observation.grad_year() != Some(self.grad_year))
    }

    pub fn derived_cohort_confidence(&self) -> Option<Confidence> {
        if self.has_cohort_conflict() {
            Some(Confidence::LOW)
        } else if self
            .published_graduations
            .iter()
            .any(|observation| observation.grad_year == self.grad_year)
            || self
                .observed_grades
                .iter()
                .any(|observation| observation.grad_year() == Some(self.grad_year))
        {
            Some(Confidence::HIGH)
        } else {
            None
        }
    }
}

#[cfg(test)]
#[path = "athlete_tests.rs"]
mod tests;
