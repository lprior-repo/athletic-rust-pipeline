#[cfg(test)]
mod tests;
mod walks;

use super::jobs;
use super::{history_stage::HistoricalStageScope, source_selection::SourceSelection};
use crate::census::{MeetCensus, MeetSourceRows};
use census_crawl::{net::Fetcher, AdapterReport, CollectionDisposition, Recorded, Recording};
use census_domain::model::SchoolYear;
use census_store::Store;
use restate_sdk::prelude::{HandlerError, Json, TerminalError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub(super) const MEETS_ARMS: &[(&str, MeetsArm)] = &[
    (crate::census::SOURCE, MeetsArm::MilesplitIndex),
    ("wiaa_results", MeetsArm::WiaaResults),
    ("wayzata", MeetsArm::Wayzata),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MeetsStageOutcome {
    pub census: MeetCensus,
    pub recorded: Vec<RecordedSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RecordedSource {
    pub slug: String,
    pub recorded: Recorded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MeetsArm {
    MilesplitIndex,
    WiaaResults,
    Wayzata,
}

pub(super) fn arm_for(slug: &str) -> Option<MeetsArm> {
    MEETS_ARMS
        .iter()
        .find(|(planned, _)| *planned == slug)
        .map(|(_, arm)| *arm)
}

pub(super) async fn meets_stage(
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    scope: HistoricalStageScope,
    sources: SourceSelection,
) -> Result<Json<MeetsStageOutcome>, HandlerError> {
    use futures::{stream, StreamExt, TryStreamExt};
    stream::iter(sources.iter())
        .map(Ok::<_, HandlerError>)
        .try_fold(
            MeetsStageOutcome {
                census: MeetCensus::default(),
                recorded: Vec::new(),
            },
            |outcome, source| collect_source(&store, &fetcher, scope, source.slug, outcome),
        )
        .await
        .map(Json)
}

async fn collect_source(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    scope: HistoricalStageScope,
    slug: &str,
    mut outcome: MeetsStageOutcome,
) -> Result<MeetsStageOutcome, HandlerError> {
    let Some(arm) = arm_for(slug) else {
        jobs::assert_some_stage_arms(slug)?;
        return Ok(outcome);
    };
    let recording = Recording::new();
    let walked = walks::armed(store, fetcher, scope, arm, &recording).await;
    take_recorded(slug, recording.drain(), &mut outcome.recorded)?;
    let census = match walked {
        Ok(census) => census,
        Err(error) => {
            let source: &dyn std::error::Error = error.as_ref();
            failed_census(slug, scope, source.to_string())
        }
    };
    outcome.census = absorb(outcome.census, census)?;
    Ok(outcome)
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

fn take_recorded(
    slug: &str,
    walked: Recorded,
    recorded: &mut Vec<RecordedSource>,
) -> Result<(), HandlerError> {
    if walked.is_empty() {
        return Ok(());
    }
    if recorded.len() >= MEETS_ARMS.len() {
        return Err(jobs::invariant("meet recording source ceiling"));
    }
    recorded
        .try_reserve(1)
        .map_err(|_| jobs::invariant("meet recording allocation"))?;
    recorded.push(RecordedSource {
        slug: slug.to_string(),
        recorded: walked,
    });
    Ok(())
}

pub(super) fn season_of(year: u16) -> Result<SchoolYear, HandlerError> {
    let start_year = i16::try_from(year).map_err(|_| not_a_season(year))?;
    SchoolYear::new(start_year).ok_or_else(|| not_a_season(year))
}

pub(super) fn not_a_season(year: u16) -> HandlerError {
    TerminalError::new(format!("season year {year} is not a school year")).into()
}
