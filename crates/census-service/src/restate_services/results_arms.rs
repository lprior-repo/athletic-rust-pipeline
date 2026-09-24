/// The results stage's arms: the pull each planned result source runs over the meets this run
/// enumerated.
///
/// Why a fourth stage rather than an arm beside the other two: the team-index and meet-index stages
/// *publish* the universes a run covers, and a result source *consumes* them. An arm that read the
/// meets it should pull would make one stage both publish and consume, and a plan carries one
/// disposition per source, not one per phase of a source's work.
///
/// The seed is the run's own table, never an argument. `source_meets` holds the meets this run's
/// walks enumerated for the jurisdiction (`census::meets` and the meet-index arms), so an arm reads
/// the meets the run itself decided it covers. A stage that took meet ids from anywhere else would be
/// a second opinion about what the run covers, which is exactly what the meet-index stage refuses to
/// be.
///
/// Each arm appends canonical observations through the adapter that owns them (athletes,
/// performances, schools) — not `source_meets` rows — so there is nothing for this caller to route
/// through an `Ingest` object, and the adapter's own journal is the resume point: `milesplit_results`
/// journals one entry per result set under its versioned phase, `athleticnet` journals each meet's
/// request pair. This stage holds no journal of its own, and a re-invocation that reaches it again
/// resumes inside whichever arm it calls.
///
/// The per-source row count is the durable record of what ran, for the reason the meet census states:
/// an arm that found nothing and an arm that never ran would otherwise leave the same trace.
use std::sync::Arc;

use census_crawl::net::Fetcher;
use census_crawl::{AdapterContext, AdapterReport};
use census_domain::model::SourceMeetRef;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use restate_sdk::prelude::{HandlerError, Json};
use serde::{Deserialize, Serialize};

use super::jobs::{adapter_context, collect_error, rows_written};
use super::meets_arms::season_of;
use super::{job_error, JobError};

/// The registry slug of the source whose own rows are keyed by an Athletic.net meet id.
const ATHLETICNET: &str = "athleticnet";

/// The host a row's URL has to name before its `/meet/<id>` segment is read as an Athletic.net meet.
const ATHLETICNET_HOST: &str = "athletic.net";

/// The same host as a subdomain suffix, so a state's own subdomain (`wi.athletic.net`) is accepted
/// without allocating a comparison string per row.
const ATHLETICNET_HOST_SUFFIX: &str = ".athletic.net";

/// MileSplit's host, and the same host as a subdomain suffix: every state site used to be its own
/// host (`oh.milesplit.com`), and both spellings name one family.
const MILESPLIT_HOST: &str = "milesplit.com";
const MILESPLIT_HOST_SUFFIX: &str = ".milesplit.com";

/// The walks the results stage can run, one per planned source.
///
/// The plan asks [`super::jurisdiction::DISPATCHED`] whether any stage sweeps a source; this table,
/// [`super::teams_arms::TEAMS_ARMS`] and [`super::meets_arms::MEETS_ARMS`] are what the chain answers
/// that claim with. `jurisdiction::tests::the_arms_are_the_dispatched_slugs` holds the union of the
/// three to the dispatched list, so a slug planned without an arm fails a test instead of failing a
/// run.
///
/// The slugs are the registry's, because a plan can only select a registered source:
/// `jurisdiction::plan::tests::every_dispatched_slug_is_registered` says so. `milesplit` arms the
/// `/raw` result-set route of the registered MileSplit source, and `athleticnet` the whole-meet pull
/// the census prefers over a per-athlete bio.
pub(super) const RESULTS_ARMS: &[(&str, ResultsArm)] = &[
    ("milesplit", ResultsArm::MilesplitResults),
    (ATHLETICNET, ResultsArm::AthleticnetMeets),
];

/// One arm per pull: what the stage runs for a planned source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ResultsArm {
    /// Every result file the selected meets publish, read whole through the `/raw` route.
    MilesplitResults,
    /// Every selected meet pulled whole: `GetMeetData` then `GetAllResultsData`, the route ADR-004
    /// prefers for a census because one meet yields orders of magnitude more rows per request than a
    /// per-athlete bio.
    AthleticnetMeets,
}

/// The arm for one planned slug, or `None` when another stage owns it.
///
/// `None` is not a source condition: a planned unit a different stage arms is that stage's work, and
/// a unit no stage arms at all is refused by [`super::assert_some_stage_arms`] when the stage that
/// should have run it reaches it.
pub(super) fn arm_for(slug: &str) -> Option<ResultsArm> {
    RESULTS_ARMS
        .iter()
        .find(|(table_slug, _)| *table_slug == slug)
        .map(|(_, arm)| *arm)
}

/// What the results stage produced, per planned source that ran.
///
/// `pub` rather than `pub(crate)` — out of scope, not out of discipline — because it is a field of
/// [`super::wire::JurisdictionState`], and a `pub` field cannot carry a less visible type. The
/// module itself is private to the service, so this stays the service's own shape.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultsStageOutcome {
    pub per_source: Vec<ResultsSourceRows>,
}

/// One source's share of a results stage: the slug the plan named, the meets it selected, and the
/// rows it wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResultsSourceRows {
    pub slug: String,
    pub meets: usize,
    pub rows: usize,
}

/// The results stage: pull the meets this run enumerated, one arm per planned result source.
///
/// Every planned slug another stage arms is skipped rather than refused — the plan lists a source
/// once, and the stages that publish universes run it there. The store scan is over the whole
/// `source_meets` table and the selection is per jurisdiction, because a re-invocation of one state
/// reads the same table a national run filled.
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
    // The selection belongs to the stage rather than to an arm: both arms read the same meets, and
    // selecting once here — moving the scan rather than copying it — is what keeps an arm from
    // re-filtering the whole table on its own.
    let selected = crate::census::select_meets(
        stored,
        &[jurisdiction],
        crate::census::SeasonScope::Year(year),
        None,
    );
    let mut outcome = ResultsStageOutcome::default();
    for slug in &sweepable {
        let Some(arm) = arm_for(slug) else {
            continue;
        };
        let (meets, report) = match arm {
            ResultsArm::MilesplitResults => milesplit_results(&context, &selected).await?,
            ResultsArm::AthleticnetMeets => {
                athleticnet_meets(&context, &selected, jurisdiction, &at).await?
            }
        };
        outcome.per_source.push(ResultsSourceRows {
            slug: slug.clone(),
            meets,
            rows: rows_written(&report)?,
        });
    }
    Ok(Json(outcome))
}

/// Every result file the selected meets publish, read whole through the `/raw` route.
///
/// One page request per meet lists the result files it has, plus one request per file through the
/// `/raw` route. A file the page marks `isMeetPro` is requested like any other: the marker is a fact
/// the page publishes, not a decision this arm makes, so the fetch layer records whatever status the
/// host answers instead of the arm assuming the file is gated.
///
/// The count reported is the number of selected rows that actually name a MileSplit results page.
/// The selection is the run's whole `source_meets` read, and most of those rows name another
/// provider's page — this arm reads its own and reports how many were its own, so an empty run and a
/// run that read nothing look different.
async fn milesplit_results(
    context: &AdapterContext<'_>,
    meets: &[SourceMeetRef],
) -> Result<(usize, AdapterReport), HandlerError> {
    let mut urls: Vec<census_crawl::milesplit::ResultSetRequest> = Vec::new();
    let mut selected = 0_usize;
    for meet in meets
        .iter()
        .filter(|meet| is_results_page(&meet.results_url))
    {
        selected = selected.saturating_add(1);
        let files = census_crawl::milesplit::fetch_meet_result_files(
            context.fetcher,
            &meet.results_url,
            &context.fetch_options(),
        )
        .await
        .map_err(|error| job_error(collect_error(error)))?;
        // Each address carries the jurisdiction of the row that named the meet: a results page that
        // redirects to `www` publishes no state of its own, and this row is the only thing that
        // knows whose meet it is.
        urls.extend(
            files
                .iter()
                .map(|file| census_crawl::milesplit::ResultSetRequest {
                    url: file.raw_url(&meet.results_url),
                    jurisdiction: meet.jurisdiction,
                }),
        );
    }
    let report = census_crawl::milesplit::collect_result_sets(
        context,
        &census_crawl::milesplit::ResultSetOptions { urls },
    )
    .await
    .map_err(|error| job_error(collect_error(error)))?;
    Ok((selected, report))
}

/// Whether a row's URL is a meet's results page on one of MileSplit's state sites.
///
/// A shape check rather than a parse: the page itself carries no provider id, and the `/raw`
/// addresses under it are what the adapter's own `ResultSetRef` validates before a request. What this
/// decides is only whether the page belongs to this arm at all — a WIAA artifact, a Wayzata schedule
/// or an Athletic.net meet is another provider's page and is never handed to the MileSplit reader.
fn is_results_page(url: &str) -> bool {
    let Some(host) = url.split('/').nth(2) else {
        return false;
    };
    let host = host.to_ascii_lowercase();
    if host != MILESPLIT_HOST && !host.ends_with(MILESPLIT_HOST_SUFFIX) {
        return false;
    }
    let named = url
        .split('/')
        .any(|segment| segment.eq_ignore_ascii_case("meets"));
    let last = url
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .is_some_and(|segment| segment.eq_ignore_ascii_case("results"));
    named && last
}

/// The Athletic.net meets this run's own rows name, pulled whole.
///
/// A state whose sources published no Athletic.net meet selects nothing and costs nothing: the arm
/// reports zero meets, which is a fact about the run's coverage, not a failure.
async fn athleticnet_meets(
    context: &AdapterContext<'_>,
    meets: &[SourceMeetRef],
    jurisdiction: UsJurisdiction,
    at: &str,
) -> Result<(usize, AdapterReport), HandlerError> {
    let ids = athleticnet_meet_ids(meets, jurisdiction);
    let meets = ids.len();
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

/// The Athletic.net meet ids this run's rows name, ascending and deduplicated.
///
/// Two row shapes carry one. A row whose source *is* Athletic.net states the id in `source_meet_id`,
/// and a row another source published — `wiaa_results`' "AN export" links, `mshsl`'s schedule rows —
/// names it inside the URL it read. Both are read, and a row that names neither is not this arm's
/// business, so a meet another platform published is never requested from Athletic.net.
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

/// The `/meet/<id>` segment of an Athletic.net URL, or `None` when the URL is not one.
///
/// The host is checked before the path: a timing provider's own URL may carry a `/meet/` segment
/// whose number is that provider's id, and reading it as an Athletic.net id would request a meet that
/// does not exist there. No allocation: the rows this walks are one per enumerated meet.
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
