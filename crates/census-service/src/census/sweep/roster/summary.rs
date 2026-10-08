use super::journal::Journal;
use census_crawl::milesplit::TeamRef;
use census_crawl::{CrawlError, CrawlResult};
use census_domain::{model::SchoolYear, UsJurisdiction};
use census_store::Store;
use std::collections::HashSet;

#[derive(Default)]
pub(in crate::census::sweep) struct Summary {
    pub(in crate::census::sweep) total: usize,
    pub(in crate::census::sweep) committed: usize,
    pub(in crate::census::sweep) held: usize,
    pub(in crate::census::sweep) athletes: usize,
    pub(in crate::census::sweep) co2027: usize,
    pub(in crate::census::sweep) co2027_boys: usize,
    pub(in crate::census::sweep) co2027_girls: usize,
    pub(in crate::census::sweep) errors: Vec<String>,
}

pub(in crate::census::sweep) fn summarize(
    store: &Store,
    phase: &str,
    teams: &[TeamRef],
    jurisdiction: UsJurisdiction,
    year: SchoolYear,
) -> CrawlResult<Summary> {
    if teams.len() > 20_000 {
        return Err(resource(teams.len()));
    }
    let mut seen = HashSet::new();
    seen.try_reserve(teams.len())
        .map_err(|_| resource(teams.len()))?;
    let mut summary = Summary::default();
    summary
        .errors
        .try_reserve_exact(teams.len())
        .map_err(|_| resource(teams.len()))?;
    teams.iter().try_fold(summary, |mut summary, team| {
        if !seen.insert(team.id.as_str()) {
            return Ok(summary);
        }
        summary.total = sum(summary.total, 1)?;
        let key = format!("{}:{}", jurisdiction.code(), team.id);
        let Some(payload) = store.journal_payload(phase, &key)? else {
            return Ok(summary);
        };
        let row: Journal =
            serde_json::from_value(payload).map_err(|source| CrawlError::Decode {
                url: format!("journal:{phase}/{key}"),
                source,
            })?;
        if !row.matches_scope(&team.id, year) {
            return Err(CrawlError::Invariant {
                detail: format!(
                    "roster journal {key} has foreign identity {}",
                    row.team_id()
                ),
            });
        }
        include(summary, &row)
    })
}

fn resource(requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "roster summary",
        requested,
        limit: 20_000,
    }
}

fn include(mut summary: Summary, row: &Journal) -> CrawlResult<Summary> {
    if row.is_terminal() {
        summary.committed = sum(summary.committed, 1)?;
    } else {
        summary.held = sum(summary.held, 1)?;
        summary.errors.push(problem(row));
    }
    summary.athletes = sum(summary.athletes, row.athletes)?;
    summary.co2027 = sum(summary.co2027, row.co2027)?;
    summary.co2027_boys = sum(summary.co2027_boys, row.co2027_boys)?;
    summary.co2027_girls = sum(summary.co2027_girls, row.co2027_girls)?;
    Ok(summary)
}

fn problem(row: &Journal) -> String {
    let source = row.capture.as_ref().map_or_else(
        || format!("team {}", row.team_id),
        |capture| capture.url.clone(),
    );
    match &row.refusal {
        Some(reason) => format!("{source}: refusal={reason}"),
        None => format!(
            "{source}: quarantine={:?}; rejected_rows={}",
            row.quarantine,
            row.rejected.len()
        ),
    }
}

pub(in crate::census::sweep) fn remaining(
    total: usize,
    committed: usize,
    _held: usize,
) -> CrawlResult<usize> {
    total
        .checked_sub(committed)
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "committed rosters exceeded the indexed teams".to_string(),
        })
}

fn sum(left: usize, right: usize) -> CrawlResult<usize> {
    left.checked_add(right)
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "roster summary count overflowed".to_string(),
        })
}
