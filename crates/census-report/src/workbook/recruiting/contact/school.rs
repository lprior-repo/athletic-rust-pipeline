//! The contact facts one school's coach rows leave behind: the sheet-level head-coach columns, the
//! athletic director, and the per-slot head-coach index the contact ladder resolves against.
//!
//! Every coach row is folded in once, whatever its role: an assistant coach names no contact, but the
//! row is still what tells the ladder that a contact source reached the school at all — which is the
//! difference between `no_public_contact_found` and `contact_source_not_attempted`.

use census_domain::model::{CanonicalCoach, CoachRole, Gender, Sport};
use std::collections::{BTreeMap, BTreeSet};

use super::heads::{conflicting, resolve, undecided, Heads};
use super::{Named, Slot};

/// School-level contact facts the athlete and coach sheets both print.
#[derive(Debug, Default, Clone)]
pub(in crate::workbook::recruiting) struct SchoolContacts {
    /// Head coach of the track programs: an outdoor head coach when the school has one, else an
    /// indoor one, then by coach name.
    pub(in crate::workbook::recruiting) head_track: Option<String>,
    pub(in crate::workbook::recruiting) head_track_email: Option<String>,
    pub(in crate::workbook::recruiting) head_cross_country: Option<String>,
    pub(in crate::workbook::recruiting) head_cross_country_email: Option<String>,
    /// The school's athletic director, first by name.
    pub(in crate::workbook::recruiting) director: Option<Named>,
    /// Every professional and personal address held on every coach row for this school.
    pub(in crate::workbook::recruiting) all_emails: String,
    /// The school's head coaches, per slot and side.
    pub(in crate::workbook::recruiting) heads: Heads,
}

/// School id -> contact facts. Every school with a coach row of any role gets an entry, so a school
/// the contact source reached and left without a contact is distinguishable from one it never
/// reached at all.
pub(in crate::workbook::recruiting) fn contacts(
    coaches: &[CanonicalCoach],
) -> BTreeMap<String, SchoolContacts> {
    let mut buckets: BTreeMap<&str, Buckets<'_>> = BTreeMap::new();
    for coach in coaches {
        buckets
            .entry(coach.school.as_str())
            .or_default()
            .push(coach);
    }
    buckets
        .into_iter()
        .map(|(school, buckets)| (school.to_string(), buckets.contacts(school)))
        .collect()
}

/// The coach rows one school contributes, while [`contacts`] walks the coach table once.
#[derive(Default)]
struct Buckets<'a> {
    heads: BTreeMap<(Slot, Gender), Vec<&'a CanonicalCoach>>,
    directors: Vec<&'a CanonicalCoach>,
    all_emails: BTreeSet<String>,
}

impl<'a> Buckets<'a> {
    /// Fold one coach row in, whatever its role: an assistant names no contact, but the row is still
    /// what tells the sheet's ladder that a contact source reached the school.
    fn push(&mut self, coach: &'a CanonicalCoach) {
        if let Some(email) = coach.professional_email.as_ref() {
            self.all_emails.insert(email.clone());
        }
        if let Some(email) = coach.personal_email.as_ref() {
            self.all_emails.insert(email.clone());
        }
        match coach.role {
            CoachRole::HeadCoach => {
                self.heads
                    .entry((head_slot(coach), coach.gender))
                    .or_default()
                    .push(coach);
            }
            CoachRole::AthleticDirector => self.directors.push(coach),
            CoachRole::AssistantCoach | CoachRole::Unknown => {}
        }
    }

    /// The school's contact facts: the sheet-level head coach columns, the director, and the bucket
    /// graph the preferred contact resolves against, with every disagreement the school's rows leave
    /// behind recorded for the retained conflict queue.
    fn contacts(&self, school: &str) -> SchoolContacts {
        let head_track = self.legacy_track();
        let head_cross_country = self.legacy_cross_country();
        let head_track_email = head_track.and_then(published_contact_email);
        let head_cross_country_email = head_cross_country.and_then(published_contact_email);
        SchoolContacts {
            head_track: head_track.map(|coach| coach.name.clone()),
            head_track_email,
            head_cross_country: head_cross_country.map(|coach| coach.name.clone()),
            head_cross_country_email,
            director: self.director(),
            all_emails: self
                .all_emails
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join("; "),
            heads: self.heads(school),
        }
    }

    /// Every head-coach row the school holds for one slot, across sides.
    fn rows_of(&self, slot: Slot) -> impl Iterator<Item = &'a CanonicalCoach> + '_ {
        self.heads
            .iter()
            .filter(move |((candidate, _), _)| *candidate == slot)
            .flat_map(|(_, rows)| rows.iter().copied())
    }

    /// The school's track head coach, as the sheet-level columns publish them: outdoor before
    /// indoor, then by name.
    fn legacy_track(&self) -> Option<&'a CanonicalCoach> {
        self.rows_of(Slot::Track).min_by(|left, right| {
            (track_rank(left.sport), left.name.as_str())
                .cmp(&(track_rank(right.sport), right.name.as_str()))
        })
    }

    /// The school's cross-country head coach, first by name.
    fn legacy_cross_country(&self) -> Option<&'a CanonicalCoach> {
        self.rows_of(Slot::CrossCountry)
            .min_by(|left, right| left.name.cmp(&right.name))
    }

    /// The school's athletic director, first by name.
    fn director(&self) -> Option<Named> {
        self.directors
            .iter()
            .min_by(|left, right| left.name.cmp(&right.name))
            .map(|coach| Named::of(coach))
    }

    /// One resolved coach per `(slot, side)` bucket, and a record of every bucket whose rows
    /// disagree about the contact.
    fn heads(&self, school: &str) -> Heads {
        let mut heads = Heads::default();
        for ((slot, side), rows) in &self.heads {
            if let Some(coach) = resolve(rows.iter().copied()) {
                heads.set(*slot, *side, Named::of(coach));
            }
            let unresolved = undecided(rows.iter().copied());
            if unresolved || conflicting(rows.iter().copied()) {
                heads.disagree(school, *slot, *side, rows, unresolved);
            }
        }
        heads
    }
}

/// Prefer the organisation address, falling back to the retained personal address.
fn published_contact_email(coach: &CanonicalCoach) -> Option<String> {
    coach
        .professional_email
        .clone()
        .or_else(|| coach.personal_email.clone())
}
/// The slot one head-coach row fills: the sport it is bound to, else the school-wide slot.
fn head_slot(coach: &CanonicalCoach) -> Slot {
    if is_track_sport(coach.sport) {
        Slot::Track
    } else if coach.sport == Some(Sport::CrossCountry) {
        Slot::CrossCountry
    } else {
        Slot::SchoolWide
    }
}

/// Head-coach sort rank: outdoor track first, then indoor track, then cross country.
fn track_rank(sport: Option<Sport>) -> u8 {
    match sport {
        Some(Sport::OutdoorTrack) => 0,
        Some(Sport::IndoorTrack) => 1,
        _ => 2,
    }
}

fn is_track_sport(sport: Option<Sport>) -> bool {
    matches!(sport, Some(Sport::OutdoorTrack) | Some(Sport::IndoorTrack))
}
