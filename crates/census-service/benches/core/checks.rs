
use anyhow::{ensure, Result};
use census_domain::model::{published_email, CanonicalCoach, CanonicalSchool, MailboxKind};

use super::{ALIASES_PER_SCHOOL, FIRST_CITY, FIRST_ENROLLMENT, LONG_SUFFIX};

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
