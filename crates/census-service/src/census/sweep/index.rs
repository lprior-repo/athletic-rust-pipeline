use census_crawl::milesplit::{TeamIndexRead, TeamRef};
use census_crawl::{CollectionDisposition as Disposition, CrawlError, CrawlResult};
use census_domain::{model::CensusRun, UsJurisdiction};
use census_store::Store;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use super::super::{teams_phase, SourceObject, StateProgress};

mod ids;
mod inputs;
mod inspection;
#[cfg(test)]
mod tests;
mod work;
pub(crate) use inspection::{inspect_rosters, RosterIndexEvidence};

#[derive(Default, Serialize, Deserialize)]
struct Receipt {
    #[serde(default)]
    disposition: Disposition,
    #[serde(default)]
    roster_teams: Vec<String>,
    #[serde(default)]
    unfinished: Vec<String>,
    #[serde(default)]
    errors: usize,
}

pub(crate) fn team_index_report(
    store: &Store,
    jurisdiction: UsJurisdiction,
) -> CrawlResult<census_crawl::AdapterReport> {
    let mut report = census_crawl::AdapterReport::new(super::super::SOURCE, "teams");
    let Some(receipt) = load(store, jurisdiction)? else {
        return Ok(report);
    };
    let capacity = receipt
        .roster_teams
        .len()
        .checked_add(receipt.unfinished.len())
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "team index obligations overflow".to_string(),
        })?;
    report
        .unfinished
        .try_reserve_exact(capacity)
        .map_err(|_| ids::resource(capacity))?;
    receipt
        .roster_teams
        .iter()
        .try_for_each(|id| -> CrawlResult<()> {
            if inputs::load(store, jurisdiction, id)?.is_some() {
                report.rows = report
                    .rows
                    .checked_add(1)
                    .ok_or_else(|| CrawlError::Arithmetic {
                        detail: "team index report count overflow".to_string(),
                    })?;
            } else {
                report.unfinished.push(format!(
                    "{}/roster-input/{id}/unmeasured",
                    jurisdiction.code()
                ));
            }
            Ok(())
        })?;
    report.disposition = measured_disposition(&receipt, report.unfinished.is_empty());
    report.errors = u64::try_from(receipt.errors).map_err(|_| CrawlError::Arithmetic {
        detail: "team index error count exceeds u64".to_string(),
    })?;
    report.unfinished.extend(receipt.unfinished);
    Ok(report)
}

fn measured_disposition(receipt: &Receipt, inputs_resolved: bool) -> Disposition {
    if receipt.disposition.is_complete()
        && (!inputs_resolved || receipt.errors != 0 || !receipt.unfinished.is_empty())
    {
        Disposition::Partial
    } else {
        receipt.disposition
    }
}

pub(super) fn configure(
    store: &Store,
    jurisdiction: UsJurisdiction,
    read: TeamIndexRead,
) -> CrawlResult<Vec<TeamRef>> {
    let verified = read.disposition.is_complete() && read.errors == 0 && read.unfinished.is_empty();
    let mut fresh = super::unique_teams(read.teams)?;
    let mut receipt =
        load(store, jurisdiction)?.map_or(Receipt::default(), core::convert::identity);
    receipt.roster_teams = ids::merge(receipt.roster_teams, &fresh)?;
    receipt.unfinished = work::merge(receipt.unfinished, read.unfinished)?;
    receipt.errors = read.errors;
    receipt.disposition = Disposition::Partial;
    save(store, jurisdiction, &receipt)?;
    inputs::persist(store, jurisdiction, &fresh)?;
    let extra = retained(store, jurisdiction, &receipt.roster_teams, &fresh)?;
    fresh
        .try_reserve_exact(extra.len())
        .map_err(|_| ids::resource(receipt.roster_teams.len()))?;
    fresh.extend(extra);
    receipt.disposition =
        if verified && receipt.unfinished.is_empty() && fresh.len() == receipt.roster_teams.len() {
            Disposition::Complete
        } else {
            Disposition::Partial
        };
    save(store, jurisdiction, &receipt)?;
    Ok(fresh)
}

fn retained(
    store: &Store,
    jurisdiction: UsJurisdiction,
    configured: &[String],
    fresh: &[TeamRef],
) -> CrawlResult<Vec<TeamRef>> {
    let mut seen = HashSet::new();
    seen.try_reserve(fresh.len())
        .map_err(|_| ids::resource(fresh.len()))?;
    seen.extend(fresh.iter().map(|team| team.id.as_str()));
    let mut extra = Vec::new();
    extra
        .try_reserve_exact(configured.len())
        .map_err(|_| ids::resource(configured.len()))?;
    configured
        .iter()
        .filter(|id| !seen.contains(id.as_str()))
        .try_for_each(|id| {
            if let Some(team) = inputs::load(store, jurisdiction, id)? {
                extra.push(team);
            }
            Ok::<_, CrawlError>(())
        })?;
    Ok(extra)
}

pub(super) fn retain_failure(
    store: &Store,
    jurisdiction: UsJurisdiction,
    error: CrawlError,
) -> CrawlError {
    let saved = (|| {
        let mut receipt =
            load(store, jurisdiction)?.map_or(Receipt::default(), core::convert::identity);
        receipt.disposition = Disposition::Partial;
        receipt.errors = receipt
            .errors
            .checked_add(1)
            .ok_or_else(|| CrawlError::Arithmetic {
                detail: "team index error count overflow".to_string(),
            })?;
        save(store, jurisdiction, &receipt)
    })();
    match saved {
        Ok(()) => error,
        Err(persistence) => CrawlError::Invariant {
            detail: format!("{error}; retaining team index failure also failed: {persistence}"),
        },
    }
}

pub(super) fn guard_progress(
    store: &Store,
    jurisdiction: UsJurisdiction,
    teams: &[TeamRef],
    mut progress: StateProgress,
) -> CrawlResult<StateProgress> {
    let receipt = load(store, jurisdiction)?;
    let covered = receipt
        .as_ref()
        .map(|receipt| covers(receipt, teams))
        .transpose()?
        .is_some_and(core::convert::identity);
    let total = receipt.as_ref().map_or(progress.rosters_total, |receipt| {
        receipt.roster_teams.len().max(progress.rosters_total)
    });
    progress.rosters_remaining =
        super::roster::remaining(total, progress.rosters_committed, progress.rosters_skipped)?;
    progress.rosters_total = total;
    progress.teams = total;
    if !covered
        || !receipt.as_ref().is_some_and(|receipt| {
            receipt.disposition.is_complete()
                && receipt.errors == 0
                && receipt.unfinished.is_empty()
        })
    {
        progress
            .errors
            .try_reserve(1)
            .map_err(|_| ids::resource(total))?;
        progress.errors.push(format!(
            "{} team index inventory remains unmeasured or unresolved",
            jurisdiction.code()
        ));
    }
    Ok(progress)
}

fn covers(receipt: &Receipt, teams: &[TeamRef]) -> CrawlResult<bool> {
    let mut supplied = HashSet::new();
    supplied
        .try_reserve(teams.len())
        .map_err(|_| ids::resource(teams.len()))?;
    supplied.extend(teams.iter().map(|team| team.id.as_str()));
    Ok(supplied.len() == receipt.roster_teams.len()
        && receipt
            .roster_teams
            .iter()
            .all(|id| supplied.contains(id.as_str())))
}

fn load(store: &Store, jurisdiction: UsJurisdiction) -> CrawlResult<Option<Receipt>> {
    let Some(payload) = store.journal_payload(&teams_phase(jurisdiction), jurisdiction.code())?
    else {
        return Ok(None);
    };
    let legacy = payload.get("roster_teams").is_none();
    let mut receipt: Receipt =
        serde_json::from_value(payload).map_err(|source| CrawlError::Canonical {
            table: "milesplit_team_index".to_string(),
            source: census_domain::model::CanonicalJsonError::Unsupported(source.to_string()),
        })?;
    ids::validate(&receipt.roster_teams)?;
    work::validate(&receipt.unfinished)?;
    if legacy && receipt.unfinished.is_empty() {
        receipt
            .unfinished
            .try_reserve_exact(1)
            .map_err(|_| ids::resource(1))?;
        receipt
            .unfinished
            .push("legacy-inventory-unmeasured".to_string());
        receipt.disposition = Disposition::Partial;
    }
    Ok(Some(receipt))
}

fn save(store: &Store, jurisdiction: UsJurisdiction, receipt: &Receipt) -> CrawlResult<()> {
    store.journal_done(&teams_phase(jurisdiction), jurisdiction.code(), receipt)?;
    Ok(())
}
