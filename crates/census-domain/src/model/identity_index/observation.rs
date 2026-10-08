use super::{CanonicalAthlete, CanonicalJsonError, EvidenceMethod, IdentityAttestation, PersonKey};
use crate::model::{athlete_identity_digest, person_key, serialized_digest};

pub(super) fn links(athlete: &CanonicalAthlete, primary: Option<PersonKey>) -> Vec<PersonKey> {
    let mut links: Vec<_> = athlete
        .source_links
        .iter()
        .filter_map(person_key)
        .filter(|key| Some(*key) != primary)
        .collect();
    links.sort_unstable();
    links.dedup();
    links
}

pub(super) fn parsed(athlete: &CanonicalAthlete) -> bool {
    athlete.evidence.iter().any(|evidence| {
        evidence.method == EvidenceMethod::Parsed
            && evidence
                .source
                .url
                .as_ref()
                .is_some_and(|url| !url.is_empty())
    })
}

pub(super) fn conflicted(
    athlete: &CanonicalAthlete,
    primary: Option<PersonKey>,
    links: &[PersonKey],
) -> bool {
    links
        .iter()
        .zip(links.iter().skip(1))
        .any(|(left, right)| left.0 == right.0)
        || primary.is_some_and(|key| links.iter().any(|link| link.0 == key.0))
        || !athlete.retained_conflicts.is_empty()
        || athlete.has_cohort_conflict()
}

pub(super) fn digest(
    athlete: &CanonicalAthlete,
    attested: &[IdentityAttestation],
) -> Result<String, CanonicalJsonError> {
    if attested.is_empty() {
        athlete_identity_digest(athlete)
    } else {
        serialized_digest(&(athlete, attested))
    }
}
