use super::{frozen, reported_profile_prs, CoverageEvidence};
use crate::report::ReportResult;
use census_domain::model::{
    AppliedAthleteIdentity, AthleteIdentityIndex, AthleteIdentityProjection, CanonicalAthlete,
    CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, RelayResult, ReviewCase, ReviewVerdictRecord, SourceAccessCondition,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Deserialize)]
pub(super) struct Loaded {
    pub(super) athletes: Vec<CanonicalAthlete>,
    #[serde(deserialize_with = "frozen::map_values")]
    pub(super) schools: Vec<CanonicalSchool>,
    #[serde(deserialize_with = "frozen::map_values")]
    pub(super) teams: Vec<CanonicalTeam>,
    pub(super) coaches: Vec<CanonicalCoach>,
    pub(super) coach_observations: Vec<CanonicalCoach>,
    pub(super) events: Vec<CanonicalEvent>,
    pub(super) meets: Vec<CanonicalMeet>,
    pub(super) performances: Vec<CanonicalPerformance>,
    pub(super) review_cases: Vec<ReviewCase>,
    pub(super) source_access: Vec<SourceAccessCondition>,
    pub(super) verdicts: Vec<ReviewVerdictRecord>,
    pub(super) identity_decisions: Vec<AppliedAthleteIdentity>,
    pub(super) coverage_evidence: CoverageEvidence,
    pub(super) relays: Vec<RelayResult>,
    #[serde(default)]
    pub(super) reported_profile_prs: reported_profile_prs::ReportedProfilePrs,
    #[serde(default)]
    pub(super) selection: Option<crate::export::qualification::SelectionBinding>,
}

pub(super) fn projection_of(loaded: &Loaded) -> ReportResult<Arc<AthleteIdentityProjection>> {
    let mut index = AthleteIdentityIndex::default();
    for athlete in &loaded.athletes {
        index
            .observe(athlete)
            .map_err(census_store::StoreError::from)?;
    }
    Ok(Arc::new(census_store::build_athlete_identity_projection(
        index,
        &loaded.review_cases,
        &loaded.verdicts,
        &loaded.identity_decisions,
    )?))
}

pub(super) fn aliases_of(
    athletes: &[CanonicalAthlete],
    projection: &AthleteIdentityProjection,
) -> HashMap<String, String> {
    let mut aliases = HashMap::new();
    for athlete in athletes {
        let subject = athlete.id.as_str();
        let canonical = projection.canonical_id(subject);
        if canonical != subject {
            aliases.insert(subject.to_string(), canonical.to_string());
        }
    }
    aliases
}
