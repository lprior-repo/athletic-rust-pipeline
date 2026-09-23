use super::*;

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
    /// shared vendor id.
    pub fn mint(school: &SchoolId, name: &str, grad_year: GradYear, gender: Gender) -> AthleteId {
        Id::mint(
            "ath",
            &[
                school.as_str(),
                &normalize_name(name),
                &grad_year.get().to_string(),
                match gender {
                    Gender::Boys => "m",
                    Gender::Girls => "f",
                    _ => "u",
                },
            ],
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
