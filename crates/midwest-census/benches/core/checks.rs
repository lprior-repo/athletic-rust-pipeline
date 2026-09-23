//! What one folded row of the seeded batch has to look like, split out of `merge`'s fold contract.
//!
//! [`school_row`] holds the school-side claims: the first writer's enrollment and city survived, the
//! co-op flag and the aliases united, the longer name is kept while the minted name does not move,
//! and provenance never emptied. [`coach_row`] holds the contact policy: the phone number united,
//! and a mailbox is published only where `publish` left it publishable — a dropped mailbox has to be
//! a recorded one. [`coach_tally`] counts the two outcomes, so the parent's fold can refuse a batch
//! that published or withheld the wrong half.
//!
//! `merge` runs these once, through [`Dataset::build`](super::Dataset::build), before any
//! measurement: a store that stopped folding an observation into its entity fails the run here
//! instead of being timed.

use anyhow::{ensure, Result};
use census_domain::model::{professional_email, CanonicalCoach, CanonicalSchool};

use super::{ALIASES_PER_SCHOOL, FIRST_CITY, FIRST_ENROLLMENT, LONG_SUFFIX};

/// The claims one folded school row has to hold: the first writer's enrollment and city, the union
/// of the co-op flag and the aliases, the longer name over an untouched minted name, and provenance.
pub(super) fn school_row(row: &CanonicalSchool) -> Result<()> {
    ensure!(
        row.enrollment == Some(FIRST_ENROLLMENT),
        "{} kept a later enrollment",
        row.id
    );
    ensure!(
        row.city.as_deref() == Some(FIRST_CITY),
        "{} kept a later city",
        row.id
    );
    ensure!(row.co_op, "{} lost the union of a co-op flag", row.id);
    let aliases = row.aliases.len();
    ensure!(
        aliases == ALIASES_PER_SCHOOL,
        "{} carries {aliases} aliases",
        row.id
    );
    ensure!(
        row.name.ends_with(LONG_SUFFIX),
        "{} lost the longer name",
        row.id
    );
    ensure!(
        !row.normalized_name.contains("campus"),
        "{} rewrote the minted name",
        row.id
    );
    let provenance = !row.source_identities.is_empty() && !row.evidence.is_empty();
    ensure!(provenance, "{} lost its provenance unions", row.id);
    Ok(())
}

/// The claims one folded coach row has to hold: the union of a phone number, and a mailbox that was
/// published only where `publish` left it publishable. Returns whether the row published one.
fn coach_row(row: &CanonicalCoach) -> Result<bool> {
    ensure!(
        row.phone.is_some(),
        "{} lost the union of a phone number",
        row.id
    );
    match row.professional_email.as_deref() {
        Some(email) => {
            let publishable = professional_email(email).is_some() && !row.email_withheld;
            ensure!(
                publishable,
                "{} published a withheld mailbox: {email}",
                row.id
            );
            Ok(true)
        }
        None => {
            ensure!(
                row.email_withheld,
                "{} dropped a mailbox without recording it",
                row.id
            );
            Ok(false)
        }
    }
}

/// How many folded coach rows publish a mailbox and how many withhold one, row by row.
pub(super) fn coach_tally(coaches: &[CanonicalCoach]) -> Result<(usize, usize)> {
    let mut published = 0_usize;
    let mut withheld = 0_usize;
    for row in coaches {
        if coach_row(row)? {
            published = published.saturating_add(1);
        } else {
            withheld = withheld.saturating_add(1);
        }
    }
    Ok((published, withheld))
}
