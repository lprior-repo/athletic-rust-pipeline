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
    pub source: SourceIdentity,
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
        std::iter::once(&self.source).chain(self.source_links.iter())
    }

    pub fn add_identity(&mut self, identity: SourceIdentity) {
        if identity.namespace == self.source.namespace && identity.id == self.source.id {
            if self.source.url.is_none() {
                self.source.url = identity.url;
            }
        } else if !self.source_links.contains(&identity) {
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
            source,
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
