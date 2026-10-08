use census_domain::model::{CanonicalCoach, Gender, Sport};

use super::provenance::{self, ContactProvenance};
use super::{ContactState, Slot};
use crate::export::provenance::{CurrentCoachContacts, SelectedContact};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workbook) struct Named {
    pub(in crate::workbook) name: String,
    pub(in crate::workbook) email: Option<String>,
    pub(in crate::workbook::recruiting) personal_email: Option<String>,
    pub(in crate::workbook::recruiting) side: Gender,
    pub(in crate::workbook::recruiting) sport: Option<Sport>,
    professional_source: Option<ContactProvenance>,
    personal_source: Option<ContactProvenance>,
    name_source: Option<ContactProvenance>,
}

impl Named {
    pub(super) fn of(coach: &CanonicalCoach, mailboxes: CurrentCoachContacts<'_>) -> Option<Self> {
        if coach.name.trim().is_empty() {
            return None;
        }
        let (email, professional_source) = owned_mailbox(mailboxes.professional());
        let (personal_email, personal_source) = owned_mailbox(mailboxes.personal());
        Some(Self {
            name: coach.name.clone(),
            email,
            personal_email,
            sport: coach.sport,
            side: coach.gender,
            professional_source,
            personal_source,
            name_source: mailboxes
                .name_only()
                .map(|fact| ContactProvenance::of(&coach.id, fact)),
        })
    }

    pub(super) fn merge(
        &mut self,
        coach: &CanonicalCoach,
        mailboxes: CurrentCoachContacts<'_>,
    ) -> bool {
        if self.conflicts(mailboxes) {
            return false;
        }
        if coach.name < self.name {
            self.name.clone_from(&coach.name);
        }
        self.merge_claims(coach, mailboxes);
        true
    }

    fn merge_claims(&mut self, coach: &CanonicalCoach, mailboxes: CurrentCoachContacts<'_>) {
        merge_mailbox(
            &mut self.email,
            &mut self.professional_source,
            mailboxes.professional(),
        );
        merge_mailbox(
            &mut self.personal_email,
            &mut self.personal_source,
            mailboxes.personal(),
        );
        if let Some(fact) = mailboxes.name_only() {
            provenance::merge(&mut self.name_source, &coach.id, fact);
        }
    }

    fn conflicts(&self, mailboxes: CurrentCoachContacts<'_>) -> bool {
        differs(
            self.email.as_deref(),
            mailboxes.professional().map(SelectedContact::mailbox),
        ) || differs(
            self.personal_email.as_deref(),
            mailboxes.personal().map(SelectedContact::mailbox),
        )
    }

    pub(in crate::workbook) fn address(&self) -> Option<&str> {
        self.email.as_deref().or(self.personal_email.as_deref())
    }

    pub(in crate::workbook) fn professional_source(&self) -> Option<&ContactProvenance> {
        self.professional_source.as_ref()
    }

    pub(in crate::workbook) fn personal_source(&self) -> Option<&ContactProvenance> {
        self.personal_source.as_ref()
    }

    pub(in crate::workbook) fn name_source(&self) -> Option<&ContactProvenance> {
        self.name_source.as_ref()
    }

    pub(in crate::workbook) fn source(&self) -> Option<&ContactProvenance> {
        self.professional_source()
            .or(self.personal_source())
            .or(self.name_source())
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

fn differs(left: Option<&str>, right: Option<&str>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if left != right)
}

fn owned_mailbox(
    selected: Option<SelectedContact<'_>>,
) -> (Option<String>, Option<ContactProvenance>) {
    (
        selected.map(|selected| selected.mailbox().to_owned()),
        selected.map(ContactProvenance::mailbox),
    )
}

fn merge_mailbox(
    address: &mut Option<String>,
    source: &mut Option<ContactProvenance>,
    selected: Option<SelectedContact<'_>>,
) {
    if address.is_none() {
        *address = selected.map(|selected| selected.mailbox().to_owned());
    }
    if let Some(selected) = selected {
        provenance::merge(source, &selected.claim().coach, selected.tenure());
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::workbook) struct Preferred<'a> {
    pub(in crate::workbook) name: &'a str,
    pub(in crate::workbook) role: String,
    pub(in crate::workbook) email: &'a str,
    pub(in crate::workbook) state: ContactState,
    pub(in crate::workbook) source_url: &'a str,
    pub(in crate::workbook) coach_id: &'a str,
    pub(in crate::workbook) observed_on: &'a str,
    pub(in crate::workbook) source_sha256: &'a str,
}

impl<'a> Preferred<'a> {
    pub(super) fn named(slot: Slot, contact: &'a Named) -> Self {
        let source = contact.source();
        Self {
            name: &contact.name,
            role: role_label(slot, contact.side),
            email: contact.address().map_or("", |address| address),
            state: contact.state(slot),
            source_url: source.map_or("", |source| source.source_url.as_str()),
            coach_id: source.map_or("", |source| source.coach_id.as_str()),
            observed_on: source.map_or("", |source| source.observed_on.as_str()),
            source_sha256: source.map_or("", |source| source.source_sha256.as_str()),
        }
    }

    pub(super) fn unnamed(state: ContactState) -> Self {
        Self {
            name: "",
            role: String::new(),
            email: "",
            state,
            source_url: "",
            coach_id: "",
            observed_on: "",
            source_sha256: "",
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
