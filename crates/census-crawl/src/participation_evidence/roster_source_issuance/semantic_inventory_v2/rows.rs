use super::*;
use crate::milesplit::{LocatedRosterAthlete, RosterRejection, SourceRowLocator};
use std::collections::BTreeSet;

pub(super) fn inventory(
    verdict: &RosterVerdict,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Vec<wire::Row>> {
    let (roster, rejected) = extract_verdict(verdict)?;
    let count = count_rows(roster, rejected)?;
    charge_rows(count, grant)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(count)
        .map_err(|_| invalid("row-local row allocation refused"))?;
    let mut identities = BTreeSet::new();
    inventory_admitted(roster, &mut identities, &mut rows, grant)?;
    inventory_rejected(rejected, &mut identities, &mut rows)?;
    finalize_rows(rows)
}
fn finalize_rows(mut rows: Vec<wire::Row>) -> StoreResult<Vec<wire::Row>> {
    rows.sort_by_key(|value| value.row.ordinal);
    validate_spans(&rows)?;
    Ok(rows)
}
fn extract_verdict(
    verdict: &RosterVerdict,
) -> Result<(&Roster, &[RosterRejection]), census_store::StoreError> {
    match verdict {
        RosterVerdict::Complete { roster } => Ok((roster, &[])),
        RosterVerdict::Partial {
            roster,
            rejected,
            unfinished: None,
        } if !rejected.is_empty() => Ok((roster, rejected)),
        _ => Err(invalid("row-local roster unfinished or quarantined")),
    }
}

fn count_rows(
    roster: &Roster,
    rejected: &[RosterRejection],
) -> Result<usize, census_store::StoreError> {
    let count = roster
        .athletes
        .len()
        .checked_add(rejected.len())
        .ok_or(census_store::StoreError::CounterOverflow)?;
    bound(count, MAX_ROWS, "row-local disposition rows")?;
    if roster.athletes.is_empty() {
        return Err(invalid("row-local admitted population absent"));
    }
    Ok(count)
}

fn charge_rows(
    count: usize,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<(), census_store::StoreError> {
    grant
        .charge_steps(
            count
                .checked_mul(16)
                .ok_or(census_store::StoreError::CounterOverflow)?,
        )
        .map_err(domain)?;
    grant
        .charge_bytes(
            count
                .checked_mul(256)
                .ok_or(census_store::StoreError::CounterOverflow)?,
        )
        .map_err(domain)?;
    Ok(())
}

fn inventory_admitted<'a>(
    roster: &'a Roster,
    identities: &mut BTreeSet<&'a str>,
    rows: &mut Vec<wire::Row>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    for member in &roster.athletes {
        let id = &member.athlete().athlete_id;
        unique(identities, id)?;
        rows.push(wire::Row {
            row: member.locator(),
            provider_id: id.clone(),
            outcome: wire::Outcome::Admitted {
                semantic_digest: member_digest(member, grant)?,
            },
        });
    }
    Ok(())
}

fn inventory_rejected<'a>(
    rejected: &'a [RosterRejection],
    identities: &mut BTreeSet<&'a str>,
    rows: &mut Vec<wire::Row>,
) -> StoreResult<()> {
    for rejected in rejected {
        let id = rejected
            .athlete_id
            .as_deref()
            .ok_or_else(|| invalid("row-local rejected identity unknown"))?;
        unique(identities, id)?;
        rows.push(wire::Row {
            row: rejected.row,
            provider_id: id.into(),
            outcome: wire::Outcome::Rejected {
                reason: rejected.kind,
            },
        });
    }
    Ok(())
}

fn unique<'a>(identities: &mut BTreeSet<&'a str>, id: &'a str) -> StoreResult<()> {
    if id.is_empty()
        || id.len() > 128
        || !id.bytes().all(|byte| byte.is_ascii_digit())
        || !identities.insert(id)
    {
        return Err(invalid(
            "row-local provider identity unknown or overlapping",
        ));
    }
    Ok(())
}

fn validate_spans(rows: &[wire::Row]) -> StoreResult<()> {
    let mut end = 0;
    for (ordinal, value) in rows.iter().enumerate() {
        let SourceRowLocator {
            ordinal: actual,
            byte_offset,
            byte_length,
        } = value.row;
        if usize::try_from(actual).map_err(|_| census_store::StoreError::CounterOverflow)?
            != ordinal
            || byte_length == 0
            || byte_offset < end
        {
            return Err(invalid("row-local disposition order or span overlaps"));
        }
        end = byte_offset
            .checked_add(byte_length)
            .ok_or(census_store::StoreError::CounterOverflow)?;
    }
    Ok(())
}

fn member_digest(
    member: &LocatedRosterAthlete,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<String> {
    let value = member.athlete();
    let current = member.current_roster().map(|current| {
        (
            current.owner(),
            current.member(),
            current.row(),
            current.section(),
            current.container(),
            current.heading(),
            current.heading_sha256(),
            [
                current.supports(census_domain::model::Sport::IndoorTrack),
                current.supports(census_domain::model::Sport::OutdoorTrack),
                current.supports(census_domain::model::Sport::CrossCountry),
            ],
        )
    });
    let bytes = encode(
        &(
            "row-local-admitted-semantics-v2",
            &value.roster_name,
            &value.name,
            value.gender,
            value.grad_year,
            &value.athlete_id,
            &value.profile_url,
            [value.indoor, value.outdoor, value.xc],
            member.locator(),
            current,
        ),
        grant,
    )?;
    Ok(hash(&bytes))
}
