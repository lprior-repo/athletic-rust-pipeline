use census_crawl::milesplit::{self, boundary, TeamRef};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{CrawlError, CrawlResult};
use census_store::{Application, Store, StoreResult};
use std::collections::HashSet;

use super::{rosters_phase, units::RosterRun};

mod journal;
pub(super) mod refusal;

use journal::{quarantine_of, roster_digest, roster_journal, roster_operation, Journal, Records};
pub(super) use refusal::retain_refusal;

#[cfg(test)]
mod tests;

pub(super) async fn fetch_and_store(
    fetcher: &Fetcher,
    store: &Store,
    team: &TeamRef,
    run: &RosterRun<'_>,
) -> CrawlResult<Application> {
    let hook = boundary_hook(run);
    let read = milesplit::fetch_roster(
        fetcher,
        team,
        &FetchOptions {
            refresh: run.refresh,
            allow_not_found: true,
            headers: Vec::new(),
        },
        hook,
    )
    .await?;
    let roster = read.verdict.roster();
    let records = roster
        .map(|roster| Records::of(roster, &run.site, run.school_year, run.observed_on))
        .transpose()?;
    let quarantine = quarantine_of(&read.verdict);
    let journal = roster_journal(
        team,
        &read,
        records.as_ref(),
        run.school_year,
        run.observed_on,
        roster,
    );
    let digest = roster_digest(
        &records,
        team,
        run.school_year,
        run.observed_on,
        &read,
        quarantine,
    )?;
    let state = run.site.jurisdiction().code();
    let operation = roster_operation(state, run.school_year, run.revision, team);
    let phase = rosters_phase(run.site.jurisdiction(), run.school_year, run.revision);
    let mut batch = store.write_batch();
    if let Some(records) = &records {
        records.stage(&mut batch)?;
    }
    if let Some(hook) = hook {
        hook.reached(boundary::Point::BeforeApply).await?;
    }
    batch.journal_done(&phase, &format!("{state}:{}", team.id), &journal)?;
    let application = batch.commit_once(&operation, &digest)?;
    if let Some(hook) = hook {
        hook.reached(boundary::Point::AfterCommitBeforeAck).await?;
    }
    Ok(application)
}

#[derive(Default)]
pub(super) struct Summary {
    pub(super) committed: usize,
    pub(super) held: usize,
    pub(super) athletes: usize,
    pub(super) co2027: usize,
    pub(super) co2027_boys: usize,
    pub(super) co2027_girls: usize,
    pub(super) errors: Vec<String>,
}

pub(super) fn summarize(store: &Store, phase: &str, teams: &[TeamRef]) -> CrawlResult<Summary> {
    let wanted: HashSet<&str> = teams.iter().map(|team| team.id.as_str()).collect();
    let mut clean: HashSet<String> = HashSet::new();
    let mut unclean: HashSet<String> = HashSet::new();
    let mut summary = Summary::default();
    for payload in store.journal_payloads(phase)? {
        let row: Journal =
            serde_json::from_value(payload).map_err(|source| CrawlError::Decode {
                url: format!("journal:{phase}"),
                source,
            })?;
        if !wanted.contains(row.team_id.as_str()) {
            continue;
        }
        if row.refusal.is_none() && row.quarantine.is_none() && row.rejected.is_empty() {
            clean.insert(row.team_id.clone());
        } else {
            unclean.insert(row.team_id.clone());
            let source = row.capture.as_ref().map_or_else(
                || format!("team {}", row.team_id),
                |capture| capture.url.clone(),
            );
            match &row.refusal {
                Some(reason) => summary.errors.push(format!("{source}: refusal={reason}")),
                None => summary.errors.push(format!(
                    "{}: quarantine={:?}; rejected_rows={}",
                    source,
                    row.quarantine,
                    row.rejected.len()
                )),
            }
        }
        summary.athletes = sum(summary.athletes, row.athletes)?;
        summary.co2027 = sum(summary.co2027, row.co2027)?;
        summary.co2027_boys = sum(summary.co2027_boys, row.co2027_boys)?;
        summary.co2027_girls = sum(summary.co2027_girls, row.co2027_girls)?;
    }
    summary.committed = clean.len();
    summary.held = unclean.difference(&clean).count();
    Ok(summary)
}
pub(crate) fn roster_is_complete(
    store: &Store,
    phase: &str,
    team: &TeamRef,
) -> StoreResult<bool> {
    let payloads = store.journal_payloads(phase)?;
    let mut complete = false;
    for payload in payloads {
        let row: Journal = serde_json::from_value(payload).map_err(|source| {
            census_store::StoreError::Decode {
                key: phase.to_string(),
                source,
            }
        })?;
        if row.team_id != team.id {
            continue;
        }
        if row.quarantine.is_some() || !row.rejected.is_empty() {
            return Ok(false);
        }
        complete = true;
    }
    Ok(complete)
}

pub(super) fn remaining(total: usize, committed: usize, held: usize) -> CrawlResult<usize> {
    committed
        .checked_add(held)
        .and_then(|attempted| total.checked_sub(attempted))
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "roster attempts exceeded the indexed teams".to_string(),
        })
}

fn sum(left: usize, right: usize) -> CrawlResult<usize> {
    left.checked_add(right)
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "roster summary count overflowed".to_string(),
        })
}

#[cfg(feature = "native-fault-injection")]
static BOUNDARY_HOOK: std::sync::OnceLock<&'static dyn boundary::Hook> = std::sync::OnceLock::new();

#[cfg(feature = "native-fault-injection")]
pub fn install_boundary_hook(hook: &'static dyn boundary::Hook) {
    match BOUNDARY_HOOK.set(hook) {
        Ok(()) | Err(_) => {}
    }
}

fn boundary_hook(_run: &RosterRun<'_>) -> Option<&'static dyn boundary::Hook> {
    #[cfg(feature = "native-fault-injection")]
    {
        BOUNDARY_HOOK.get().copied()
    }
    #[cfg(not(feature = "native-fault-injection"))]
    {
        None
    }
}
