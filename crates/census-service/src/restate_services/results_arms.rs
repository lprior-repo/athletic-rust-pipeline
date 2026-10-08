use std::sync::Arc;

use census_crawl::net::Fetcher;
use census_crawl::{AdapterContext, AdapterReport, UnresolvedCounters};
use census_domain::model::SourceMeetRef;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use restate_sdk::prelude::{HandlerError, Json};
use serde::{Deserialize, Serialize};

use super::jobs::{adapter_context, assert_some_stage_arms, collect_error, rows_written};
use super::meets_arms::season_of;
use super::{job_error, JobError};

const ATHLETICNET: &str = "athleticnet";

const ATHLETICNET_HOST: &str = "athletic.net";

const ATHLETICNET_HOST_SUFFIX: &str = ".athletic.net";

pub(super) const RESULTS_ARMS: &[(&str, ResultsArm)] = &[
    ("milesplit", ResultsArm::MilesplitResults),
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultsSourceRows {
    pub slug: String,
    pub meets: usize,
    pub rows: usize,
    #[serde(default)]
    pub errors: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unresolved: Option<UnresolvedCounters>,
}

pub(super) async fn results_stage(
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    year: u16,
    refresh: bool,
    at: String,
    sweepable: Vec<String>,
) -> Result<Json<ResultsStageOutcome>, HandlerError> {
    let context = adapter_context(&store, &fetcher, season_of(year)?, refresh, &at, None);
    let stored: Vec<SourceMeetRef> = store
        .scan(Table::SourceMeets)
        .map_err(|error| job_error(JobError::from(error)))?;
    let selected = crate::census::select_meets(
        stored,
        &[jurisdiction],
        crate::census::SeasonScope::Year(year),
        None,
    );
    let mut outcome = ResultsStageOutcome::default();
    for slug in &sweepable {
        let Some(arm) = arm_for(slug) else {
            assert_some_stage_arms(slug)?;
            continue;
        };
        let (meets, report) = match arm {
            ResultsArm::MilesplitResults => milesplit_results(&context, &selected).await?,
            ResultsArm::AthleticnetMeets => {
                athleticnet_meets(&context, &selected, jurisdiction, &at).await?
            }
        };
        outcome.per_source.push(source_rows(slug, meets, &report)?);
    }
    Ok(Json(outcome))
}

fn source_rows(
    slug: &str,
    meets: usize,
    report: &AdapterReport,
) -> Result<ResultsSourceRows, HandlerError> {
    Ok(ResultsSourceRows {
        slug: slug.to_string(),
        meets,
        rows: rows_written(report)?,
        errors: report.errors,
        unresolved: report.unresolved,
    })
}

async fn milesplit_results(
    context: &AdapterContext<'_>,
    meets: &[SourceMeetRef],
) -> Result<(usize, AdapterReport), HandlerError> {
    let selected: Vec<census_crawl::milesplit::MeetPage> = meets
        .iter()
        .filter(|meet| census_crawl::milesplit::is_results_page(&meet.results_url))
        .map(|meet| census_crawl::milesplit::MeetPage {
            results_url: meet.results_url.clone(),
            jurisdiction: meet.jurisdiction,
        })
        .collect();
    let pages = census_crawl::milesplit::read_meet_pages(
        context.fetcher,
        selected.iter().cloned(),
        &context.fetch_options(),
    )
    .await
    .map_err(|error| job_error(collect_error(error)))?;
    let urls = pages.files.iter().map(|file| file.request()).collect();
    let mut report = census_crawl::milesplit::collect_result_sets(
        context,
        &census_crawl::milesplit::ResultSetOptions { urls },
    )
    .await
    .map_err(|error| job_error(collect_error(error)))?;
    for (url, reason) in &pages.quarantined {
        report.note(format!("quarantined meet page {url}: {reason}"));
    }
    report.note(format!("meet_pages_read={}", pages.pages_read));
    Ok((selected.len(), report))
}

async fn athleticnet_meets(
    context: &AdapterContext<'_>,
    meets: &[SourceMeetRef],
    jurisdiction: UsJurisdiction,
    at: &str,
) -> Result<(usize, AdapterReport), HandlerError> {
    let ids = athleticnet_meet_ids(meets, jurisdiction);
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

fn athleticnet_meet_ids(rows: &[SourceMeetRef], jurisdiction: UsJurisdiction) -> Vec<i64> {
    let mut ids: Vec<i64> = rows
        .iter()
        .filter(|row| row.jurisdiction == jurisdiction)
        .filter_map(|row| {
            if row.source == ATHLETICNET {
                return row.source_meet_id.parse::<i64>().ok();
            }
            meet_id_in(&row.results_url)
        })
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
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
