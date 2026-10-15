use crate::milesplit::wire::{
    CurrentRosterAssertion, LocatedRosterAthlete, RosterTemporalUnavailable as Gap, TeamRef,
};
use scan::{scan, Scan};
#[cfg(test)]
use scan::{MAX_DEPTH, MAX_HEADING_BYTES};
#[cfg(test)]
use sha2::{Digest, Sha256};

mod scan;
type TemporalResult<T> = Result<T, Gap>;

pub(super) fn bind(
    html: &str,
    team: &TeamRef,
    members: &mut [LocatedRosterAthlete],
    fields: &super::Fields<'_>,
) {
    let scan = match scan(html) {
        Ok(scan) => scan,
        Err(reason) => {
            for member in members {
                member.withhold_temporal(reason);
            }
            return;
        }
    };
    for member in members {
        let result = assertion(html, team, member, fields, &scan);
        match result {
            Ok(proof) => member.bind_current_roster(proof),
            Err(reason) => member.withhold_temporal(reason),
        }
    }
}

fn assertion(
    html: &str,
    team: &TeamRef,
    member: &LocatedRosterAthlete,
    fields: &super::Fields<'_>,
    scan: &Scan,
) -> TemporalResult<CurrentRosterAssertion> {
    let row = member.locator();
    let index = scan
        .rows
        .binary_search_by_key(&row.byte_offset, |row| row.span.start)
        .map_err(|_| Gap::InactiveMember)?;
    let active = scan.rows.get(index).ok_or(Gap::MalformedMarkup)?;
    let scope = scan.scopes.get(active.scope).ok_or(Gap::MalformedMarkup)?;
    if scope.heading_count > 1 || scope.container_count > 1 {
        return Err(Gap::AmbiguousStatement);
    }
    let heading = scope.heading.as_ref().ok_or(Gap::MissingStatement)?;
    if scope.section.end == 0
        || scope.container.end == 0
        || heading.span.end > scope.container.start
    {
        return Err(Gap::MalformedMarkup);
    }
    if row.byte_offset.checked_add(row.byte_length) != Some(active.span.end) {
        return Err(Gap::InactiveMember);
    }
    let live = fields
        .athlete(&active.active, row)
        .map_err(|_| Gap::InactiveMember)?;
    if &live != member.athlete() {
        return Err(Gap::InactiveMember);
    }
    if html.get(heading.span.clone()).is_none() {
        return Err(Gap::MalformedMarkup);
    }
    Ok(CurrentRosterAssertion {
        owner: copy(&team.id)?,
        member: copy(&live.athlete_id)?,
        row,
        section: scope.section.clone(),
        container: scope.container.clone(),
        heading: heading.span.clone(),
        heading_sha256: heading.sha256,
        sports: [live.indoor, live.outdoor, live.xc],
    })
}

fn copy(value: &str) -> TemporalResult<String> {
    let mut result = String::new();
    result
        .try_reserve_exact(value.len())
        .map_err(|_| Gap::ResourceLimit)?;
    result.push_str(value);
    Ok(result)
}

#[cfg(test)]
mod tests;
