//! The head coaches one school's coach rows leave behind, indexed by contact slot and side of a team.
//!
//! One program arrives as several rows: a boys' and a girls' head coach of the same sport, a
//! re-observation that published a new address, a row the source never bound to a side of a team.
//! This module resolves each `(slot, side)` bucket to the one row the contact ladder reads, records
//! every bucket whose rows disagree, and marks the buckets the precedence order cannot separate —
//! the rows that share the newest observation and agree on whether they published an address, and
//! still disagree about who the coach is.

use census_domain::model::{CanonicalCoach, Gender};
use std::cmp::Reverse;

use super::{role_label, Disagreement, Named, Slot};

/// One side of one slot, as the coach rows resolved it: the row precedence picked, and whether the
/// rows of that side disagree in a way the precedence order cannot separate.
#[derive(Debug, Default, Clone)]
pub(super) struct Side {
    /// The row the precedence order resolved this side to, when the side has a row at all.
    pub(super) row: Option<Named>,
    /// The bucket holds rows the precedence order cannot separate, so [`Side::row`] is only the
    /// name-and-id tie-break's choice rather than an evidenced one.
    pub(super) undecided: bool,
}

/// The four sides a coach row can be published for.
#[derive(Debug, Default, Clone)]
struct Sides {
    boys: Side,
    girls: Side,
    mixed: Side,
    unknown: Side,
}

impl Sides {
    /// Record the row one side resolved to.
    fn set(&mut self, side: Gender, named: Named) {
        self.side_mut(side).row = Some(named);
    }

    /// Record that one side's rows cannot be separated by the precedence order.
    fn undecide(&mut self, side: Gender) {
        self.side_mut(side).undecided = true;
    }

    /// The row the athlete reaches first: their own side, then a side-less row, then an unplaced
    /// one, then boys, then girls.
    fn resolve(&self, side: Gender) -> Option<&Side> {
        [
            side,
            Gender::Mixed,
            Gender::Unknown,
            Gender::Boys,
            Gender::Girls,
        ]
        .into_iter()
        .map(|candidate| self.side(candidate))
        .find(|bucket| bucket.row.is_some())
    }

    fn side(&self, side: Gender) -> &Side {
        match side {
            Gender::Boys => &self.boys,
            Gender::Girls => &self.girls,
            Gender::Mixed => &self.mixed,
            Gender::Unknown => &self.unknown,
        }
    }

    fn side_mut(&mut self, side: Gender) -> &mut Side {
        match side {
            Gender::Boys => &mut self.boys,
            Gender::Girls => &mut self.girls,
            Gender::Mixed => &mut self.mixed,
            Gender::Unknown => &mut self.unknown,
        }
    }
}

/// A school's head coaches, bucketed per slot and side.
#[derive(Debug, Default, Clone)]
pub(in crate::workbook::recruiting) struct Heads {
    track: Sides,
    cross_country: Sides,
    school_wide: Sides,
    conflicts: usize,
    disagreements: Vec<Disagreement>,
}

impl Heads {
    /// Record the coach one `(slot, side)` bucket resolved to.
    pub(super) fn set(&mut self, slot: Slot, side: Gender, named: Named) {
        if let Some(sides) = self.sides_mut(slot) {
            sides.set(side, named);
        }
    }

    /// Record that one `(slot, side)` bucket holds rows that disagree about the contact: the bucket
    /// is counted, marked undecided when the precedence order cannot separate its rows, and
    /// described for the retained conflict queue.
    pub(super) fn disagree(
        &mut self,
        school: &str,
        slot: Slot,
        side: Gender,
        rows: &[&CanonicalCoach],
        undecided: bool,
    ) {
        if let Some(sides) = self.sides_mut(slot) {
            if undecided {
                sides.undecide(side);
            }
        }
        self.conflicts = self.conflicts.saturating_add(1);
        self.disagreements.push(Disagreement {
            school: school.to_string(),
            role: role_label(slot, side),
            decided: !undecided,
            rows: describe(rows.iter().copied()),
        });
    }

    /// Buckets whose rows disagree about a head coach.
    pub(in crate::workbook::recruiting) fn conflicts(&self) -> usize {
        self.conflicts
    }

    /// The side of one slot the athlete reaches, with the row precedence resolved it to.
    pub(super) fn resolve(&self, slot: Slot, side: Gender) -> Option<&Side> {
        self.sides(slot)?.resolve(side)
    }

    /// Every bucket whose rows disagree, in slot and side order.
    pub(super) fn into_disagreements(self) -> Vec<Disagreement> {
        self.disagreements
    }

    fn sides(&self, slot: Slot) -> Option<&Sides> {
        match slot {
            Slot::Track => Some(&self.track),
            Slot::CrossCountry => Some(&self.cross_country),
            Slot::SchoolWide => Some(&self.school_wide),
            Slot::Director => None,
        }
    }

    fn sides_mut(&mut self, slot: Slot) -> Option<&mut Sides> {
        match slot {
            Slot::Track => Some(&mut self.track),
            Slot::CrossCountry => Some(&mut self.cross_country),
            Slot::SchoolWide => Some(&mut self.school_wide),
            Slot::Director => None,
        }
    }
}

/// Whether a bucket holds rows the precedence order cannot separate: the rows that share the newest
/// observation and agree on whether they published an address, yet disagree about the coach's name
/// or the address they published. Two rows that agree on both are one observation, not a conflict.
pub(super) fn undecided<'a>(rows: impl Iterator<Item = &'a CanonicalCoach>) -> bool {
    let rows: Vec<&CanonicalCoach> = rows.collect();
    let Some(best) = rows.iter().map(|row| evidence_key(row)).max() else {
        return false;
    };
    let mut identities: Vec<(&str, Option<&str>)> = Vec::new();
    for row in rows {
        if evidence_key(row) != best {
            continue;
        }
        let identity = (row.name.as_str(), coach_email(row));
        if !identities.contains(&identity) {
            identities.push(identity);
        }
    }
    identities.len() > 1
}

/// The preferred published address, with the consumer mailbox as an explicit fallback.
fn coach_email(coach: &CanonicalCoach) -> Option<&str> {
    coach
        .professional_email
        .as_deref()
        .or(coach.personal_email.as_deref())
}

/// The evidence precedence compares whether the row published an address, then its newest date.
fn evidence_key(coach: &CanonicalCoach) -> (bool, &str) {
    (coach_email(coach).is_some(), observed_on(coach))
}

/// The newest `Evidence.observed_on` one row carries, blank when it carries no evidence.
pub(super) fn observed_on(coach: &CanonicalCoach) -> &str {
    coach
        .evidence
        .iter()
        .map(|evidence| evidence.observed_on.as_str())
        .max()
        .unwrap_or_default()
}

/// The row one `(slot, side)` bucket resolves to: a row that published a professional address, or
/// its personal fallback, wins over one that published none; newer evidence then decides.
pub(super) fn resolve<'a>(
    rows: impl Iterator<Item = &'a CanonicalCoach>,
) -> Option<&'a CanonicalCoach> {
    rows.min_by(|left, right| rank(left).cmp(&rank(right)))
}

/// Whether a `(slot, side)` bucket's rows publish two or more different addresses.
pub(super) fn conflicting<'a>(rows: impl Iterator<Item = &'a CanonicalCoach>) -> bool {
    let mut addresses: Vec<&str> = Vec::new();
    for row in rows {
        let Some(address) = coach_email(row) else {
            continue;
        };
        if addresses.contains(&address) {
            continue;
        }
        addresses.push(address);
        if addresses.len() > 1 {
            return true;
        }
    }
    false
}

/// The total order a bucket resolves by: an address before no address, then newest observation,
/// then coach name and id.
fn rank(coach: &CanonicalCoach) -> (Reverse<bool>, Reverse<&str>, &str, &str) {
    (
        Reverse(coach_email(coach).is_some()),
        Reverse(observed_on(coach)),
        coach.name.as_str(),
        coach.id.as_str(),
    )
}

/// Every row of one bucket, including a personal address when that is all it published.
fn describe<'a>(rows: impl Iterator<Item = &'a CanonicalCoach>) -> Vec<String> {
    rows.map(|row| {
        let address = coach_email(row).unwrap_or("no published address");
        format!("{}: {} (observed {})", row.name, address, observed_on(row))
    })
    .collect()
}
