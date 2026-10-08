mod collection;
mod selection;
use std::sync::Arc;

use census_crawl::net::Fetcher;
use census_crawl::{AdapterContext, AdapterReport, CollectionDisposition, UnresolvedCounters};
use census_domain::model::SourceMeetRef;
use census_domain::UsJurisdiction;
use census_store::Store;
use restate_sdk::prelude::{HandlerError, Json};
use serde::{Deserialize, Serialize};

use super::job_error;
use super::jobs::{assert_some_stage_arms, collect_error, rows_written};
use super::{history_stage::HistoricalStageScope, source_selection::SourceSelection};

const MILESPLIT: &str = "milesplit";
const ATHLETICNET: &str = "athleticnet";

const ATHLETICNET_HOST: &str = "athletic.net";

const ATHLETICNET_HOST_SUFFIX: &str = ".athletic.net";

pub(super) const RESULTS_ARMS: &[(&str, ResultsArm)] = &[
    (MILESPLIT, ResultsArm::MilesplitResults),
    (ATHLETICNET, ResultsArm::AthleticnetMeets),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResultsArm {
    MilesplitResults,
    AthleticnetMeets,
}

pub(super) fn arm_for(slug: &str) -> Option<ResultsArm> {
    RESULTS_ARMS
        .iter()
        .find(|(table_slug, _)| *table_slug == slug)
        .map(|(_, arm)| *arm)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultsStageOutcome {
    pub per_source: Vec<ResultsSourceRows>,
    #[serde(default)]
    pub pending: Vec<String>,
    #[serde(default)]
    pub required_sources: Vec<String>,
}

impl ResultsStageOutcome {
    pub fn is_terminal(&self) -> bool {
        self.pending.is_empty()
            && !self.required_sources.is_empty()
            && self.required_sources.iter().all(|required| {
                self.per_source
                    .iter()
                    .filter(|source| source.slug == *required)
                    .count()
                    == 1
                    && self
                        .per_source
                        .iter()
                        .any(|source| source.slug == *required && source_complete(source))
            })
            && self.per_source.iter().all(source_complete)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultsSourceRows {
    pub slug: String,
    pub meets: usize,
    pub rows: Option<usize>,
    #[serde(default)]
    pub disposition: census_crawl::CollectionDisposition,
    #[serde(default)]
    pub errors: u64,
    #[serde(default)]
    pub withheld: Option<u64>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub unfinished: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved: Option<UnresolvedCounters>,
}

pub(super) async fn results_stage(
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    scope: HistoricalStageScope,
    sources: SourceSelection,
) -> Result<Json<ResultsStageOutcome>, HandlerError> {
    use futures::{stream, StreamExt, TryStreamExt};
    let selected = selection::select(&store, scope)?;
    let required_sources =
        sources
            .iter()
            .filter_map(required_slug)
            .try_fold(Vec::new(), |mut required, slug| {
                required
                    .try_reserve(1)
                    .map_err(|_| super::jobs::invariant("result source allocation"))?;
                required.push(slug.to_string());
                Ok::<_, HandlerError>(required)
            })?;
    let (store, fetcher, meets) = (&store, &fetcher, selected.meets.as_slice());
    let per_source = stream::iter(sources.iter().filter_map(required_slug))
        .map(Ok::<_, HandlerError>)
        .try_fold(Vec::new(), |mut rows, slug| async move {
            let row = collect_source(slug, store, fetcher, meets, scope).await?;
            rows.try_reserve(1)
                .map_err(|_| super::jobs::invariant("result source allocation"))?;
            rows.push(row);
            Ok(rows)
        })
        .await?;
    Ok(Json(ResultsStageOutcome {
        per_source,
        pending: selected.pending,
        required_sources,
    }))
}

fn required(source: &census_crawl::SourceDescriptor) -> bool {
    source.capabilities.bulk_results || source.capabilities.pr_evidence
}

fn required_slug(source: &'static census_crawl::SourceDescriptor) -> Option<&'static str> {
    required(source).then_some(source.slug)
}

fn source_complete(source: &ResultsSourceRows) -> bool {
    source.disposition.is_complete()
        && source.rows.is_some()
        && source.errors == 0
        && source.withheld == Some(0)
        && source.unfinished.is_empty()
        && source
            .unresolved
            .is_some_and(|value| value.rows == 0 && value.labels == 0)
}

async fn collect_source(
    slug: &str,
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    selected: &[SourceMeetRef],
    scope: HistoricalStageScope,
) -> Result<ResultsSourceRows, HandlerError> {
    let Some(arm) = arm_for(slug) else {
        assert_some_stage_arms(slug)?;
        return Ok(delegated_source(slug));
    };
    collection::collect(slug, arm, store, fetcher, (selected, scope)).await
}

fn delegated_source(slug: &str) -> ResultsSourceRows {
    ResultsSourceRows {
        slug: slug.to_string(),
        meets: 0,
        rows: None,
        disposition: CollectionDisposition::Unknown,
        errors: 0,
        withheld: None,
        notes: Vec::new(),
        unfinished: vec![format!("{slug}/acquisition-stage-outcome")],
        unresolved: None,
    }
}

fn failed_source(slug: &str, meets: usize, error: HandlerError) -> ResultsSourceRows {
    let source: &dyn std::error::Error = error.as_ref();
    ResultsSourceRows {
        slug: slug.to_string(),
        meets,
        rows: None,
        disposition: CollectionDisposition::Failed,
        errors: 1,
        withheld: None,
        notes: vec![source.to_string()],
        unfinished: Vec::new(),
        unresolved: None,
    }
}

fn source_rows(
    slug: &str,
    meets: usize,
    report: AdapterReport,
) -> Result<ResultsSourceRows, HandlerError> {
    Ok(ResultsSourceRows {
        slug: slug.to_string(),
        meets,
        rows: Some(rows_written(&report)?),
        disposition: report.disposition,
        errors: report.errors,
        withheld: Some(report.rejections),
        notes: report.notes,
        unfinished: report.unfinished,
        unresolved: report.unresolved.or_else(|| {
            report
                .disposition
                .is_complete()
                .then_some(UnresolvedCounters { rows: 0, labels: 0 })
        }),
    })
}

async fn milesplit_results(
    context: &AdapterContext<'_>,
    meet: &SourceMeetRef,
) -> Result<(usize, AdapterReport), HandlerError> {
    let page = census_crawl::milesplit::MeetPage {
        results_url: meet.results_url.clone(),
        jurisdiction: meet.jurisdiction,
    };
    let pages = census_crawl::milesplit::read_meet_pages(
        context.fetcher,
        std::iter::once(page),
        &context.fetch_options(),
    )
    .await
    .map_err(|error| job_error(collect_error(error)))?;
    let urls = pages.files.iter().try_fold(Vec::new(), |mut urls, file| {
        if urls.len() >= 4096 {
            return Err(super::jobs::invariant(
                "meet result-set frontier exceeds 4096",
            ));
        }
        urls.try_reserve(1)
            .map_err(|_| super::jobs::invariant("meet result-set allocation"))?;
        urls.push(file.request());
        Ok(urls)
    })?;
    let report = census_crawl::milesplit::collect_result_sets(
        context,
        &census_crawl::milesplit::ResultSetOptions { urls },
    )
    .await
    .map_err(|error| job_error(collect_error(error)))?;
    let mut report = record_quarantines(report, pages.quarantined)?;
    report.note(format!("meet_pages_read={}", pages.pages_read));
    Ok((1, report))
}

fn record_quarantines(
    report: AdapterReport,
    quarantined: Vec<(String, String)>,
) -> Result<AdapterReport, HandlerError> {
    quarantined
        .into_iter()
        .try_fold(report, |mut report, (url, reason)| {
            report.errors = report
                .errors
                .checked_add(1)
                .ok_or_else(|| super::jobs::invariant("meet-page quarantine counter overflow"))?;
            report.disposition = CollectionDisposition::Partial;
            report
                .unfinished
                .try_reserve(1)
                .map_err(|_| super::jobs::invariant("quarantine locator allocation"))?;
            report.unfinished.push(url.clone());
            report.note(format!("quarantined meet page {url}: {reason}"));
            Ok(report)
        })
}

async fn athleticnet_meets(
    context: &AdapterContext<'_>,
    meets: &[SourceMeetRef],
    jurisdiction: UsJurisdiction,
    at: &str,
) -> Result<(usize, AdapterReport), HandlerError> {
    let ids = athleticnet_meet_ids(meets, jurisdiction)?;
    let meets = ids.len();
    if meets == 0 {
        return Ok((0, AdapterReport::new(ATHLETICNET, "performances")));
    }
    let report = census_crawl::athleticnet::collect(
        context,
        &census_crawl::athleticnet::Options {
            meets: ids,
            observed_on: at.to_string(),
            states: vec![jurisdiction],
            ..census_crawl::athleticnet::Options::default()
        },
    )
    .await
    .map_err(|error| job_error(collect_error(error)))?;
    Ok((meets, report))
}

fn athleticnet_meet_ids(
    rows: &[SourceMeetRef],
    jurisdiction: UsJurisdiction,
) -> Result<Vec<i64>, HandlerError> {
    let mut ids = rows
        .iter()
        .filter(|row| row.jurisdiction == jurisdiction)
        .try_fold(Vec::new(), |mut ids, row| {
            let id = if row.source == ATHLETICNET {
                Some(row.source_meet_id.parse::<i64>().map_err(|_| {
                    super::jobs::invariant(&format!(
                        "invalid Athletic.net meet owner at {}",
                        row.results_url
                    ))
                })?)
            } else {
                meet_id_in(&row.results_url)
            };
            if let Some(id) = id {
                if id <= 0 {
                    return Err(super::jobs::invariant(
                        "Athletic.net meet owner must be positive",
                    ));
                }
                if ids.len() >= 65536 {
                    return Err(super::jobs::invariant(
                        "Athletic.net meet selection capacity",
                    ));
                }
                ids.try_reserve(1).map_err(|_| {
                    super::jobs::invariant("Athletic.net meet selection allocation")
                })?;
                ids.push(id);
            }
            Ok(ids)
        })?;
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

fn meet_id_in(url: &str) -> Option<i64> {
    let host = url.split('/').nth(2)?.to_ascii_lowercase();
    if host != ATHLETICNET_HOST && !host.ends_with(ATHLETICNET_HOST_SUFFIX) {
        return None;
    }
    let mut segments = url.split('/');
    segments.find(|segment| segment.eq_ignore_ascii_case("meet"))?;
    segments.next().and_then(|id| id.parse::<i64>().ok())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod replay_tests;
