//! The one recruiting contact an athlete's row prefers, and the state that says what was looked at.
//!
//! §50's last four columns answer "who do I contact for this athlete, and how do we know": the
//! preferred contact's name, the role the sheet names it under, the address, and the state that says
//! what was looked at. This module owns the rule; `school` folds the coach table into the facts it
//! resolves against, `heads` resolves each slot's rows, `athletes` renders the result and documents
//! the columns, and `coaches` prints the same school facts row by row.
//!
//! # The preferred contact
//!
//! The athlete's own evidence picks the slot, and the slot ladder picks the contact:
//!
//! 1. the head coach of the athlete's evidence-bearing sport — the track slot (`Head TF Coach`) for
//!    an athlete with stored indoor or outdoor track evidence, the cross-country slot
//!    (`Head XC Coach`) for an athlete whose only sport is cross country, and the track slot for an
//!    athlete that stores no sport at all, which is the sheet's first coaching contact;
//! 2. else the school's other head-coach slot;
//! 3. else a head coach whose row carries no sport binding (the source publishes school-wide head
//!    coach rows as well as per-sport ones);
//! 4. else the school's athletic director.
//!
//! The rung that matched sets [`ContactState`]: a coach's published address is
//! `professional_coach_email`, the director's is `professional_ad_email`, and a named contact with
//! no published address anywhere is `coach_name_only`. When the coach table holds rows for the
//! school but none of them names a head coach or an athletic director, the state is
//! `no_public_contact_found`; when it holds no row for the school at all, no contact source has ever
//! landed one and the state is `contact_source_not_attempted`. Every state names what was looked at,
//! so a blank cell is never the answer to "did we look".
//!
//! # Which head coach, when a school's rows name several
//!
//! The source publishes one head coach per side of a team — `Boys Track and Field` and
//! `Girls Track and Field` both arrive as [`Sport::OutdoorTrack`] — so a slot is bucketed by the side
//! its rows were published for, and a bucket resolves to one row: the athlete's own side first, then
//! a side-less (`Gender::Mixed`) row, then an `Unknown` one, then boys, then girls. Within one
//! bucket the row that published an address wins over a row that published none, and the
//! newest `Evidence.observed_on` wins among rows otherwise equal; ties fall back to the coach's name,
//! then to the coach's id, so two runs over one store publish the same coach.
//!
//! Two rows of one bucket publishing different addresses is the disagreement that order resolves: the
//! rung still answers with the winner's kind, and the bucket is recorded as a `contact_conflict` row
//! on the workbook's `Conflicts` sheet, because the retained-conflict queue prints rows rather than
//! counts. When the evidence cannot separate the rows either — the rows that share the newest
//! observation disagree about the coach's name or the address they published — nothing evidenced
//! picks a coach, the bucket is recorded just the same, and the state is `contact_conflict` instead
//! of a coin toss between names. The rows themselves stay visible on the `Coaches` sheet, which
//! publishes every coach the store holds.
//!
//! A preferred-contact email carries the professional address first, falling back explicitly to the
//! personal address when no professional address was published. A blank means no source published one.

mod heads;
mod normalise;
mod school;

use census_domain::model::{CanonicalAthlete, CanonicalCoach, Sport};
use normalise::{role_label, Named};

pub(super) use normalise::Preferred;
pub(super) use school::{contacts, SchoolContacts};

/// The `Contact Coverage State` column's vocabulary: what the row looked at, and what it found.
///
/// The column is `Contact Coverage State` and not `Contact State`: the `Coverage State` column one
/// column away already means the sheet pass that wrote the row, and two columns called "State" would
/// read as the same quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ContactState {
    /// A head coach of the athlete's own sport published an address.
    ProfessionalCoachEmail,
    /// No coach of the athlete's sport published one; the athletic director did.
    ProfessionalAdEmail,
    /// A head coach of the athlete's own sport published only a consumer mailbox.
    PersonalCoachEmail,
    /// No coach of the athlete's sport published one; the athletic director published only a consumer mailbox.
    PersonalAdEmail,
    /// A contact is named, and no public address exists anywhere for the school.
    CoachNameOnly,
    /// The coach table holds rows for the school, and none names a head coach or an athletic
    /// director: a contact source reached the school and published no contact.
    NoPublicContactFound,
    /// The coach table holds no row for the school at all: no contact source has landed one there.
    ContactSourceNotAttempted,
    /// The school's rows disagree about the athlete's own slot and the precedence order cannot
    /// separate them, so which coach is current is unresolved rather than merely unpublished.
    ContactConflict,
}

impl ContactState {
    /// The cell text, the vocabulary the `Contact Coverage State` column publishes.
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::ProfessionalCoachEmail => "professional_coach_email",
            Self::ProfessionalAdEmail => "professional_ad_email",
            Self::CoachNameOnly => "coach_name_only",
            Self::NoPublicContactFound => "no_public_contact_found",
            Self::ContactSourceNotAttempted => "contact_source_not_attempted",
            Self::ContactConflict => "contact_conflict",
            Self::PersonalCoachEmail => "personal_coach_email",
            Self::PersonalAdEmail => "personal_ad_email",
        }
    }
}

/// The contact slots one school's coach table can fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Slot {
    /// The track programs' head coach: the sheet's `Head TF Coach` column.
    Track,
    /// The cross-country program's head coach: the sheet's `Head XC Coach` column.
    CrossCountry,
    /// A head coach whose row carries no sport binding.
    SchoolWide,
    /// The school's athletic director.
    Director,
}

impl Slot {
    /// The slot the athlete's stored sports prefer: track for a track athlete, cross country for an
    /// athlete whose only sport is cross country, and track for an athlete that stores no sport.
    fn preferred(sports: &[Sport]) -> Self {
        let track = sports
            .iter()
            .any(|sport| matches!(sport, Sport::IndoorTrack | Sport::OutdoorTrack));
        if track || !sports.contains(&Sport::CrossCountry) {
            Self::Track
        } else {
            Self::CrossCountry
        }
    }

    /// The school's other coaching slot: a track athlete falls back to the cross-country coach and
    /// the other way round.
    fn other(self) -> Self {
        match self {
            Self::CrossCountry => Self::Track,
            Self::Track | Self::SchoolWide | Self::Director => Self::CrossCountry,
        }
    }

    /// The label the sheet's own contact columns carry, so a reader can find the column a contact
    /// came from.
    fn label(self) -> &'static str {
        match self {
            Self::Track => "Head TF Coach",
            Self::CrossCountry => "Head XC Coach",
            Self::SchoolWide => "Head Coach",
            Self::Director => "Athletic Director",
        }
    }
}

/// One school's contact disagreement, as the retained conflict queue reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::workbook) struct Disagreement {
    /// The school the disagreeing rows belong to: the queue row's subject id.
    pub(in crate::workbook) school: String,
    /// The slot and side the rows disagree about, as the `Preferred Contact Role` cell names them.
    pub(in crate::workbook) role: String,
    /// Whether the precedence order separated the rows, or left the bucket unresolved.
    pub(in crate::workbook) decided: bool,
    /// Every row of the bucket: the coach, the address they published, and the observation that
    /// dates the row.
    pub(in crate::workbook) rows: Vec<String>,
}

/// Every school whose coach rows disagree about a head coach, as the retained conflict queue reads
/// them: the same buckets the athletes' ladder resolves against, so no disagreement — decided or
/// undecided — stays invisible.
pub(in crate::workbook) fn disagreements(coaches: &[CanonicalCoach]) -> Vec<Disagreement> {
    contacts(coaches)
        .into_values()
        .flat_map(|facts| facts.heads.into_disagreements())
        .collect()
}

/// The contact one athlete's row prefers, in the order this module's header states.
pub(super) fn preferred(
    contacts: Option<&SchoolContacts>,
    athlete: &CanonicalAthlete,
) -> Preferred {
    let Some(contacts) = contacts else {
        return Preferred::unnamed(ContactState::ContactSourceNotAttempted);
    };
    let slot = Slot::preferred(&athlete.sports);
    let heads = [
        (slot, contacts.heads.resolve(slot, athlete.gender)),
        (
            slot.other(),
            contacts.heads.resolve(slot.other(), athlete.gender),
        ),
        (
            Slot::SchoolWide,
            contacts.heads.resolve(Slot::SchoolWide, athlete.gender),
        ),
    ];
    let mut named: Option<(Slot, &Named)> = None;
    for (slot, bucket) in heads.iter() {
        let Some(bucket) = bucket else {
            continue;
        };
        if bucket.undecided {
            return Preferred::unnamed(ContactState::ContactConflict);
        }
        let Some(coach) = bucket.row.as_ref() else {
            continue;
        };
        if coach.email.is_some() {
            return Preferred::named(*slot, coach, ContactState::ProfessionalCoachEmail);
        }
        if coach.personal_email.is_some() {
            return Preferred::named(*slot, coach, ContactState::PersonalCoachEmail);
        }
        if named.is_none() {
            named = Some((*slot, coach));
        }
    }
    let director = contacts.director.as_ref();
    if let Some(director) = director.filter(|director| director.email.is_some()) {
        return Preferred::named(Slot::Director, director, ContactState::ProfessionalAdEmail);
    }
    if let Some(director) = director.filter(|director| director.personal_email.is_some()) {
        return Preferred::named(Slot::Director, director, ContactState::PersonalAdEmail);
    }
    if let Some((slot, coach)) = named {
        if coach.personal_email.is_some() {
            return Preferred::named(slot, coach, ContactState::PersonalCoachEmail);
        }
        return Preferred::named(slot, coach, ContactState::CoachNameOnly);
    }
    if let Some(director) = director {
        return Preferred::named(Slot::Director, director, ContactState::CoachNameOnly);
    }
    Preferred::unnamed(ContactState::NoPublicContactFound)
}
