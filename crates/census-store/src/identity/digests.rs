use crate::digest::serialized_digest;
use census_domain::model::{CanonicalAthlete, ReviewVerdictRecord};

pub(crate) fn athlete_identity_digest(
    athlete: &CanonicalAthlete,
) -> Result<String, serde_json::Error> {
    serialized_digest(athlete)
}

pub(crate) fn identity_verdict_digest(
    verdict: &ReviewVerdictRecord,
) -> Result<String, serde_json::Error> {
    serialized_digest(verdict)
}
