use super::serialization_digest::serialized_digest;
use super::{AthleteCandidateId, AthleteId, CanonicalAthlete, ReviewVerdictRecord, SourceNamespace};
use serde::{Deserialize, Serialize};

pub const ATHLETE_IDENTITY_POLICY: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppliedIdentityKind {
    SourceBound,
    SamePerson,
    DifferentPerson,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityMember {
    pub subject: AthleteCandidateId,
    pub evidence_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedAthleteIdentity {
    pub id: String,
    pub policy: u32,
    pub kind: AppliedIdentityKind,
    pub members: Vec<IdentityMember>,
    pub canonical_id: Option<AthleteId>,
    pub case_id: Option<String>,
    pub verdict_digest: Option<String>,
    pub observed_at: String,
}

type PersonKey = (&'static str, u64);

pub fn person_provider(namespace: &SourceNamespace) -> Option<&'static str> {
    match namespace {
        SourceNamespace::MilesplitAthlete => Some("milesplit"),
        SourceNamespace::TfrrsAthlete => Some("tfrrs"),
        SourceNamespace::DirectAthleticsAthlete => Some("direct_athletics"),
        SourceNamespace::AthleticNet { kind } if kind == "athlete" => Some("athleticnet"),
        _ => None,
    }
}

pub fn person_key(source: &super::SourceIdentity) -> Option<PersonKey> {
    let provider = person_provider(&source.namespace)?;
    let id = source.id.parse::<u64>().ok()?;
    (id > 0
        && source.id.bytes().all(|b| b.is_ascii_digit())
        && !source.id.starts_with('0'))
        .then_some((provider, id))
}


pub fn athlete_identity_digest(athlete: &CanonicalAthlete) -> Result<String, serde_json::Error> {
    serialized_digest(athlete)
}

pub fn identity_verdict_digest(verdict: &ReviewVerdictRecord) -> Result<String, serde_json::Error> {
    serialized_digest(verdict)
}
