use std::collections::BTreeMap;

use census_domain::model::{
    assess_coach_tenure, CanonicalCoach, CoachId, CoachTenure, Gender, SchoolYear,
    TenureAssessmentError,
};

use super::normalise::{role_label, Named};
use super::{ContactState, Disagreement, Slot};

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

impl Heads {
    pub(super) fn insert(
        &mut self,
        school: &str,
        slot: Slot,
        side: Gender,
        rows: &[&CanonicalCoach],
        school_year: SchoolYear,
    ) {
        let outcome = resolve(rows, school_year);
        if matches!(outcome, Outcome::Conflict | Outcome::TenureConflict) {
            let state = match outcome {
                Outcome::TenureConflict => ContactState::ContactTenureConflict,
                _ => ContactState::ContactConflict,
            };
            self.disagreements.push(Disagreement {
                school: school.to_owned(),
                role: role_label(slot, side),
                state,
                rows: describe(rows, school_year),
            });
        }
        self.scopes.insert((slot, side), outcome);
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
            .unwrap_or(&Outcome::Unknown)
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
    let mut named: Option<Named> = None;
    for coach in rows.iter().filter(|coach| current_row(coach, school_year)) {
        match named.as_mut() {
            Some(named) => {
                if !named.merge(coach) {
                    return Outcome::Conflict;
                }
            }
            None => {
                let Some(current) = Named::of(coach) else {
                    return Outcome::InvalidEvidence;
                };
                named = Some(current);
            }
        }
    }
    match named {
        Some(named) => Outcome::Current(named),
        None => Outcome::InvalidEvidence,
    }
}

fn current_row(coach: &CanonicalCoach, school_year: SchoolYear) -> bool {
    coach.tenure_evidence.iter().any(|fact| {
        matches!(fact.tenure, CoachTenure::Current { school_year: year } if year == school_year)
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
