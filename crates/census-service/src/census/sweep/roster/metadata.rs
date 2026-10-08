use census_crawl::milesplit::{Roster, RosterAthlete, TeamRef};
use census_crawl::{CrawlError, CrawlResult};
use std::ops::ControlFlow;

const MAX_BYTES: usize = 4096;
const MAX_PROJECTED: usize = 8 * 1024 * 1024;
const MAX_WINDOW_ROWS: usize = 64;

pub(super) struct Window<'a> {
    team: &'a TeamRef,
    athletes: &'a [RosterAthlete],
    observed_on: &'a str,
}

impl<'a> Window<'a> {
    pub(super) fn next(
        roster: &'a Roster,
        start: usize,
        observed_on: &'a str,
    ) -> CrawlResult<Option<Self>> {
        let common = common(&roster.team, observed_on)?;
        let remaining = roster.athletes.get(start..).ok_or_else(arithmetic)?;
        if remaining.is_empty() {
            return Ok(None);
        }
        let count = window_len(remaining, common)?;
        let athletes = remaining.get(..count).ok_or_else(arithmetic)?;
        Ok(Some(Self {
            team: &roster.team,
            athletes,
            observed_on,
        }))
    }
    pub(super) fn team(&self) -> &'a TeamRef {
        self.team
    }
    pub(super) fn athletes(&self) -> &'a [RosterAthlete] {
        self.athletes
    }
    pub(super) fn observed_on(&self) -> &'a str {
        self.observed_on
    }
}

fn common(team: &TeamRef, observed_on: &str) -> CrawlResult<usize> {
    [
        ("roster school name", team.name.as_str()),
        ("roster source URL", team.url.as_str()),
        ("roster team identity", team.id.as_str()),
        ("roster location", team.city_state.as_str()),
        ("roster observed date", observed_on),
    ]
    .into_iter()
    .try_fold(0, |bytes, (resource, value)| {
        if value.len() > MAX_BYTES {
            return Err(CrawlError::Resource {
                resource,
                requested: value.len(),
                limit: MAX_BYTES,
            });
        }
        add(bytes, value.len())
    })
}

fn window_len(rows: &[RosterAthlete], common: usize) -> CrawlResult<usize> {
    let initial = add(scale(common, 32)?, 16 * 1024)?;
    let outcome =
        rows.iter()
            .take(MAX_WINDOW_ROWS)
            .try_fold((initial, 0_usize), |(used, count), row| {
                match row_bytes(row, common).and_then(|bytes| add(used, bytes)) {
                    Ok(next) if next <= MAX_PROJECTED => match count.checked_add(1) {
                        Some(count) => ControlFlow::Continue((next, count)),
                        None => ControlFlow::Break(Err(arithmetic())),
                    },
                    Ok(bytes) if count == 0 => ControlFlow::Break(Err(projected_resource(bytes))),
                    Err(error) if count == 0 => ControlFlow::Break(Err(error)),
                    _ => ControlFlow::Break(Ok(count)),
                }
            });
    match outcome {
        ControlFlow::Continue((_, count)) => Ok(count),
        ControlFlow::Break(result) => result,
    }
}

fn row_bytes(row: &RosterAthlete, common: usize) -> CrawlResult<usize> {
    let raw = [
        row.name.len(),
        row.roster_name.len(),
        row.athlete_id.len(),
        row.profile_url.len(),
    ]
    .into_iter()
    .try_fold(common, add)?;
    add(scale(raw, 24)?, 16 * 1024)
}

pub(super) fn school_name(value: &str) -> CrawlResult<String> {
    if value.len() > MAX_BYTES {
        return Err(name_resource(value.len()));
    }
    let mut name = String::new();
    name.try_reserve_exact(value.len())
        .map_err(|_| name_resource(value.len()))?;
    name.push_str(value);
    Ok(name)
}

fn add(left: usize, right: usize) -> CrawlResult<usize> {
    left.checked_add(right).ok_or_else(arithmetic)
}
fn scale(bytes: usize, copies: usize) -> CrawlResult<usize> {
    bytes.checked_mul(copies).ok_or_else(arithmetic)
}
fn arithmetic() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "roster projection capacity overflowed".to_string(),
    }
}
fn projected_resource(requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "roster projected bytes",
        requested,
        limit: MAX_PROJECTED,
    }
}
fn name_resource(requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "roster school name",
        requested,
        limit: MAX_BYTES,
    }
}
