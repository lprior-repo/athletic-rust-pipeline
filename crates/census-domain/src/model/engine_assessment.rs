use super::engine_valid::{refuse_date, refuse_evidence, refuse_id, MAX_LOGICAL_ID_BYTES};
use super::*;
use crate::DomainError;

pub const COHORT_ASSESSMENT_POLICY: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CohortVerdict {
    Member2027,
    NotMember,
    NeedsReview,
}

impl CohortVerdict {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Member2027 => "member2027",
            Self::NotMember => "not_member",
            Self::NeedsReview => "needs_review",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CohortAssessment {
    id: String,
    subject_id: String,
    grad_year: GradYear,
    verdict: CohortVerdict,
    policy: u32,
    evidence_digest: String,
    observed_at: String,
    supersedes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawAssessment {
    id: String,
    subject_id: String,
    grad_year: GradYear,
    verdict: CohortVerdict,
    policy: u32,
    evidence_digest: String,
    observed_at: String,
    supersedes: Option<String>,
}

fn assessment_id(subject_id: &str, verdict: CohortVerdict, policy: u32, evidence: &str) -> String {
    format!(
        "cohort:{subject_id}:{}:p{policy}:{evidence}",
        verdict.slug()
    )
}

fn check(
    subject_id: &str,
    grad_year: GradYear,
    verdict: CohortVerdict,
    policy: u32,
    evidence: &str,
    observed_at: &str,
) -> Result<(), DomainError> {
    refuse_id("subject_id", subject_id)?;
    if policy != COHORT_ASSESSMENT_POLICY {
        return Err(DomainError::Unsupported { field: "policy" });
    }
    refuse_evidence(evidence)?;
    refuse_date("observed_at", observed_at)?;
    match verdict {
        CohortVerdict::Member2027 if grad_year != GradYear::CO2027 => {
            return Err(DomainError::Unsupported { field: "grad_year" });
        }
        CohortVerdict::NotMember if grad_year == GradYear::CO2027 => {
            return Err(DomainError::Unsupported { field: "grad_year" });
        }
        CohortVerdict::Member2027 | CohortVerdict::NotMember | CohortVerdict::NeedsReview => {}
    }
    Ok(())
}

impl CohortAssessment {
    pub fn new(
        subject_id: String,
        grad_year: GradYear,
        verdict: CohortVerdict,
        evidence_digest: String,
        observed_at: String,
    ) -> Result<Self, DomainError> {
        check(
            &subject_id,
            grad_year,
            verdict,
            COHORT_ASSESSMENT_POLICY,
            &evidence_digest,
            &observed_at,
        )?;
        let id = assessment_id(
            &subject_id,
            verdict,
            COHORT_ASSESSMENT_POLICY,
            &evidence_digest,
        );
        refuse_id("id", &id)?;
        Ok(Self {
            id,
            subject_id,
            grad_year,
            verdict,
            policy: COHORT_ASSESSMENT_POLICY,
            evidence_digest,
            observed_at,
            supersedes: None,
        })
    }
    pub fn supersede(
        &self,
        verdict: CohortVerdict,
        grad_year: GradYear,
        evidence_digest: String,
        observed_at: String,
    ) -> Result<Self, DomainError> {
        let mut next = Self::new(
            self.subject_id.clone(),
            grad_year,
            verdict,
            evidence_digest,
            observed_at,
        )?;
        next.supersedes = Some(self.id.clone());
        Ok(next)
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn subject_id(&self) -> &str {
        &self.subject_id
    }
    pub fn grad_year(&self) -> GradYear {
        self.grad_year
    }
    pub fn verdict(&self) -> CohortVerdict {
        self.verdict
    }
    pub fn policy(&self) -> u32 {
        self.policy
    }
    pub fn evidence_digest(&self) -> &str {
        &self.evidence_digest
    }
    pub fn observed_at(&self) -> &str {
        &self.observed_at
    }
    pub fn supersedes(&self) -> Option<&str> {
        self.supersedes.as_deref()
    }
    pub fn is_terminal_member(&self) -> bool {
        self.verdict != CohortVerdict::NeedsReview
    }
}

impl<'de> Deserialize<'de> for CohortAssessment {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawAssessment::deserialize(deserializer)?;
        check(
            &raw.subject_id,
            raw.grad_year,
            raw.verdict,
            raw.policy,
            &raw.evidence_digest,
            &raw.observed_at,
        )
        .map_err(serde::de::Error::custom)?;
        let expected = assessment_id(
            &raw.subject_id,
            raw.verdict,
            raw.policy,
            &raw.evidence_digest,
        );
        if expected != raw.id {
            return Err(serde::de::Error::custom(
                "assessment id does not match its fields",
            ));
        }
        if raw.id.len() > MAX_LOGICAL_ID_BYTES {
            return Err(serde::de::Error::custom(
                "assessment id is outside its permitted range",
            ));
        }
        Ok(Self {
            id: raw.id,
            subject_id: raw.subject_id,
            grad_year: raw.grad_year,
            verdict: raw.verdict,
            policy: raw.policy,
            evidence_digest: raw.evidence_digest,
            observed_at: raw.observed_at,
            supersedes: raw.supersedes,
        })
    }
}
