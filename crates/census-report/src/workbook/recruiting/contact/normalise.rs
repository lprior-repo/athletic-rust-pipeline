use census_domain::model::{CanonicalCoach, Gender, Sport};

use super::{ContactState, Slot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workbook::recruiting) struct Named {
    pub(in crate::workbook::recruiting) name: String,
    pub(in crate::workbook::recruiting) email: Option<String>,
    pub(in crate::workbook::recruiting) personal_email: Option<String>,
    pub(in crate::workbook::recruiting) side: Gender,
    pub(in crate::workbook::recruiting) sport: Option<Sport>,
}

impl Named {
    pub(super) fn of(coach: &CanonicalCoach) -> Option<Self> {
        if coach.name.trim().is_empty() {
            return None;
        }
        Some(Self {
            name: coach.name.clone(),
            email: nonempty(&coach.professional_email).map(str::to_owned),
            personal_email: nonempty(&coach.personal_email).map(str::to_owned),
            sport: coach.sport,
            side: coach.gender,
        })
    }

    pub(super) fn merge(&mut self, coach: &CanonicalCoach) -> bool {
        if differs(self.email.as_deref(), nonempty(&coach.professional_email))
            || differs(self.personal_email.as_deref(), nonempty(&coach.personal_email))
        {
            return false;
        }
        if coach.name < self.name {
            self.name.clone_from(&coach.name);
        }
        if self.email.is_none() {
            self.email = nonempty(&coach.professional_email).map(str::to_owned);
        }
        if self.personal_email.is_none() {
            self.personal_email = nonempty(&coach.personal_email).map(str::to_owned);
        }
        true
    }

    pub(in crate::workbook::recruiting) fn address(&self) -> Option<&str> {
        self.email.as_deref().or(self.personal_email.as_deref())
    }

    pub(super) fn state(&self, slot: Slot) -> ContactState {
        match (slot, self.email.is_some(), self.personal_email.is_some()) {
            (Slot::Director, true, _) => ContactState::ProfessionalAdEmail,
            (Slot::Director, false, true) => ContactState::PersonalAdEmail,
            (Slot::Director, false, false) => ContactState::AdNameOnly,
            (_, true, _) => ContactState::ProfessionalCoachEmail,
            (_, false, true) => ContactState::PersonalCoachEmail,
            (_, false, false) => ContactState::CoachNameOnly,
        }
    }
}

fn nonempty(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|value| !value.trim().is_empty())
}

fn differs(left: Option<&str>, right: Option<&str>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if left != right)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workbook::recruiting) struct Preferred {
    pub(in crate::workbook::recruiting) name: String,
    pub(in crate::workbook::recruiting) role: String,
    pub(in crate::workbook::recruiting) email: String,
    pub(in crate::workbook::recruiting) state: ContactState,
}

impl Preferred {
    pub(super) fn named(slot: Slot, contact: &Named) -> Self {
        Self {
            name: contact.name.clone(),
            role: role_label(slot, contact.side),
            email: contact.address().unwrap_or_default().to_owned(),
            state: contact.state(slot),
        }
    }

    pub(super) fn unnamed(state: ContactState) -> Self {
        Self { name: String::new(), role: String::new(), email: String::new(), state }
    }
}

pub(super) fn role_label(slot: Slot, side: Gender) -> String {
    match (slot, side) {
        (Slot::Director, _) => slot.label().to_owned(),
        (_, Gender::Boys) => format!("{} (boys)", slot.label()),
        (_, Gender::Girls) => format!("{} (girls)", slot.label()),
        _ => slot.label().to_owned(),
    }
}
