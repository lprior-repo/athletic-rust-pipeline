mod claims;
mod heads;
mod normalise;
mod school;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use census_domain::model::{CanonicalAthlete, CanonicalCoach, Gender, SchoolYear, Sport};
use heads::Outcome;

pub(in crate::workbook) use normalise::{Named, Preferred};
pub(in crate::workbook) use school::{contacts, SchoolContacts};

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
    pub(super) const fn of(sport: Sport) -> Self {
        match sport {
            Sport::OutdoorTrack => Self::OutdoorTrack,
            Sport::IndoorTrack => Self::IndoorTrack,
            Sport::CrossCountry => Self::CrossCountry,
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

pub(in crate::workbook) struct ScopedContacts<'a> {
    outdoor: &'a Outcome,
    indoor: &'a Outcome,
    cross_country: &'a Outcome,
    director: &'a Outcome,
    assistants: &'a [Named],
    athlete: &'a CanonicalAthlete,
}

pub(in crate::workbook) fn scoped<'a>(
    school: Option<&'a SchoolContacts>,
    athlete: &'a CanonicalAthlete,
) -> ScopedContacts<'a> {
    let school = school.filter(|school| school.school == athlete.school);
    let resolve = |sport| {
        if !athlete.sports.contains(&sport) {
            return &Outcome::Unknown;
        }
        school
            .map(|school| school.heads.resolve(Slot::of(sport), athlete.gender))
            .map_or(&Outcome::Unknown, |value| value)
    };
    ScopedContacts {
        outdoor: resolve(Sport::OutdoorTrack),
        indoor: resolve(Sport::IndoorTrack),
        cross_country: resolve(Sport::CrossCountry),
        director: school
            .map(|school| school.heads.resolve(Slot::Director, Gender::Mixed))
            .map_or(&Outcome::Unknown, |value| value),
        assistants: school
            .map(|school| school.assistants.as_slice())
            .map_or(&[][..], |value| value),
        athlete,
    }
}

impl ScopedContacts<'_> {
    fn heads(&self) -> impl Iterator<Item = (Slot, &Outcome)> {
        [
            (Slot::OutdoorTrack, self.outdoor),
            (Slot::IndoorTrack, self.indoor),
            (Slot::CrossCountry, self.cross_country),
        ]
        .into_iter()
    }

    pub(in crate::workbook) fn preferred(&self) -> Preferred {
        let mut named = None;
        let mut blocker = None;
        for (slot, outcome) in self.heads() {
            if let Some(coach) = outcome.named() {
                if coach.address().is_some() {
                    return Preferred::named(slot, coach);
                }
                if named.is_none() {
                    named = Some((slot, coach));
                }
            }
            blocker = blocker.or(outcome.blocker());
        }
        if named.is_none() {
            if let Some(state) = blocker {
                return Preferred::unnamed(state);
            }
        }
        if let Some(director) = self.director() {
            if director.address().is_some() || named.is_none() {
                return Preferred::named(Slot::Director, director);
            }
        }
        if let Some((slot, coach)) = named {
            return Preferred::named(slot, coach);
        }
        Preferred::unnamed(
            self.director
                .blocker()
                .map_or(ContactState::ContactResearchUnknown, |value| value),
        )
    }

    pub(in crate::workbook) fn track_names(&self) -> Option<String> {
        self.track_field(|coach| Some(coach.name.as_str()))
    }

    pub(in crate::workbook) fn track_emails(&self) -> Option<String> {
        self.track_field(Named::address)
    }

    fn track_field(&self, field: fn(&Named) -> Option<&str>) -> Option<String> {
        let multiple = self.outdoor.named().is_some() && self.indoor.named().is_some();
        let mut text = String::new();
        for (slot, outcome) in [
            (Slot::OutdoorTrack, self.outdoor),
            (Slot::IndoorTrack, self.indoor),
        ] {
            let Some(value) = outcome.named().and_then(field) else {
                continue;
            };
            if !text.is_empty() {
                text.push_str("; ");
            }
            if multiple {
                text.push_str(slot.label());
                text.push_str(": ");
            }
            text.push_str(value);
        }
        (!text.is_empty()).then_some(text)
    }

    pub(in crate::workbook) fn cross_country(&self) -> Option<&Named> {
        self.cross_country.named()
    }

    pub(in crate::workbook) fn director(&self) -> Option<&Named> {
        self.director.named()
    }

    pub(in crate::workbook) fn professional_coach_email(&self) -> Option<&str> {
        self.heads()
            .find_map(|(_, outcome)| outcome.named()?.email.as_deref())
    }

    pub(in crate::workbook) fn all_emails(&self) -> String {
        let assistants = self.assistants.iter().filter(|coach| {
            coach
                .sport
                .is_some_and(|sport| self.athlete.sports.contains(&sport))
                && matches_side(coach.side, self.athlete.gender)
        });
        let eligible = self
            .heads()
            .filter_map(|(_, outcome)| outcome.named())
            .chain(self.director())
            .chain(assistants);
        let mut addresses = BTreeSet::new();
        for coach in eligible {
            addresses.extend(
                coach
                    .email
                    .iter()
                    .chain(&coach.personal_email)
                    .map(String::as_str),
            );
        }
        let mut text = String::new();
        for address in addresses {
            if !text.is_empty() {
                text.push_str("; ");
            }
            text.push_str(address);
        }
        text
    }
}

fn matches_side(coach: Gender, athlete: Gender) -> bool {
    matches!(coach, Gender::Mixed)
        || matches!(
            (coach, athlete),
            (Gender::Boys, Gender::Boys) | (Gender::Girls, Gender::Girls)
        )
}
