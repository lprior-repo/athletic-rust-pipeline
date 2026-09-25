//! What one folded row of the seeded batch has to look like, split out of `merge`'s fold contract.
//!
//! [`school_row`] holds the school-side claims: the first writer's enrollment and city survived, the
//! co-op flag and the aliases united, the longer name is kept while the minted name does not move,
//! and provenance never emptied. [`coach_row`] holds the contact policy: every valid mailbox is
//! published in the field for its domain kind. [`coach_tally`] counts the professional and personal
//! fields, so the parent's fold can refuse a batch that lost an address or put it in the wrong field.
//!
//! `merge` runs these once, through [`Dataset::build`](super::Dataset::build), before any
//! measurement: a store that stopped folding an observation into its entity fails the run here
//! instead of being timed.

use anyhow::{ensure, Result};
use census_domain::model::{
    published_email, CanonicalCoach, CanonicalSchool, MailboxKind,
};

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
/// The claims one folded coach row has to hold: the phone union survived, exactly one mailbox
/// survived, and its domain kind agrees with the field that carries it.
fn coach_row(row: &CanonicalCoach) -> Result<MailboxKind> {
    ensure!(
        row.phone.is_some(),
        "{} lost the union of a phone number",
        row.id
    );
    ensure!(
        row.professional_email.is_some() ^ row.personal_email.is_some(),
        "{} lost or duplicated its seeded mailbox",
        row.id
    );
    let (email, expected_kind) = match (
        row.professional_email.as_deref(),
        row.personal_email.as_deref(),
    ) {
        (Some(email), None) => (email, MailboxKind::Professional),
        (None, Some(email)) => (email, MailboxKind::Personal),
        _ => {
            return Err(anyhow::anyhow!(
                "{} lost or duplicated its seeded mailbox",
                row.id
            ));
        }
    };
    let actual_kind = match published_email(email) {
        Some((_, kind)) => kind,
        None => {
            return Err(anyhow::anyhow!(
                "{} lost a valid mailbox: {email}",
                row.id
            ));
        }
    };
    ensure!(
        actual_kind == expected_kind,
        "{} placed {email} in the wrong mailbox field",
        row.id
    );
    Ok(actual_kind)
}

/// Count folded coach rows by the domain kind of their published mailbox.
pub(super) fn coach_tally(coaches: &[CanonicalCoach]) -> Result<(usize, usize)> {
    coaches
        .iter()
        .try_fold((0_usize, 0_usize), |(professional, personal), row| {
            match coach_row(row)? {
                MailboxKind::Professional => Ok((professional.saturating_add(1), personal)),
                MailboxKind::Personal => Ok((professional, personal.saturating_add(1))),
            }
        })
}
