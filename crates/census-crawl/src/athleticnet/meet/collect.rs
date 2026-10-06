use super::read::EventMetadata;
use super::wire::{AllResults, MeetData};
use super::{meet_requests, metadata_request, MEET_ENDPOINT};
use crate::athleticnet::collect::{
    appended_total, journaled_urls, live_index, stats_of, store_accumulated, EntityCounts,
};
use crate::athleticnet::map::{Accumulator, Stats};
use crate::athleticnet::{Options, PARSE_VERSION};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{SchoolId, SourceRef};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

mod walk;

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
    let index = live_index(ctx)?;
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
    run.report.note(format!(
        "unsupported graduation inference: {} raw grade/year observations retained for review",
        appended_total(&run.batches, |batch| batch.unsupported_cohorts),
    ));
    run.report.note(
        "this source is outside the core scope (`report --core`): it is a reseller of results the \
         platform also gathers from governing bodies and timers, so the core comparison stays \
         independent of it",
    );
    Ok(run.report)
}

struct MeetRun {
    resolved: HashMap<String, SchoolId>,
    stats: Stats,
    accumulated: Accumulator,
    pending: Vec<(String, Value)>,
    batches: Vec<EntityCounts>,
    report: AdapterReport,
    done: HashSet<String>,
}

impl MeetRun {
    fn flush(&mut self, ctx: &AdapterContext<'_>) -> CrawlResult<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let mut page = ctx.write_batch();
        let batch = store_accumulated(ctx, std::mem::take(&mut self.accumulated), &mut page)?;
        self.batches.push(batch);
        for (url, payload) in self.pending.drain(..) {
            page.journal_done("athleticnet", &url, &payload)?;
        }
        page.commit()?;
        Ok(())
    }
}

struct MeetUrls {
    meet_id: i64,
    meet: String,
    results: String,
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

    fn journaled(&self, done: &HashSet<String>) -> bool {
        done.contains(&self.meet)
            && done.contains(&self.results)
            && self.metadata.as_ref().is_none_or(|url| done.contains(url))
    }

    fn journal(&self, rows: u64, pending: &mut Vec<(String, Value)>) {
        pending.push(journal_entry(&self.meet, self.meet_id, 0));
        pending.push(journal_entry(&self.results, self.meet_id, rows));
    }
}

struct Documents {
    meet: MeetData,
    results: AllResults,
    metadata: Option<EventMetadata>,
}

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
