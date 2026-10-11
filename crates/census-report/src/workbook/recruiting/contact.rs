mod heads;
mod normalise;
mod provenance;
mod school;
mod scoped;
#[cfg(test)]
mod tests;

use census_domain::model::{CanonicalCoach, SchoolYear, Sport};

pub(in crate::workbook) use normalise::{Named, Preferred};
pub(in crate::workbook) use school::{attach_research, contacts, SchoolContacts};
pub(in crate::workbook) use scoped::{scoped, ScopedContacts};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::workbook) enum ContactState {
    ProfessionalCoachEmail,
    ProfessionalAdEmail,
    PersonalCoachEmail,
    PersonalAdEmail,
    CoachNameOnly,
    AdNameOnly,
    ContactConflict,
    ContactResearchUnknown,
    ContactResearchEmpty,
    ContactResearchBlocked,
    ContactTenureConflict,
    ContactEvidenceInvalid,
}

impl ContactState {
    pub(in crate::workbook) const fn as_str(self) -> &'static str {
        match self {
            Self::ProfessionalCoachEmail => "professional_coach_email",
            Self::PersonalCoachEmail => "personal_coach_email",
            Self::ProfessionalAdEmail => "professional_ad_email",
            Self::PersonalAdEmail => "personal_ad_email",
            Self::CoachNameOnly => "coach_name_only",
            Self::AdNameOnly => "ad_name_only",
            Self::ContactConflict => "contact_conflict",
            Self::ContactResearchUnknown => "contact_research_unknown",
            Self::ContactResearchEmpty => "contact_research_empty",
            Self::ContactResearchBlocked => "contact_research_blocked",
            Self::ContactTenureConflict => "contact_tenure_conflict",
            Self::ContactEvidenceInvalid => "contact_evidence_invalid",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Slot {
    OutdoorTrack,
    IndoorTrack,
    CrossCountry,
    Director,
}

impl Slot {
    pub(super) const fn of(sport: Sport) -> Option<Self> {
        match sport {
            Sport::OutdoorTrack => Some(Self::OutdoorTrack),
            Sport::IndoorTrack => Some(Self::IndoorTrack),
            Sport::CrossCountry => Some(Self::CrossCountry),
            Sport::Unknown => None,
        }
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::OutdoorTrack => "Head Outdoor TF Coach",
            Self::IndoorTrack => "Head Indoor TF Coach",
            Self::CrossCountry => "Head XC Coach",
            Self::Director => "Athletic Director",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workbook) struct Disagreement {
    pub(in crate::workbook) school: String,
    pub(in crate::workbook) role: String,
    pub(in crate::workbook) state: ContactState,
    pub(in crate::workbook) rows: Vec<String>,
}

pub(in crate::workbook) fn disagreements(
    coaches: &[CanonicalCoach],
    school_year: SchoolYear,
) -> Vec<Disagreement> {
    contacts(coaches, school_year)
        .into_values()
        .flat_map(|facts| facts.heads.into_disagreements())
        .collect()
}
