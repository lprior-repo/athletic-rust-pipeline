mod heads;
mod normalise;
mod provenance;
mod school;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use census_domain::model::{CanonicalAthlete, CanonicalCoach, Gender, SchoolYear, Sport};
use heads::Outcome;

pub(in crate::workbook) use normalise::{Named, Preferred};
pub(in crate::workbook) use provenance::ContactProvenance;
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
    ScopedContacts {
        outdoor: scoped_head(school, athlete, Sport::OutdoorTrack),
        indoor: scoped_head(school, athlete, Sport::IndoorTrack),
        cross_country: scoped_head(school, athlete, Sport::CrossCountry),
        director: school
            .map(|school| school.heads.resolve(Slot::Director, Gender::Mixed))
            .map_or(&Outcome::UNKNOWN, core::convert::identity),
        assistants: school
            .map(|school| school.assistants.as_slice())
            .map_or(&[][..], |value| value),
        athlete,
    }
}

fn scoped_head<'a>(
    school: Option<&'a SchoolContacts>,
    athlete: &CanonicalAthlete,
    sport: Sport,
) -> &'a Outcome {
    if !athlete.sports.contains(&sport) {
        return &Outcome::UNKNOWN;
    }
    school
        .zip(Slot::of(sport))
        .map(|(school, slot)| school.heads.resolve(slot, athlete.gender))
        .map_or(&Outcome::UNKNOWN, core::convert::identity)
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

    pub(in crate::workbook) fn preferred(&self) -> Preferred<'_> {
        let (named, blocker) = self.head_choice();
        if let Some((slot, coach)) = named.filter(|(_, coach)| coach.address().is_some()) {
            return Preferred::named(slot, coach);
        }
        self.fallback_preferred(named, blocker)
    }

    fn head_choice(&self) -> (Option<(Slot, &Named)>, Option<ContactState>) {
        let mut named = None;
        let mut blocker = None;
        for (slot, outcome) in self.heads() {
            if let Some(coach) = outcome.named() {
                if coach.address().is_some() {
                    return (Some((slot, coach)), blocker);
                }
                if named.is_none() {
                    named = Some((slot, coach));
                }
            }
            blocker = blocker.or(outcome.blocker());
        }
        (named, blocker)
    }

    fn fallback_preferred<'a>(
        &'a self,
        named: Option<(Slot, &'a Named)>,
        blocker: Option<ContactState>,
    ) -> Preferred<'a> {
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

    fn eligible(&self) -> impl Iterator<Item = &Named> {
        let assistants = self.assistants.iter().filter(|coach| {
            coach
                .sport
                .is_some_and(|sport| self.athlete.sports.contains(&sport))
                && matches_side(coach.side, self.athlete.gender)
        });
        self.heads()
            .filter_map(|(_, outcome)| outcome.named())
            .chain(self.director())
            .chain(assistants)
    }

    pub(in crate::workbook) fn mailboxes(
        &self,
    ) -> impl Iterator<Item = (&Named, &str, &ContactProvenance)> {
        self.eligible().flat_map(|coach| {
            [
                coach.email.as_deref().zip(coach.professional_source()),
                coach.personal_email.as_deref().zip(coach.personal_source()),
            ]
            .into_iter()
            .flatten()
            .map(move |(address, source)| (coach, address, source))
        })
    }

    pub(in crate::workbook) fn all_emails(&self) -> String {
        let addresses: BTreeSet<_> = self.mailboxes().map(|(_, address, _)| address).collect();
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
