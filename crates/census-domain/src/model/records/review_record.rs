use serde::{Deserialize, Serialize};

use crate::model::AthleteCandidateId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetainedConflict {
    pub id: String,
    pub family: String,
    pub subject_id: String,
    pub subject: String,
    pub detail: String,
}

impl RetainedConflict {
    pub fn new(
        family: &str,
        subject_id: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        let subject_id = subject_id.into();
        Self {
            id: format!("{family}:{subject_id}"),
            family: family.to_string(),
            subject_id,
            subject: subject.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    #[default]
    Pending,
    Resolved,
    Retained,
    Superseded,
}

pub const REVIEW_POLICY_REVISION: u32 = 2;

pub const COHORT_EVIDENCE_FAMILY: &str = "Class-of-2027 cohort evidence";
pub const ATHLETE_IDENTITY_FAMILY: &str = "Athlete identity";
pub const SCHOOL_IDENTITY_FAMILY: &str = "School identity";
pub const COHORT_UNVERIFIED_FAMILY: &str = "Class-of-2027 cohort unverified";
pub const IDENTITY_UNVERIFIED_FAMILY: &str = "Athlete identity unverified";

pub const COHORT_DECISION_FAMILIES: [&str; 2] =
    [COHORT_UNVERIFIED_FAMILY, IDENTITY_UNVERIFIED_FAMILY];
pub const UNRESOLVED_VENUE_FAMILY: &str = "Meet venue unresolved";
pub const UNRESOLVED_SCHOOL_FAMILY: &str = "School jurisdiction unresolved";
pub const CONTACT_CONFLICT_FAMILY: &str = "Recruiting contact conflict";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewCase {
    pub id: String,
    pub family: String,
    pub subject_id: String,
    pub subject: String,
    pub detail: String,
    pub state: ReviewState,
    pub member_ids: Vec<AthleteCandidateId>,
}

impl ReviewCase {
    pub fn matches_evidence(&self, evidence: &super::evidence::CaseEvidence) -> bool {
        if self.family.is_empty() || self.subject_id.is_empty() {
            return false;
        }
        let Some((binding, digest)) = self.id.rsplit_once(':') else {
            return false;
        };
        let Some((subject, policy)) = binding.rsplit_once(":p") else {
            return false;
        };
        let subject = subject
            .strip_prefix(self.family.as_str())
            .and_then(|s| s.strip_prefix(':'));
        subject == Some(self.subject_id.as_str())
            && !policy.starts_with('0')
            && policy.bytes().all(|byte| byte.is_ascii_digit())
            && policy.parse::<u32>().ok() == Some(REVIEW_POLICY_REVISION)
            && digest == evidence.digest()
    }

    pub fn pending(
        family: &str,
        subject_id: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        let subject = subject.into();
        let detail = detail.into();
        let evidence = super::evidence::CaseEvidence::of([subject.as_str(), detail.as_str()]);
        Self::pending_with_evidence(family, subject_id, subject, detail, evidence)
    }

    pub fn pending_with_evidence(
        family: &str,
        subject_id: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
        evidence: super::evidence::CaseEvidence,
    ) -> Self {
        let subject_id = subject_id.into();
        Self {
            id: format!(
                "{family}:{subject_id}:p{REVIEW_POLICY_REVISION}:{}",
                evidence.digest()
            ),
            family: family.to_string(),
            subject_id,
            subject: subject.into(),
            detail: detail.into(),
            state: ReviewState::Pending,
            member_ids: Vec::new(),
        }
    }

    pub fn minted(
        family: &str,
        subject_id: impl Into<String>,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        let mut case = Self::pending(family, subject_id, subject, detail);
        if Self::decided_by_its_own_rules(family) {
            case.state = ReviewState::Retained;
        }
        case
    }

    pub fn decided_by_its_own_rules(family: &str) -> bool {
        COHORT_DECISION_FAMILIES.contains(&family)
    }
}
