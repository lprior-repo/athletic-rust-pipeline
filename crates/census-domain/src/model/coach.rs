use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalCoach {
    pub id: CoachId,
    pub name: String,
    pub school: SchoolId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sport: Option<Sport>,
    pub gender: Gender,
    pub role: CoachRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub professional_email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personal_email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
    pub source_identities: Vec<SourceIdentity>,
    pub evidence: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tenure_evidence: Vec<contact_tenure::CoachTenureEvidence>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retained_conflicts: Vec<RetainedConflict>,
}

impl CanonicalCoach {
    pub fn new(
        school: &SchoolId,
        name: impl Into<String>,
        sport: Option<Sport>,
        gender: Gender,
        role: CoachRole,
    ) -> Self {
        let name = name.into();
        let sport_key = match sport {
            Some(sport) => format!("Some({})", sport.stable_key()),
            None => "None".to_string(),
        };
        let id = Id::mint(
            "coa",
            &[
                school.as_str(),
                &normalize_name(&name),
                &sport_key,
                gender.stable_key(),
                role.stable_key(),
            ],
        );
        Self {
            id,
            name,
            school: school.clone(),
            sport,
            gender,
            role,
            professional_email: None,
            personal_email: None,
            phone: None,
            source_identities: Vec::new(),
            evidence: Vec::new(),
            tenure_evidence: Vec::new(),
            retained_conflicts: Vec::new(),
        }
    }

    pub fn has_published_email(&self) -> bool {
        self.professional_email.is_some() || self.personal_email.is_some()
    }

    pub fn set_published_email(&mut self, address: &str) {
        match published_email(address) {
            Some((address, MailboxKind::Professional)) if self.professional_email.is_none() => {
                self.professional_email = Some(address);
            }
            Some((address, MailboxKind::Personal)) if self.personal_email.is_none() => {
                self.personal_email = Some(address);
            }
            _ => {}
        }
    }

    pub fn tenure_state(
        &self,
        school_year: SchoolYear,
    ) -> Result<contact_tenure::CoachTenure, contact_tenure::TenureAssessmentError> {
        contact_tenure::assess_coach_tenure(&self.tenure_evidence, school_year)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachRole {
    HeadCoach,
    AssistantCoach,
    AthleticDirector,
    Unknown,
}

impl CoachRole {
    pub const fn stable_key(self) -> &'static str {
        match self {
            Self::HeadCoach => "HeadCoach",
            Self::AssistantCoach => "AssistantCoach",
            Self::AthleticDirector => "AthleticDirector",
            Self::Unknown => "Unknown",
        }
    }
}
