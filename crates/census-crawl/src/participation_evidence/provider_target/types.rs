use super::super::{hash, invalid, RetainedHtmlCaptureV1};
use census_domain::model::{Gender, GradYear, ProfileDate, SourceIdentity};
use census_store::StoreResult;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceTargetPacketRefV1(String);

impl SourceTargetPacketRefV1 {
    pub fn parse(value: &str) -> StoreResult<Self> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(invalid("provider target packet reference is not a SHA256"));
        }
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub struct ProviderTargetProofV1 {
    capture: String,
    interpretation: String,
    locator: String,
    acquired_at: DateTime<Utc>,
}

impl ProviderTargetProofV1 {
    pub(super) fn new(capture: &RetainedHtmlCaptureV1, locator: String) -> Self {
        let interpretation = hash(
            format!(
                "{}\n{}\n{}",
                super::INTERPRETATION,
                capture.body_sha256(),
                locator
            )
            .as_bytes(),
        );
        Self {
            capture: capture.body_sha256().into(),
            interpretation,
            locator,
            acquired_at: capture.acquired_at(),
        }
    }
    pub fn capture(&self) -> &str {
        &self.capture
    }
    pub fn interpretation(&self) -> &str {
        &self.interpretation
    }
    pub fn locator(&self) -> &str {
        &self.locator
    }
    pub fn acquired_at(&self) -> DateTime<Utc> {
        self.acquired_at
    }
}

#[derive(Debug)]
pub struct UnmappedPersonGenderV1 {
    pub(super) raw: Option<String>,
    pub(super) proof: ProviderTargetProofV1,
}

impl UnmappedPersonGenderV1 {
    pub fn raw(&self) -> Option<&str> {
        self.raw.as_deref()
    }
    pub fn source_scope(&self) -> &'static str {
        "schema_org_person_gender"
    }
    pub fn proof(&self) -> &ProviderTargetProofV1 {
        &self.proof
    }
}

#[derive(Debug)]
pub struct CheckedMileSplitProfileSubjectV1 {
    pub(super) person: SourceIdentity,
    pub(super) name: String,
    pub(super) school: SourceIdentity,
    pub(super) school_name: String,
    pub(super) cohort: GradYear,
    pub(super) locality: String,
    pub(super) person_gender: UnmappedPersonGenderV1,
    pub(super) proofs: Vec<ProviderTargetProofV1>,
}

impl CheckedMileSplitProfileSubjectV1 {
    pub fn person(&self) -> &SourceIdentity {
        &self.person
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn school(&self) -> &SourceIdentity {
        &self.school
    }
    pub fn school_name(&self) -> &str {
        &self.school_name
    }
    pub fn cohort(&self) -> GradYear {
        self.cohort
    }
    pub fn locality(&self) -> &str {
        &self.locality
    }
    pub fn person_gender(&self) -> &UnmappedPersonGenderV1 {
        &self.person_gender
    }
    pub fn competition_category(&self) -> Gender {
        Gender::Unknown
    }
    pub fn proofs(&self) -> &[ProviderTargetProofV1] {
        &self.proofs
    }
}

#[derive(Debug)]
pub struct CheckedLinkedCompetitionCategoryV1 {
    pub(super) raw: Option<String>,
    pub(super) category: Option<Gender>,
    pub(super) meet_id: String,
    pub(super) result_document_id: String,
    pub(super) result_document_url: String,
    pub(super) event_code: String,
    pub(super) mark: String,
    pub(super) meet_name: String,
    pub(super) meet_day: ProfileDate,
    pub(super) context: ProviderTargetProofV1,
    pub(super) school_context: ProviderTargetProofV1,
    pub(super) proof: ProviderTargetProofV1,
}

impl CheckedLinkedCompetitionCategoryV1 {
    pub fn raw(&self) -> Option<&str> {
        self.raw.as_deref()
    }
    pub fn category(&self) -> Option<Gender> {
        self.category
    }
    pub fn source_scope(&self) -> &'static str {
        "milesplit_meet_team_result_competition_category"
    }
    pub fn meet_id(&self) -> &str {
        &self.meet_id
    }
    pub fn result_document_id(&self) -> &str {
        &self.result_document_id
    }
    pub fn result_document_url(&self) -> &str {
        &self.result_document_url
    }
    pub fn event_code(&self) -> &str {
        &self.event_code
    }
    pub fn mark(&self) -> &str {
        &self.mark
    }
    pub fn meet_name(&self) -> &str {
        &self.meet_name
    }
    pub fn meet_day(&self) -> &ProfileDate {
        &self.meet_day
    }
    pub fn context_proof(&self) -> &ProviderTargetProofV1 {
        &self.context
    }
    pub fn school_proof(&self) -> &ProviderTargetProofV1 {
        &self.school_context
    }
    pub fn proof(&self) -> &ProviderTargetProofV1 {
        &self.proof
    }
}

#[derive(Debug)]
pub enum CompetitionCategoryEvidenceV1 {
    Unavailable {
        rows: Vec<CheckedLinkedCompetitionCategoryV1>,
    },
    Agreed {
        category: Gender,
        rows: Vec<CheckedLinkedCompetitionCategoryV1>,
    },
    Conflicted {
        rows: Vec<CheckedLinkedCompetitionCategoryV1>,
    },
}

impl CompetitionCategoryEvidenceV1 {
    pub fn rows(&self) -> &[CheckedLinkedCompetitionCategoryV1] {
        match self {
            Self::Unavailable { rows } | Self::Agreed { rows, .. } | Self::Conflicted { rows } => {
                rows
            }
        }
    }
}

#[derive(Debug)]
pub struct CheckedProviderTargetIntakeV1 {
    pub(super) reference: SourceTargetPacketRefV1,
    pub(super) profile: CheckedMileSplitProfileSubjectV1,
    pub(super) categories: CompetitionCategoryEvidenceV1,
}

impl CheckedProviderTargetIntakeV1 {
    pub fn reference(&self) -> &SourceTargetPacketRefV1 {
        &self.reference
    }
    pub fn profile(&self) -> &CheckedMileSplitProfileSubjectV1 {
        &self.profile
    }
    pub fn categories(&self) -> &CompetitionCategoryEvidenceV1 {
        &self.categories
    }
}
