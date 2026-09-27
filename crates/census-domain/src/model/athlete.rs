use super::*;
use std::cmp::Ordering;

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
    pub public_profile_urls: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceIdentity>,
    pub source_links: Vec<SourceIdentity>,
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
        match self.source.as_mut() {
            Some(source) if source.namespace == identity.namespace && source.id == identity.id => {
                if source.url.is_none() {
                    source.url = identity.url;
                }
            }
            Some(_) => {
                if !self.source_links.contains(&identity) {
                    self.source_links.push(identity);
                }
            }
            None => self.source = Some(identity),
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
        let canonical_name = name.into();
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
            public_profile_urls: Vec::new(),
            source: Some(source),
            source_links: Vec::new(),
            evidence: Vec::new(),
            retained_conflicts: Vec::new(),
        }
    }

    pub fn derived_cohort_confidence(&self) -> Option<Confidence> {
        if self
            .observed_grades
            .iter()
            .any(|observation| observation.grad_year() != self.grad_year)
        {
            Some(Confidence::LOW)
        } else if self
            .observed_grades
            .iter()
            .any(|observation| observation.grad_year() == self.grad_year)
        {
            Some(Confidence::HIGH)
        } else {
            None
        }
    }
}

#[derive(Debug, Deserialize)]
struct PersistedAthlete {
    id: AthleteId,
    canonical_name: String,
    known_names: Vec<String>,
    grad_year: GradYear,
    school: SchoolId,
    gender: Gender,
    sports: Vec<Sport>,
    observed_grades: Vec<ObservedGrade>,
    public_profile_urls: Vec<String>,
    #[serde(default)]
    source: Option<SourceIdentity>,
    #[serde(default)]
    source_links: Vec<SourceIdentity>,
    #[serde(default)]
    source_identities: Vec<SourceIdentity>,
    evidence: Vec<Evidence>,
    #[serde(default)]
    retained_conflicts: Vec<RetainedConflict>,
}

impl From<PersistedAthlete> for CanonicalAthlete {
    fn from(persisted: PersistedAthlete) -> Self {
        let mut source_links = persisted.source_links;
        let source = persisted
            .source
            .or_else(|| persisted.source_identities.first().cloned());
        for identity in persisted.source_identities {
            if Some(&identity) != source.as_ref() && !source_links.contains(&identity) {
                source_links.push(identity);
            }
        }
        Self {
            id: persisted.id,
            canonical_name: persisted.canonical_name,
            known_names: persisted.known_names,
            grad_year: persisted.grad_year,
            school: persisted.school,
            gender: persisted.gender,
            sports: persisted.sports,
            observed_grades: persisted.observed_grades,
            public_profile_urls: persisted.public_profile_urls,
            source,
            source_links,
            evidence: persisted.evidence,
            retained_conflicts: persisted.retained_conflicts,
        }
    }
}

#[cfg(test)]
#[path = "athlete_tests.rs"]
mod tests;
