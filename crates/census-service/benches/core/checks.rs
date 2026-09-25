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
use census_domain::model::{published_email, CanonicalCoach, CanonicalSchool, MailboxKind};

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
/// The claims one folded coach row has to hold: the phone union survived, at least one mailbox
/// survived, and every present mailbox is published under the field for its domain kind.
fn coach_row(row: &CanonicalCoach) -> Result<(bool, bool)> {
    ensure!(
        row.phone.is_some(),
        "{} lost the union of a phone number",
        row.id
    );
    let mut professional = false;
    let mut personal = false;
    for (address, kind) in [
        (row.professional_email.as_deref(), MailboxKind::Professional),
        (row.personal_email.as_deref(), MailboxKind::Personal),
    ] {
        let Some(address) = address else { continue };
        let actual = published_email(address)
            .map(|(_, kind)| kind)
            .ok_or_else(|| anyhow::anyhow!("{} lost a valid mailbox: {address}", row.id))?;
        ensure!(
            actual == kind,
            "{} filed {address} under the wrong mailbox field",
            row.id
        );
        match kind {
            MailboxKind::Professional => professional = true,
            MailboxKind::Personal => personal = true,
        }
    }
    ensure!(
        professional || personal,
        "{} lost its seeded mailbox",
        row.id
    );
    Ok((professional, personal))
}

/// Count the professional and personal mailbox fields the folded rows carry.
pub(super) fn coach_tally(coaches: &[CanonicalCoach]) -> Result<(usize, usize)> {
    coaches
        .iter()
        .try_fold((0_usize, 0_usize), |(professional, personal), row| {
            let (has_professional, has_personal) = coach_row(row)?;
            Ok((
                professional.saturating_add(usize::from(has_professional)),
                personal.saturating_add(usize::from(has_personal)),
            ))
        })
}
