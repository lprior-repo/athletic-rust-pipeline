use std::collections::BTreeMap;

use census_domain::model::{
    assess_coach_tenure, CanonicalCoach, CoachId, CoachTenure, Gender, SchoolYear,
    TenureAssessmentError,
};

use super::normalise::{role_label, Named};
use super::{ContactState, Disagreement, Slot};
use crate::export::provenance::{current_coach_contacts, ContactSelectionError};

#[derive(Debug, Clone)]
pub(in crate::workbook::recruiting) struct Outcome(Result<Named, Unavailable>);

#[derive(Debug, Clone)]
enum Unavailable {
    Conflict,
    TenureConflict,
    InvalidEvidence,
    Unknown,
}

impl Outcome {
    pub(in crate::workbook::recruiting) const UNKNOWN: Self = Self(Err(Unavailable::Unknown));

    pub(in crate::workbook::recruiting) fn named(&self) -> Option<&Named> {
        self.0.as_ref().ok()
    }

    pub(super) fn blocker(&self) -> Option<ContactState> {
        match self.0 {
            Err(Unavailable::Conflict) => Some(ContactState::ContactConflict),
            Err(Unavailable::TenureConflict) => Some(ContactState::ContactTenureConflict),
            Err(Unavailable::InvalidEvidence) => Some(ContactState::ContactEvidenceInvalid),
            Ok(_) | Err(Unavailable::Unknown) => None,
        }
    }

    fn is_unknown(&self) -> bool {
        matches!(self.0, Err(Unavailable::Unknown))
    }

    fn unavailable(reason: Unavailable) -> Self {
        Self(Err(reason))
    }

    fn combine(self, other: Self) -> Self {
        use Unavailable::{Conflict, InvalidEvidence, TenureConflict, Unknown};
        Self(match (self.0, other.0) {
            (Err(TenureConflict), _) | (_, Err(TenureConflict)) => Err(TenureConflict),
            (Err(Conflict), _) | (_, Err(Conflict)) => Err(Conflict),
            (Ok(_), Ok(_)) => Err(Conflict),
            (current @ Ok(_), _) | (_, current @ Ok(_)) => current,
            (Err(InvalidEvidence), _) | (_, Err(InvalidEvidence)) => Err(InvalidEvidence),
            _ => Err(Unknown),
        })
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
        if let Some(state) = outcome.blocker().filter(|state| {
            matches!(
                state,
                ContactState::ContactConflict | ContactState::ContactTenureConflict
            )
        }) {
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
                if !outcome.is_unknown() {
                    return outcome;
                }
            }
        }
        self.scopes
            .get(&(slot, Gender::Mixed))
            .map_or(&Outcome::UNKNOWN, core::convert::identity)
    }

    pub(super) fn into_disagreements(self) -> Vec<Disagreement> {
        self.disagreements
    }
}

pub(super) fn resolve(rows: &[&CanonicalCoach], school_year: SchoolYear) -> Outcome {
    owners(rows)
        .into_values()
        .fold(Outcome::UNKNOWN, |outcome, owner| {
            outcome.combine(resolve_owner(&owner, school_year))
        })
}

pub(super) fn individuals(rows: &[&CanonicalCoach], school_year: SchoolYear) -> Vec<Named> {
    owners(rows)
        .into_values()
        .filter_map(|owner| resolve_owner(&owner, school_year).0.ok())
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
        Err(TenureAssessmentError::Conflict) => {
            return Outcome::unavailable(Unavailable::TenureConflict)
        }
        Err(TenureAssessmentError::InvalidEvidence { .. }) => {
            return Outcome::unavailable(Unavailable::InvalidEvidence)
        }
        Ok(CoachTenure::Former { .. } | CoachTenure::Unknown) => return Outcome::UNKNOWN,
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
                    return Outcome::unavailable(Unavailable::Conflict);
                }
            }
            None => match Named::of(coach, mailboxes) {
                Some(current) => named = Some(current),
                None => return Outcome::unavailable(Unavailable::InvalidEvidence),
            },
        }
    }
    Outcome(named.ok_or(Unavailable::Unknown))
}

fn selection_error(error: ContactSelectionError) -> Outcome {
    Outcome::unavailable(match error {
        ContactSelectionError::MailboxConflict => Unavailable::Conflict,
        ContactSelectionError::Tenure(TenureAssessmentError::Conflict) => {
            Unavailable::TenureConflict
        }
        ContactSelectionError::MissingCaptureUrl
        | ContactSelectionError::Tenure(TenureAssessmentError::InvalidEvidence { .. }) => {
            Unavailable::InvalidEvidence
        }
    })
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
