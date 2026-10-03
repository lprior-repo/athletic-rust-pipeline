use census_domain::model::{CanonicalCoach, Gender, Sport};

use super::{ContactState, Slot};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workbook) struct Named {
    pub(in crate::workbook) name: String,
    pub(in crate::workbook) email: Option<String>,
    pub(in crate::workbook::recruiting) personal_email: Option<String>,
    pub(in crate::workbook::recruiting) side: Gender,
    pub(in crate::workbook::recruiting) sport: Option<Sport>,
    source: Option<(String, String)>,
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
            source: provenance(coach),
        })
    }

    pub(super) fn merge(&mut self, coach: &CanonicalCoach) -> bool {
        if differs(self.email.as_deref(), nonempty(&coach.professional_email))
            || differs(
                self.personal_email.as_deref(),
                nonempty(&coach.personal_email),
            )
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
        let source = provenance(coach);
        if source.as_ref().is_some_and(|next| {
            self.source
                .as_ref()
                .is_none_or(|current| (&next.1, &next.0) > (&current.1, &current.0))
        }) {
            self.source = source;
        }
        true
    }

    pub(in crate::workbook) fn address(&self) -> Option<&str> {
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

fn provenance(coach: &CanonicalCoach) -> Option<(String, String)> {
    let (url, observed_on) = crate::export::coach_source(coach);
    url.map(|url| {
        (
            url.to_owned(),
            observed_on
                .map_or(Default::default(), core::convert::identity)
                .to_owned(),
        )
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workbook) struct Preferred {
    pub(in crate::workbook) name: String,
    pub(in crate::workbook) role: String,
    pub(in crate::workbook) email: String,
    pub(in crate::workbook) state: ContactState,
    pub(in crate::workbook) source_url: String,
}

impl Preferred {
    pub(super) fn named(slot: Slot, contact: &Named) -> Self {
        Self {
            name: contact.name.clone(),
            role: role_label(slot, contact.side),
            email: contact
                .address()
                .map_or(Default::default(), core::convert::identity)
                .to_owned(),
            state: contact.state(slot),
            source_url: contact
                .source
                .as_ref()
                .map(|source| source.0.clone())
                .map_or(Default::default(), core::convert::identity),
        }
    }

    pub(super) fn unnamed(state: ContactState) -> Self {
        Self {
            name: String::new(),
            role: String::new(),
            email: String::new(),
            state,
            source_url: String::new(),
        }
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
