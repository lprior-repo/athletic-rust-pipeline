//! Pulling the listed meets whole: the request pair per meet, the journal that lets a re-run
//! resume, and the run report.
//!
//! The walk itself lives in `walk`, split out when this file passed the repository's file budget:
//! what stays here is the dispatch, the run state and the page flush.

use super::read::EventMetadata;
use super::wire::{AllResults, MeetData};
use super::{meet_requests, metadata_request, MEET_ENDPOINT};
use crate::athleticnet::collect::{
    appended_total, consolidated_index, journaled_urls, stats_of, store_accumulated, EntityCounts,
};
use crate::athleticnet::map::{Accumulator, Stats};
use crate::athleticnet::{Options, PARSE_VERSION};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{SchoolId, SourceRef};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

mod walk;

/// Pull every listed meet whole, in the order the operator listed them.
///
/// Each meet is two requests — `Meet/GetMeetData` and `Meet/GetAllResultsData`, the second
/// authorized with the token the first minted — plus `Meet/GetEventDivisionData` when
/// [`Options::event_metadata`] is set. A meet's rows and the journal entries that name its requests
/// reach the store in one commit, flushed every [`FLUSH_UNITS`] meets, so a re-run resumes at the
/// first meet whose requests are not already journaled at this parse version and no journal entry
/// can name a meet whose rows were never written.
pub(in crate::athleticnet) async fn collect(
    ctx: &AdapterContext<'_>,
    options: &Options,
) -> CrawlResult<AdapterReport> {
    let mut run = MeetRun {
        resolved: HashMap::new(),
        stats: Stats::default(),
        accumulated: Accumulator::default(),
        pending: Vec::new(),
        batches: Vec::new(),
        report: AdapterReport::new("athleticnet", "performances"),
        done: journaled_urls(ctx)?,
    };
    let index = consolidated_index(ctx)?;
    let source = SourceRef::new("athleticnet", Some(MEET_ENDPOINT.to_string()));
    let (requests_before, cache_before) = stats_of(ctx).await;
    let rows = run.pull(ctx, options, &source, &index).await?;
    let (requests_after, cache_after) = stats_of(ctx).await;
    run.report.rows = rows;
    run.report.requests = requests_after.saturating_sub(requests_before);
    run.report.from_cache = cache_after.saturating_sub(cache_before);
    run.report.errors = run.stats.fetches_failed;
    run.report.note(format!(
        "canonical entities: schools {} meets {} teams {} athletes {} events {} performances {}",
        appended_total(&run.batches, |batch| batch.schools),
        appended_total(&run.batches, |batch| batch.meets),
        appended_total(&run.batches, |batch| batch.teams),
        appended_total(&run.batches, |batch| batch.athletes),
        appended_total(&run.batches, |batch| batch.events),
        appended_total(&run.batches, |batch| batch.performances),
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
    /// Meets absorbed since the last flush: the URL to journal and the payload to journal it with.
    pending: Vec<(String, Value)>,
    /// What each flush appended; the report's entity note sums them.
    batches: Vec<EntityCounts>,
    report: AdapterReport,
    /// The URLs an earlier run already journaled at the current parse version.
    done: HashSet<String>,
}

impl MeetRun {
    /// Append the page's rows, then journal its meets: one commit, so a run that stops between two
    /// pages can neither skip a meet whose rows are missing nor hold rows for a meet it re-reads.
    fn flush(&mut self, ctx: &AdapterContext<'_>) -> CrawlResult<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let mut page = ctx.store.write_batch();
        let batch = store_accumulated(ctx, std::mem::take(&mut self.accumulated), &mut page)?;
        self.batches.push(batch);
        for (url, payload) in self.pending.drain(..) {
            page.journal_done("athleticnet", &url, &payload)?;
        }
        page.commit()?;
        Ok(())
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

    /// Queue every URL this meet's pull spent, for the page's journal commit.
    fn journal(&self, rows: u64, pending: &mut Vec<(String, Value)>) {
        pending.push(journal_entry(&self.meet, self.meet_id, 0));
        pending.push(journal_entry(&self.results, self.meet_id, rows));
    }
}

/// The documents of one meet, once all of them have been read.
struct Documents {
    meet: MeetData,
    results: AllResults,
    metadata: Option<EventMetadata>,
}

/// One spent request's journal entry, in the shape the bio path journals its own: the caller holds
/// it until the page naming the request's rows is ready to commit.
fn journal_entry(url: &str, meet_id: i64, rows: u64) -> (String, Value) {
    (
        url.to_string(),
        json!({
            "url": url,
            "parser": PARSE_VERSION,
            "parsed": true,
            "meet": meet_id,
            "rows": rows,
        }),
    )
}
