use super::{ContactState, Slot};
use census_domain::model::Gender;

/// One head coach a school's rows resolved to: the name, preferred published address, and side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workbook::recruiting) struct Named {
    pub(in crate::workbook::recruiting) name: String,
    pub(in crate::workbook::recruiting) email: Option<String>,
    /// A consumer mailbox the coach published but no professional address exists.
    pub(in crate::workbook::recruiting) personal_email: Option<String>,
    pub(in crate::workbook::recruiting) side: Gender,
}

impl Named {
    /// Only the professional address; personal mail does not masquerade as professional.
    pub(super) fn of(coach: &census_domain::model::CanonicalCoach) -> Self {
        Self {
            name: coach.name.clone(),
            email: coach.professional_email.clone(),
            personal_email: coach.personal_email.clone(),
            side: coach.gender,
        }
    }
}

/// The preferred contact one athlete's row publishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workbook::recruiting) struct Preferred {
    /// `Preferred Recruiting Contact`: the named contact, blank when the school names none.
    pub(in crate::workbook::recruiting) name: String,
    /// `Preferred Contact Role`: the slot the contact came from, blank when nothing is named.
    pub(in crate::workbook::recruiting) role: String,
    /// `Preferred Contact Email`: the published address, blank unless one was published.
    pub(in crate::workbook::recruiting) email: String,
    /// `Contact Coverage State`: never blank.
    pub(in crate::workbook::recruiting) state: ContactState,
}

impl Preferred {
    /// A contact that was found, with the state its rung publishes.
    pub(super) fn named(slot: Slot, contact: &Named, state: ContactState) -> Self {
        let email = match state {
            ContactState::PersonalCoachEmail | ContactState::PersonalAdEmail => {
                contact.personal_email.clone().unwrap_or_default()
            }
            _ => contact.email.clone().unwrap_or_default(),
        };
        Self {
            name: contact.name.clone(),
            role: role_label(slot, contact.side),
            email,
            state,
        }
    }

    /// No contact: the state alone, every cell blank.
    pub(super) fn unnamed(state: ContactState) -> Self {
        Self {
            name: String::new(),
            role: String::new(),
            email: String::new(),
            state,
        }
    }
}

/// The role cell: the slot's label, with the side of the team appended when the row was published
/// for one side rather than for the whole team.
pub(super) fn role_label(slot: Slot, side: Gender) -> String {
    match side {
        Gender::Boys => format!("{} (boys)", slot.label()),
        Gender::Girls => format!("{} (girls)", slot.label()),
        Gender::Mixed | Gender::Unknown => slot.label().to_string(),
    }
}
