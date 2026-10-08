use crate::export::provenance::{current_coach_contacts, CurrentCoachContacts, SelectedContact};
use census_domain::model::{CanonicalCoach, CoachTenureEvidence, SchoolYear};

pub(in crate::workbook) struct Projection<'a> {
    pub professional: Option<SelectedContact<'a>>,
    pub personal: Option<SelectedContact<'a>>,
    pub name: Option<&'a CoachTenureEvidence>,
    pub state: &'static str,
}

fn of(coach: &CanonicalCoach, year: SchoolYear) -> Projection<'_> {
    match current_coach_contacts(coach, year) {
        Ok(Some(selected)) => current(selected),
        Ok(None) => empty("no_current_claim"),
        Err(crate::export::provenance::ContactSelectionError::MailboxConflict) => {
            empty("mailbox_conflict")
        }
        Err(crate::export::provenance::ContactSelectionError::MissingCaptureUrl) => {
            empty("missing_capture_url")
        }
        Err(crate::export::provenance::ContactSelectionError::Tenure(_)) => {
            empty("invalid_or_conflicting_tenure")
        }
    }
}

pub(in crate::workbook) fn admitted<'a>(
    coach: &'a CanonicalCoach,
    year: SchoolYear,
    school: Option<&super::contact::SchoolContacts>,
) -> Projection<'a> {
    let selected = of(coach, year);
    if selected.state != "current_claim"
        || school.is_some_and(|school| school.coach_admitted(coach))
    {
        selected
    } else {
        empty("school_program_contact_withheld")
    }
}

fn current(selected: CurrentCoachContacts<'_>) -> Projection<'_> {
    Projection {
        professional: selected.professional(),
        personal: selected.personal(),
        name: selected.name_only(),
        state: "current_claim",
    }
}

fn empty(state: &'static str) -> Projection<'static> {
    Projection {
        professional: None,
        personal: None,
        name: None,
        state,
    }
}

pub(in crate::workbook) fn capture(fact: Option<&CoachTenureEvidence>) -> [&str; 3] {
    match fact {
        Some(fact) => [
            fact.source.url.as_deref().map_or("", |url| url),
            fact.source_sha256.as_str(),
            fact.retrieved_at.as_str(),
        ],
        None => ["", "", ""],
    }
}
