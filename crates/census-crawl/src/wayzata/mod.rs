use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::school_index::SchoolIndex;

mod map;
mod parse;
mod walk;

pub use map::{level_of, resolve_venue, venue_candidates, venue_state, VenueResolution};
pub use parse::{schedule_rows, schedule_url, MeetRow, ScheduleSport};
use walk::{completed_pages, Walk};

#[cfg(test)]
use census_domain::model::{CanonicalMeet, SourceIdentity, SourceNamespace};
#[cfg(test)]
use census_store::Table;
#[cfg(test)]
use serde_json::json;

const PARSE_VERSION: u32 = 3;

const ADAPTER_ID: &str = "wayzata_schedule";

pub const PROVIDER: &str = "wayzata";

pub const BASE: &str = "https://www.wayzataresults.com";

pub struct Options {
    pub years: Vec<i16>,
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: Option<String>,
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let observed_on = match options.observed_on.clone() {
        Some(value) => value,
        None => ctx.observed_on.clone(),
    };
    let (requests_before, cache_before) = stats_of(ctx).await;
    let done = completed_pages(ctx)?;
    let years = if options.years.is_empty() {
        default_years(&observed_on)
    } else {
        options.years.clone()
    };

    let schools: Vec<census_domain::model::CanonicalSchool> =
        ctx.store.scan(census_store::Table::Schools)?;
    let index = SchoolIndex::from_schools(&schools);

    let mut walk = Walk::new(observed_on);
    walk.run(ctx, options, &done, &years, &index).await?;
    walk.finish(ctx, (requests_before, cache_before)).await
}

fn default_years(observed_on: &str) -> Vec<i16> {
    let year = observed_on
        .split('-')
        .next()
        .and_then(|year| year.parse::<i16>().ok())
        .map_or(2026, |value| value);
    vec![year, year.saturating_sub(1)]
}

async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.requests, stats.cache_hits)
}

#[cfg(test)]
mod tests;
