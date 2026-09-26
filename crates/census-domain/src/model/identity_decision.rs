//! Evidence-bound Rust decisions. Source observations and result owners are never rewritten.

use super::{AthleteCandidateId, AthleteId, CanonicalAthlete, EvidenceMethod, ReviewVerdictRecord, SourceNamespace};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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

/// An applied decision is separate from a proposed/admitted model answer. A changed member,
/// contradiction, review state, or verdict invalidates this record in the read projection.
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

/// A document row or meet entry is not a durable person identifier.
pub fn person_provider(namespace: &SourceNamespace) -> Option<&'static str> {
    match namespace {
        SourceNamespace::MilesplitAthlete => Some("milesplit"),
        SourceNamespace::TfrrsAthlete => Some("tfrrs"),
        SourceNamespace::DirectAthleticsAthlete => Some("direct_athletics"),
        SourceNamespace::AthleticNet { kind } | SourceNamespace::LegacyAthleticNet { kind }
            if kind == "athlete" => Some("athleticnet"),
        _ => None,
    }
}

pub fn has_person_source(athlete: &CanonicalAthlete) -> bool {
    athlete.source_owner().is_some_and(|source| person_provider(&source.namespace).is_some())
        && athlete.evidence.iter().any(|evidence|
            matches!(evidence.method, EvidenceMethod::Fetched | EvidenceMethod::Parsed)
                && evidence.source.url.as_ref().is_some_and(|url| !url.is_empty()))
}

/// Positive identity support, not merely absence of a contradiction or agreement about a grade.
pub fn shares_person_source(left: &CanonicalAthlete, right: &CanonicalAthlete) -> bool {
    left.source_identities.iter().any(|a| {
        let Some(provider) = person_provider(&a.namespace) else { return false };
        right.source_identities.iter().any(|b|
            person_provider(&b.namespace) == Some(provider) && a.id == b.id && !a.id.is_empty())
    })
}

pub fn athlete_identity_digest(athlete: &CanonicalAthlete) -> Result<String, serde_json::Error> {
    digest(&(&athlete.id, &athlete.canonical_name, &athlete.school, athlete.grad_year,
        athlete.gender, &athlete.source_identities, &athlete.observed_grades,
        &athlete.evidence, &athlete.retained_conflicts))
}

pub fn identity_verdict_digest(verdict: &ReviewVerdictRecord) -> Result<String, serde_json::Error> {
    digest(verdict)
}

fn digest(value: &impl Serialize) -> Result<String, serde_json::Error> {
    let mut writer = DigestWriter(Sha256::new());
    serde_json::to_writer(&mut writer, value)?;
    Ok(format!("{:x}", writer.0.finalize()))
}

struct DigestWriter(Sha256);

impl std::io::Write for DigestWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}
