use super::{AthleteCandidateId, AthleteId, SourceNamespace};
use serde::{Deserialize, Serialize};

pub const ATHLETE_IDENTITY_POLICY: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityDecisionIssue {
    WrongPolicy,
    InvalidMembership,
    StaleMemberEvidence,
    RetainedConflict,
    InvalidCanonicalTarget,
    CompetingSourceClaims,
    MissingResolvedCase,
    StaleCaseEvidence,
    MissingAcceptedVerdict,
    MissingPositiveIdentityEvidence,
    UnresolvedReview,
    ConflictingApplications,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerdictKind {
    SamePerson,
    DifferentPerson,
    Insufficient,
}

impl VerdictKind {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::SamePerson => "same_person",
            Self::DifferentPerson => "different_person",
            Self::Insufficient => "insufficient_evidence",
        }
    }
}

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

pub type PersonKey = (&'static str, u64);

fn validate_identity_membership(
    kind: AppliedIdentityKind,
    members: &[IdentityMember],
    canonical_id: Option<&AthleteId>,
) -> Option<String> {
    if kind == AppliedIdentityKind::SourceBound && members.len() != 1 {
        return Some(format!(
            "SourceBound identity requires exactly one member, found {}",
            members.len()
        ));
    }
    if kind == AppliedIdentityKind::SamePerson && members.len() < 2 {
        return Some(format!(
            "SamePerson identity requires at least two members, found {}",
            members.len()
        ));
    }
    if kind == AppliedIdentityKind::SamePerson && canonical_id.is_none() {
        return Some("SamePerson identity requires a canonical_id".to_string());
    }
    if kind == AppliedIdentityKind::DifferentPerson && canonical_id.is_some() {
        return Some("DifferentPerson identity must not have a canonical_id".to_string());
    }
    None
}

impl AppliedAthleteIdentity {
    pub fn new(
        id: &str,
        kind: AppliedIdentityKind,
        members: Vec<IdentityMember>,
        canonical_id: Option<AthleteId>,
        case_id: Option<String>,
        verdict_digest: Option<String>,
        observed_at: &str,
    ) -> Self {
        if let Some(error) = validate_identity_membership(kind, &members, canonical_id.as_ref()) {
            debug_assert!(false, "{error}");
        }
        Self {
            id: id.to_string(),
            policy: ATHLETE_IDENTITY_POLICY,
            kind,
            members,
            canonical_id,
            case_id,
            verdict_digest,
            observed_at: observed_at.to_string(),
        }
    }

    pub fn new_checked(
        id: &str,
        kind: AppliedIdentityKind,
        members: Vec<IdentityMember>,
        canonical_id: Option<AthleteId>,
        case_id: Option<String>,
        verdict_digest: Option<String>,
        observed_at: &str,
    ) -> Result<Self, String> {
        if let Some(error) = validate_identity_membership(kind, &members, canonical_id.as_ref()) {
            return Err(error);
        }
        Ok(Self {
            id: id.to_string(),
            policy: ATHLETE_IDENTITY_POLICY,
            kind,
            members,
            canonical_id,
            case_id,
            verdict_digest,
            observed_at: observed_at.to_string(),
        })
    }
}

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
    (id > 0 && source.id.bytes().all(|b| b.is_ascii_digit()) && !source.id.starts_with('0'))
        .then_some((provider, id))
}
