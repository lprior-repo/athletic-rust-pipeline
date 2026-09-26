use super::*;
use std::cmp::Ordering;

/// A candidate-search index, not a person's identity. Equal school, name, graduating class and
/// category narrow a review search; they never authorize an identity merge. Source-owned subjects
/// use [`CanonicalAthlete::new`], and an applied decision is required to join subjects.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AthleteCandidateKey {
    /// The school the athlete competes for: part of the key, so a transfer is a second candidate.
    pub school: SchoolId,
    /// Normalized once at construction: the raw spelling is evidence the row keeps, never key
    /// material.
    pub name: String,
    pub grad_year: GradYear,
    pub gender: Gender,
}

impl AthleteCandidateKey {
    /// Build the key from a source's raw spelling of a name.
    pub fn new(school: &SchoolId, name: &str, grad_year: GradYear, gender: Gender) -> Self {
        Self {
            school: school.clone(),
            name: normalize_name(name),
            grad_year,
            gender,
        }
    }

    /// The candidate-search bucket id. Multiple distinct people may share this bucket.
    pub fn index_id(&self) -> AthleteIndexId {
        self.mint()
    }

    /// The gender side as an id spells it: `Mixed` and `Unknown` both mint `u`, which is why those two
    /// stay a retained disagreement rather than a merge when one row is re-observed as the other.
    const fn gender_letter(gender: Gender) -> &'static str {
        match gender {
            Gender::Boys => "m",
            Gender::Girls => "f",
            Gender::Mixed | Gender::Unknown => "u",
        }
    }

    /// The one place a candidate key becomes an id: the four fields, in the order [`Id::mint`] hashes
    /// them, under the `ath` prefix both roles use.
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
    /// The order the canonical-member rule reads, and it is the *mint* order: the school, the
    /// normalized name, the class (as a number — the years the census places are four-digit, so this
    /// is the order their digits spell) and the gender letter.
    ///
    /// The letter alone cannot order two keys, because `Mixed` and `Unknown` share `u`, so the frozen
    /// [`Gender::stable_key`] breaks the tie. Nothing here reads the enum's declaration order: adding
    /// a `Gender` variant must not land between two existing ones and move a cluster id.
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
    /// The school the athlete competes for: part of the natural key, so it stays on the row even
    /// though the jurisdiction below is derived from it.
    pub school: SchoolId,
    pub gender: Gender,
    pub sports: Vec<Sport>,
    /// Grade observations, newest last; never collapsed into `grad_year` alone.
    pub observed_grades: Vec<ObservedGrade>,
    pub public_profile_urls: Vec<String>,
    /// The source object this subject represents; required in the current wire contract.
    pub source: SourceIdentity,
    /// Additional identifiers explicitly published for that same source object.
    pub source_links: Vec<SourceIdentity>,
    pub evidence: Vec<Evidence>,
    /// Canonical-id collisions this row's merge retained: another natural key minted this id, so the
    /// row below is the one that survived and the other subject's facts were not absorbed. Empty on
    /// every row whose fields still state the id they minted, which is every row until one collides.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_conflicts: Vec<RetainedConflict>,
}

impl CanonicalAthlete {
    /// The identity this row carries for one source namespace: the provider's own athlete id, the
    /// §31 key material a row minted from that source cites back. `None` when this athlete is not
    /// known to that source, which is the honest answer for a row merged from sources that never
    /// named it.
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

    /// Mint a source-owned subject. Candidate-search facts alone cannot mint a person.
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
            &[index.as_str(), &source.namespace.to_string(), &source.id],
        )
    }

    /// The candidate key this row's own fields restate, exactly as [`NaturalKey`] restates its mint
    /// material: the school it competes for, its canonical name, its class and its gender side.
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

    /// Confidence in the cohort derivation only. Agreement about graduation year says nothing
    /// about whether two source subjects are the same person.
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
