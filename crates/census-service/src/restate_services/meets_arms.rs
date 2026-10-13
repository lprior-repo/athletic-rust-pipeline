mod diagnostics;
mod prerequisites;
#[cfg(test)]
mod tests;
mod walks;

use super::history_stage::HistoricalStageScope;
use super::jobs;
use crate::census::{MeetCensus, MeetSourceRows};
use census_crawl::{net::Fetcher, AdapterReport, CollectionDisposition, Recording};
use census_domain::model::SchoolYear;
use census_store::Store;
use restate_sdk::prelude::{HandlerError, Json, TerminalError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub(super) use walks::required;

pub(super) const MEETS_ARMS: &[(&str, MeetsArm)] = &[
    (crate::census::SOURCE, MeetsArm::MilesplitIndex),
    ("athleticnet", MeetsArm::AthleticnetHistoricalDiscovery),
    ("wiaa_results", MeetsArm::WiaaResults),
    ("wayzata", MeetsArm::Wayzata),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MeetsStageOutcome {
    pub census: MeetCensus,
    pub identity: String,
    pub spool: census_crawl::recording::spool::FragmentSummary,
    pub next: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MeetsArm {
    MilesplitIndex,
    AthleticnetHistoricalDiscovery,
    WiaaResults,
    Wayzata,
}

pub(super) fn arm_for(slug: &str) -> Option<MeetsArm> {
    MEETS_ARMS
        .iter()
        .find(|(planned, _)| *planned == slug)
        .map(|(_, arm)| *arm)
}

pub(super) async fn collect_fragment(
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    scope: HistoricalStageScope,
    source: (String, usize),
    identity: String,
) -> Result<Json<MeetsStageOutcome>, HandlerError> {
    const PHASE: &str = "meet_fragment_outcomes_v1";
    if let Some(previous) = store
        .journal_payload(PHASE, &identity)
        .map_err(store_error)?
    {
        return serde_json::from_value(previous)
            .map(Json)
            .map_err(|error| jobs::invariant(&format!("decoding durable meet fragment: {error}")));
    }
    let (slug, ordinal) = source;
    let recording = Recording::spooled(store.as_ref().clone(), identity.clone())
        .map_err(jobs::collect_error)?;
    let collected = match arm_for(&slug) {
        Some(arm) => walks::fragment(&store, &fetcher, scope, (arm, ordinal), &recording).await,
        None => walks::unarmed(&store, &fetcher, scope, &slug, &recording).await,
    };
    let (census, more) = match collected {
        Ok(outcome) => outcome,
        Err(error) => (failed_census(&slug, scope, format!("{error:?}")), false),
    };
    let census = diagnostics::bounded_census(&store, census, &identity, ordinal)?;
    let spool = recording
        .finish_fragments()
        .map_err(|error| super::job_error(jobs::collect_error(error)))?;
    let next = if more {
        Some(
            ordinal
                .checked_add(1)
                .ok_or_else(|| jobs::invariant("meet fragment cursor overflow"))?,
        )
    } else {
        None
    };
    let outcome = MeetsStageOutcome {
        census,
        identity,
        spool,
        next,
    };
    store
        .journal_done(PHASE, &outcome.identity, &outcome)
        .map_err(store_error)?;
    Ok(Json(outcome))
}

pub(super) fn absorb_fragment(
    slug: &str,
    mut census: MeetCensus,
    mut next: MeetCensus,
) -> Result<MeetCensus, HandlerError> {
    if slug == crate::census::SOURCE {
        next.pages = add(census.pages, next.pages)?;
        next.fetched = add(census.fetched, next.fetched)?;
        next.seen = add(census.seen, next.seen)?;
        return Ok(next);
    }
    census.rows = add(census.rows, next.rows)?;
    let Some(mut source) = next.sources.pop() else {
        return Ok(census);
    };
    if let Some(previous) = census.sources.pop() {
        source.rows = add(previous.rows, source.rows)?;
        source.withheld = match (previous.withheld, source.withheld) {
            (Some(left), Some(right)) => Some(
                left.checked_add(right)
                    .ok_or_else(|| jobs::invariant("meet withheld count overflow"))?,
            ),
            _ => None,
        };
        source.unresolved = match (previous.unresolved, source.unresolved) {
            (Some(left), Some(right)) => Some(census_crawl::UnresolvedCounters {
                rows: left
                    .rows
                    .checked_add(right.rows)
                    .ok_or_else(|| jobs::invariant("meet unresolved rows overflow"))?,
                labels: left
                    .labels
                    .checked_add(right.labels)
                    .ok_or_else(|| jobs::invariant("meet unresolved labels overflow"))?,
            }),
            _ => None,
        };
        if source.errors.is_empty() {
            source.errors = previous.errors;
        }
        if source.unfinished.is_empty() {
            source.unfinished = previous.unfinished;
        }
        if !source.errors.is_empty() || !source.unfinished.is_empty() {
            source.disposition = CollectionDisposition::Partial;
        }
    }
    census.sources.push(source);
    Ok(census)
}

pub(super) fn absorb(
    mut census: MeetCensus,
    extra: MeetCensus,
) -> Result<MeetCensus, HandlerError> {
    census.pages = add(census.pages, extra.pages)?;
    census.fetched = add(census.fetched, extra.fetched)?;
    census.seen = add(census.seen, extra.seen)?;
    census.rows = add(census.rows, extra.rows)?;
    census.seasons = add(census.seasons, extra.seasons)?;
    census.truncated = add(census.truncated, extra.truncated)?;
    census.repeated = add(census.repeated, extra.repeated)?;
    census
        .sources
        .try_reserve(extra.sources.len())
        .map_err(|_| jobs::invariant("meet source allocation"))?;
    census
        .frontiers
        .try_reserve(extra.frontiers.len())
        .map_err(|_| jobs::invariant("meet frontier allocation"))?;
    census.sources.extend(extra.sources);
    census.frontiers.extend(extra.frontiers);
    Ok(census)
}

fn add(left: usize, right: usize) -> Result<usize, HandlerError> {
    left.checked_add(right)
        .ok_or_else(|| jobs::invariant("meet census counter overflow"))
}

pub(super) fn report_census(slug: &str, report: AdapterReport) -> Result<MeetCensus, HandlerError> {
    let rows = jobs::rows_written(&report)?;
    let errors = if report.errors > 0
        || report.rejections > 0
        || report
            .unresolved
            .is_some_and(|value| value.rows > 0 || value.labels > 0)
    {
        std::iter::once(format!(
            "{} errors, {} rejected; unresolved {:?}: {}",
            report.errors,
            report.rejections,
            report.unresolved,
            report.notes.join("; ")
        ))
        .collect()
    } else {
        Vec::new()
    };
    Ok(MeetCensus {
        rows,
        sources: vec![MeetSourceRows {
            slug: slug.to_string(),
            rows,
            disposition: report.disposition,
            unfinished: report.unfinished,
            errors,
            notes: report.notes,
            withheld: Some(report.rejections),
            unresolved: report.unresolved.or_else(|| {
                report
                    .disposition
                    .is_complete()
                    .then_some(census_crawl::UnresolvedCounters { rows: 0, labels: 0 })
            }),
        }],
        ..MeetCensus::default()
    })
}

pub(super) fn failed_census(
    slug: &str,
    scope: HistoricalStageScope,
    message: String,
) -> MeetCensus {
    MeetCensus {
        sources: vec![MeetSourceRows {
            slug: slug.to_string(),
            rows: 0,
            disposition: CollectionDisposition::Failed,
            unfinished: vec![format!(
                "{slug}/{}/{}",
                scope.jurisdiction.code(),
                scope.year
            )],
            errors: vec![message],
            notes: Vec::new(),
            withheld: None,
            unresolved: None,
        }],
        ..MeetCensus::default()
    }
}

pub(super) fn season_of(year: u16) -> Result<SchoolYear, HandlerError> {
    let start_year = i16::try_from(year).map_err(|_| not_a_season(year))?;
    SchoolYear::new(start_year).ok_or_else(|| not_a_season(year))
}

pub(super) fn not_a_season(year: u16) -> HandlerError {
    TerminalError::new(format!("season year {year} is not a school year")).into()
}

fn store_error(error: census_store::StoreError) -> HandlerError {
    super::job_error(super::JobError::from(error))
}
