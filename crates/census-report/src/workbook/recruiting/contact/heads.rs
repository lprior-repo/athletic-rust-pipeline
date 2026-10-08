use std::collections::BTreeMap;

use census_domain::model::{
    assess_coach_tenure, CanonicalCoach, CoachId, CoachTenure, Gender, SchoolYear,
    TenureAssessmentError,
};

use super::normalise::{role_label, Named};
use super::{ContactState, Disagreement, Slot};
use crate::export::provenance::{current_coach_contacts, ContactSelectionError};

#[derive(Debug, Default, Clone)]
pub(in crate::workbook::recruiting) enum Outcome {
    Current(Named),
    Conflict,
    TenureConflict,
    InvalidEvidence,
    #[default]
    Unknown,
}

impl Outcome {
    pub(in crate::workbook::recruiting) fn named(&self) -> Option<&Named> {
        match self {
            Self::Current(named) => Some(named),
            _ => None,
        }
    }

    pub(super) fn blocker(&self) -> Option<ContactState> {
        match self {
            Self::Conflict => Some(ContactState::ContactConflict),
            Self::TenureConflict => Some(ContactState::ContactTenureConflict),
            Self::InvalidEvidence => Some(ContactState::ContactEvidenceInvalid),
            Self::Current(_) | Self::Unknown => None,
        }
    }

    fn combine(self, other: Self) -> Self {
        match (self, other) {
            (Self::TenureConflict, _) | (_, Self::TenureConflict) => Self::TenureConflict,
            (Self::Conflict, _) | (_, Self::Conflict) => Self::Conflict,
            (Self::Current(_), Self::Current(_)) => Self::Conflict,
            (current @ Self::Current(_), _) | (_, current @ Self::Current(_)) => current,
            (Self::InvalidEvidence, _) | (_, Self::InvalidEvidence) => Self::InvalidEvidence,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub(in crate::workbook::recruiting) struct Heads {
    scopes: BTreeMap<(Slot, Gender), Outcome>,
    disagreements: Vec<Disagreement>,
}

pub(super) struct HeadScope<'a> {
    pub(super) school: &'a str,
    pub(super) slot: Slot,
    pub(super) side: Gender,
    pub(super) school_year: SchoolYear,
}

impl Heads {
    pub(super) fn insert(&mut self, scope: HeadScope<'_>, rows: &[&CanonicalCoach]) {
        let outcome = resolve(rows, scope.school_year);
        if matches!(outcome, Outcome::Conflict | Outcome::TenureConflict) {
            let state = match outcome {
                Outcome::TenureConflict => ContactState::ContactTenureConflict,
                _ => ContactState::ContactConflict,
            };
            self.disagreements.push(Disagreement {
                school: scope.school.to_owned(),
                role: role_label(scope.slot, scope.side),
                state,
                rows: describe(rows, scope.school_year),
            });
        }
        self.scopes.insert((scope.slot, scope.side), outcome);
    }

    pub(in crate::workbook::recruiting) fn conflicts(&self) -> usize {
        self.disagreements.len()
    }

    pub(super) fn resolve(&self, slot: Slot, side: Gender) -> &Outcome {
        if matches!(side, Gender::Boys | Gender::Girls) {
            if let Some(outcome) = self.scopes.get(&(slot, side)) {
                if !matches!(outcome, Outcome::Unknown) {
                    return outcome;
                }
            }
        }
        self.scopes
            .get(&(slot, Gender::Mixed))
            .map_or(&Outcome::Unknown, |value| value)
    }

    pub(super) fn into_disagreements(self) -> Vec<Disagreement> {
        self.disagreements
    }
}

pub(super) fn resolve(rows: &[&CanonicalCoach], school_year: SchoolYear) -> Outcome {
    owners(rows)
        .into_values()
        .fold(Outcome::Unknown, |outcome, owner| {
            outcome.combine(resolve_owner(&owner, school_year))
        })
}

pub(super) fn individuals(rows: &[&CanonicalCoach], school_year: SchoolYear) -> Vec<Named> {
    owners(rows)
        .into_values()
        .filter_map(|owner| match resolve_owner(&owner, school_year) {
            Outcome::Current(named) => Some(named),
            _ => None,
        })
        .collect()
}

fn owners<'a>(rows: &[&'a CanonicalCoach]) -> BTreeMap<&'a CoachId, Vec<&'a CanonicalCoach>> {
    let mut owners: BTreeMap<&CoachId, Vec<&CanonicalCoach>> = BTreeMap::new();
    for coach in rows {
        owners.entry(&coach.id).or_default().push(coach);
    }
    owners
}

fn resolve_owner(rows: &[&CanonicalCoach], school_year: SchoolYear) -> Outcome {
    let evidence = rows.iter().flat_map(|coach| &coach.tenure_evidence);
    match assess_coach_tenure(evidence, school_year) {
        Err(TenureAssessmentError::Conflict) => return Outcome::TenureConflict,
        Err(TenureAssessmentError::InvalidEvidence { .. }) => return Outcome::InvalidEvidence,
        Ok(CoachTenure::Former { .. } | CoachTenure::Unknown) => return Outcome::Unknown,
        Ok(CoachTenure::Current { .. }) => {}
    }
    resolve_current(rows, school_year)
}

fn resolve_current(rows: &[&CanonicalCoach], school_year: SchoolYear) -> Outcome {
    let mut named: Option<Named> = None;
    for coach in rows {
        let mailboxes = match current_coach_contacts(coach, school_year) {
            Ok(Some(mailboxes)) => mailboxes,
            Ok(None) => continue,
            Err(error) => return selection_error(error),
        };
        match named.as_mut() {
            Some(named) => {
                if !named.merge(coach, mailboxes) {
                    return Outcome::Conflict;
                }
            }
            None => match Named::of(coach, mailboxes) {
                Some(current) => named = Some(current),
                None => return Outcome::InvalidEvidence,
            },
        }
    }
    named.map_or(Outcome::Unknown, Outcome::Current)
}

fn selection_error(error: ContactSelectionError) -> Outcome {
    match error {
        ContactSelectionError::MailboxConflict => Outcome::Conflict,
        ContactSelectionError::Tenure(TenureAssessmentError::Conflict) => Outcome::TenureConflict,
        ContactSelectionError::MissingCaptureUrl
        | ContactSelectionError::Tenure(TenureAssessmentError::InvalidEvidence { .. }) => {
            Outcome::InvalidEvidence
        }
    }
}

fn describe(rows: &[&CanonicalCoach], school_year: SchoolYear) -> Vec<String> {
    let mut descriptions: Vec<_> = rows
        .iter()
        .map(|coach| {
            format!(
                "{} [{}]: professional={:?}; personal={:?}; tenure={:?}",
                coach.name,
                coach.id,
                coach.professional_email,
                coach.personal_email,
                coach.tenure_state(school_year)
            )
        })
        .collect();
    descriptions.sort();
    descriptions
}
