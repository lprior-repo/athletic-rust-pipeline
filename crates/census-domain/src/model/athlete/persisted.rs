use super::*;

#[derive(Debug, Deserialize)]
pub(super) struct PersistedAthlete {
    id: AthleteId,
    canonical_name: String,
    known_names: Vec<String>,
    grad_year: GradYear,
    school: SchoolId,
    gender: Gender,
    sports: Vec<Sport>,
    observed_grades: Vec<ObservedGrade>,
    #[serde(default)]
    published_graduations: Vec<PublishedGraduation>,
    public_profile_urls: Vec<String>,
    #[serde(default)]
    source: Option<SourceIdentity>,
    #[serde(default)]
    source_links: Vec<SourceIdentity>,
    #[serde(default)]
    source_identities: Vec<SourceIdentity>,
    #[serde(default)]
    identity_attestations: Vec<IdentityAttestation>,
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
            published_graduations: persisted.published_graduations,
            public_profile_urls: persisted.public_profile_urls,
            source,
            source_links,
            identity_attestations: persisted.identity_attestations,
            evidence: persisted.evidence,
            retained_conflicts: persisted.retained_conflicts,
        }
    }
}
