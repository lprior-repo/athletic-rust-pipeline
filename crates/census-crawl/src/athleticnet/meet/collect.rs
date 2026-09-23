//! Pulling the listed meets whole: the request pair per meet, the journal that lets a re-run
//! resume, and the run report.

use super::count::{note, MeetStats};
use super::read::EventMetadata;
use super::wire::{AllResults, EventDivisions, MeetData};
use super::{meet_requests, metadata_request, MEET_ENDPOINT};
use crate::athleticnet::collect::{
    consolidated_index, journaled_urls, stats_of, store_accumulated,
};
use crate::athleticnet::map::{Accumulator, Stats};
use crate::athleticnet::{Options, PARSE_VERSION};
use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{SchoolId, SourceRef};
use census_domain::school_index::SchoolIndex;
use serde_json::json;
use std::collections::{HashMap, HashSet};

/// Pull every listed meet whole, in the order the operator listed them.
///
/// Each meet is two requests — `Meet/GetMeetData` and `Meet/GetAllResultsData`, the second
/// authorized with the token the first minted — plus `Meet/GetEventDivisionData` when
/// [`Options::event_metadata`] is set. Every request is journaled as it lands, so a re-run resumes
/// at the first meet whose requests are not already journaled at this parse version.
pub(in crate::athleticnet) async fn collect(
    ctx: &AdapterContext<'_>,
    options: &Options,
) -> CrawlResult<AdapterReport> {
    let mut run = MeetRun {
        resolved: HashMap::new(),
        stats: Stats::default(),
        accumulated: Accumulator::default(),
        report: AdapterReport::new("athleticnet", "performances"),
        done: journaled_urls(ctx)?,
    };
    let index = consolidated_index(ctx)?;
    let source = SourceRef::new("athleticnet", Some(MEET_ENDPOINT.to_string()));
    let (requests_before, cache_before) = stats_of(ctx).await;
    let rows = run.pull(ctx, options, &source, &index).await?;
    let counts = store_accumulated(ctx, run.accumulated)?;
    let (requests_after, cache_after) = stats_of(ctx).await;
    run.report.rows = rows;
    run.report.requests = requests_after.saturating_sub(requests_before);
    run.report.from_cache = cache_after.saturating_sub(cache_before);
    run.report.errors = run.stats.fetches_failed;
    run.report.note(format!(
        "canonical entities: schools {} meets {} teams {} athletes {} events {} performances {}",
        counts.schools,
        counts.meets,
        counts.teams,
        counts.athletes,
        counts.events,
        counts.performances
    ));
    run.report.note(
        "this source is outside the core scope (`report --core`): it is a reseller of results the \
         platform also gathers from governing bodies and timers, so the core comparison stays \
         independent of it",
    );
    Ok(run.report)
}

/// One meet run's sinks.
struct MeetRun {
    resolved: HashMap<String, SchoolId>,
    stats: Stats,
    accumulated: Accumulator,
    report: AdapterReport,
    /// The URLs an earlier run already journaled at the current parse version.
    done: HashSet<String>,
}

impl MeetRun {
    /// The meets, one request pair at a time.
    async fn pull(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        source: &SourceRef,
        index: &SchoolIndex,
    ) -> CrawlResult<u64> {
        let mut rows: u64 = 0;
        let mut totals = MeetStats::default();
        for (processed, meet_id) in options.meets.iter().enumerate() {
            if options.meet_limit.is_some_and(|limit| processed >= limit) {
                break;
            }
            let urls = MeetUrls::new(*meet_id, options.event_metadata);
            if urls.journaled(&self.done) {
                continue;
            }
            let Some(documents) = self.documents(ctx, options, &urls).await? else {
                continue;
            };
            let (stored, counts) = self.absorb(&documents, source, index, &options.observed_on);
            rows = rows.saturating_add(stored);
            totals.merge(&counts);
            urls.journal(ctx, stored)?;
        }
        note(&mut self.report, &totals, options.event_metadata);
        Ok(rows)
    }

    /// The documents of one meet, or `None` when one of them could not be read.
    async fn documents(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        urls: &MeetUrls,
    ) -> CrawlResult<Option<Documents>> {
        let Some(meet) = self.fetch::<MeetData>(ctx, options, &urls.meet, None).await else {
            return Ok(None);
        };
        let token = meet.token.clone();
        let Some(results) = self
            .fetch::<AllResults>(ctx, options, &urls.results, token.as_deref())
            .await
        else {
            return Ok(None);
        };
        let metadata = self.metadata(ctx, options, urls, token.as_deref()).await?;
        Ok(Some(Documents {
            meet,
            results,
            metadata,
        }))
    }

    /// The third request's decoded document, or `None` when it was not spent or could not be read.
    async fn metadata(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        urls: &MeetUrls,
        token: Option<&str>,
    ) -> CrawlResult<Option<EventMetadata>> {
        let Some(url) = urls.metadata.as_deref() else {
            return Ok(None);
        };
        let Some(document) = self.fetch::<EventDivisions>(ctx, options, url, token).await else {
            return Ok(None);
        };
        journal(ctx, url, urls.meet_id, 0)?;
        Ok(Some(EventMetadata::new(&document)))
    }

    /// Walk one meet's documents into the accumulator, and narrate what the walk counted.
    fn absorb(
        &mut self,
        documents: &Documents,
        source: &SourceRef,
        index: &SchoolIndex,
        observed_on: &str,
    ) -> (u64, MeetStats) {
        let (stored, counts) = super::absorb_meet(
            &documents.meet,
            &documents.results,
            documents.metadata.as_ref(),
            source,
            observed_on,
            index,
            &mut self.resolved,
            &mut self.stats,
            &mut self.accumulated,
        );
        self.report
            .note(format!("meet {}: {counts}", documents.meet.meet.id));
        if let Some(metadata) = documents.metadata.as_ref() {
            self.report.note(format!(
                "meet {} metadata: {} events declared, {} of them field events, {} hurdle races",
                documents.meet.meet.id,
                metadata.len(),
                metadata.field_events(),
                metadata.hurdles()
            ));
        }
        (stored, counts)
    }

    /// Fetch and decode one document, or `None` when it could not be read.
    async fn fetch<T: serde::de::DeserializeOwned>(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        url: &str,
        token: Option<&str>,
    ) -> Option<T> {
        let mut headers = vec![("Accept".to_string(), "application/json".to_string())];
        if let Some(token) = token {
            headers.push(("anettokens".to_string(), token.to_string()));
        }
        let fetch_options = FetchOptions {
            refresh: options.refresh,
            allow_not_found: false,
            headers,
        };
        let fetched = match ctx.fetcher.get(url, &fetch_options).await {
            Ok(fetched) => fetched,
            Err(error) => {
                self.stats.fetches_failed = self.stats.fetches_failed.saturating_add(1);
                self.report.note(format!("{url}: {error}"));
                return None;
            }
        };
        match serde_json::from_str(&fetched.text()) {
            Ok(document) => Some(document),
            Err(error) => {
                self.stats.fetches_failed = self.stats.fetches_failed.saturating_add(1);
                self.report.note(format!(
                    "{url}: body is not the expected document ({error})"
                ));
                None
            }
        }
    }
}

/// The URLs one meet's pull spends.
struct MeetUrls {
    meet_id: i64,
    meet: String,
    results: String,
    /// The third request, when the run asked for the per-event metadata.
    metadata: Option<String>,
}

impl MeetUrls {
    fn new(meet_id: i64, event_metadata: bool) -> Self {
        let [meet, results] = meet_requests(meet_id);
        Self {
            meet_id,
            meet,
            results,
            metadata: event_metadata.then(|| metadata_request(meet_id)),
        }
    }

    /// Whether an earlier run already journaled every URL this pull would spend at this parse
    /// version. The metadata URL counts too: asking for it after a run that did not must not read
    /// as already-pulled.
    fn journaled(&self, done: &HashSet<String>) -> bool {
        done.contains(&self.meet)
            && done.contains(&self.results)
            && self.metadata.as_ref().is_none_or(|url| done.contains(url))
    }

    /// Journal every URL this meet's pull spent.
    fn journal(&self, ctx: &AdapterContext<'_>, rows: u64) -> CrawlResult<()> {
        journal(ctx, &self.meet, self.meet_id, 0)?;
        journal(ctx, &self.results, self.meet_id, rows)?;
        Ok(())
    }
}

/// The documents of one meet, once all of them have been read.
struct Documents {
    meet: MeetData,
    results: AllResults,
    metadata: Option<EventMetadata>,
}

/// Journal one spent request, in the shape the bio path journals its own.
fn journal(ctx: &AdapterContext<'_>, url: &str, meet_id: i64, rows: u64) -> CrawlResult<()> {
    ctx.store.journal_done(
        "athleticnet",
        url,
        &json!({
            "url": url,
            "parser": PARSE_VERSION,
            "parsed": true,
            "meet": meet_id,
            "rows": rows,
        }),
    )?;
    Ok(())
}
