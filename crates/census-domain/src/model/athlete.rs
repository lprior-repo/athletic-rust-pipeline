use super::*;
use std::cmp::Ordering;

// -------------------------------------------------------------------------------------------------
// Candidate keys
// -------------------------------------------------------------------------------------------------

/// The four facts two sources have to agree on to be naming one athlete candidate: the school the
/// athlete competes for, the normalized name, the class and the gender side.
///
/// This is the *candidate* key — what one observation names — and a candidate is not yet an athlete:
/// two spellings of one person, or one person's school transfer, are two candidates that only a
/// decision (a rule or a review verdict) may resolve into one cluster. Both id roles are minted from
/// the same bytes, so a cluster of one candidate prints the id that candidate already had, and a
/// member that sorts after the canonical one never moves a cluster's id.
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

    /// This candidate's own id: what one source's observation identifies.
    pub fn candidate_id(&self) -> AthleteCandidateId {
        self.mint()
    }

    /// The id of a cluster whose canonical member is this candidate.
    ///
    /// A cluster is addressed by its canonical member — the minimum member under [`Ord`] — so a
    /// cluster of one prints exactly that candidate's id, and a member sorting after it never moves
    /// the cluster. Losing the canonical member is the one change that moves a cluster's id, and it
    /// has to record the departure to stay reversible.
    pub fn cluster_id(&self) -> AthleteId {
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
    pub source_identities: Vec<SourceIdentity>,
    pub evidence: Vec<Evidence>,
    pub identity_confidence: Confidence,
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
        self.source_identities
            .iter()
            .find(|identity| &identity.namespace == namespace)
    }

    /// Mint an athlete from (school, normalized name, grad year, gender).
    ///
    /// Two sources that agree on those four facts produce the same canonical athlete without any
    /// shared vendor id. The four facts are the [`AthleteCandidateKey`], and its cluster rule is the
    /// only place that turns a key into an id — so a row minted here is a cluster of one candidate and
    /// carries that candidate's id.
    pub fn mint(school: &SchoolId, name: &str, grad_year: GradYear, gender: Gender) -> AthleteId {
        AthleteCandidateKey::new(school, name, grad_year, gender).cluster_id()
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
    ) -> Self {
        let canonical_name = name.into();
        let id = CanonicalAthlete::mint(school, &canonical_name, grad_year, gender);
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
            source_identities: Vec::new(),
            evidence: Vec::new(),
            identity_confidence: Confidence::MEDIUM,
            retained_conflicts: Vec::new(),
        }
    }

    /// The identity confidence this row's own grade observations imply.
    ///
    /// An observation that disagrees with the published cohort lowers it instead of silently
    /// rewriting the athlete's graduating class, and one that agrees raises it to the high bar. A row
    /// carrying no observation states no derivation and keeps the confidence it was built with, which
    /// is what lets a source that knows the cohort without naming a grade level say so.
    ///
    /// [`None`] is that last case: the rule has nothing to say about the row, so the field it was
    /// given is the row's own claim rather than a value this derivation overwrites.
    pub fn derived_identity_confidence(&self) -> Option<Confidence> {
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
