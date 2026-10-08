use serde_json::{json, Value};

use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::UsJurisdiction;

mod batches;
mod map;
mod parse;
mod targets;
mod tokens;

use futures::{stream, StreamExt, TryStreamExt};
mod discover;

pub use map::{build_entities, BatchEntities};
pub use parse::{AthleteHit, HitTeam};
pub use targets::{meet_targets, MeetSelection, MeetTarget};
pub use tokens::{gender_from_token, grade_from_token, school_year_for_date, sport_for};

const RESULT_WINDOW: usize = 10_000;
const PAGE_SIZE: usize = 2_000;

const ENDPOINT: &str = "https://search.athletic.live/athlete_list/_search";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

pub fn batch_query(meet_ids: &[u64], from: usize) -> Value {
    json!({
        "size": PAGE_SIZE,
        "from": from,
        "track_total_hits": true,
        "query": { "bool": { "filter": [
            { "terms": { "mi": meet_ids } },
            { "terms": { "y": ["11", "12", "JR", "SR", "Jr", "Sr"] } }
        ] } },
        "_source": ["i", "n", "y", "g", "mi", "ani", "t"]
    })
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let before = ctx.fetcher.stats().await;
    let mut report = AdapterReport::new("athleticlive_athletes", "athletes");
    let targets = discover::discover(ctx, options, &mut report)?;
    let mut report = stream::iter(targets.iter().enumerate())
        .map(Ok::<_, CrawlError>)
        .try_fold(report, |mut report, (ordinal, target)| async move {
            if options.limit.is_some_and(|limit| ordinal >= limit) {
                crate::directory::acquisition::owe(
                    &mut report,
                    format!("{ENDPOINT}#meet={}&from=0", target.athleticlive_meet_id),
                )?;
                return Ok(report);
            }
            batches::run_target(ctx, options, target, report).await
        })
        .await?;
    if report.unfinished.is_empty() {
        report.finish_frontier();
    }
    let after = ctx.fetcher.stats().await;
    report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    Ok(report)
}

#[cfg(test)]
mod tests;
