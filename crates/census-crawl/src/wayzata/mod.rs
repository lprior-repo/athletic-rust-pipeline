use crate::{AdapterContext, AdapterReport, CrawlResult};
use chrono::Datelike;

mod budget;
mod map;
mod parse;
mod projection;
mod receipt;
mod walk;
pub(super) mod results;
pub use map::{level_of, venue_state};
pub use parse::{schedule_rows, schedule_url, MeetRow, ScheduleSport};

#[cfg(test)]
use serde_json::json;

const PARSE_VERSION: u32 = 4;

const ADAPTER_ID: &str = "wayzata_schedule";

pub const PROVIDER: &str = "wayzata";

pub const BASE: &str = "https://www.wayzataresults.com";

pub struct Options {
    pub years: Vec<i16>,
    pub jurisdictions: Vec<census_domain::UsJurisdiction>,
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: Option<String>,
}

#[tracing::instrument(skip(ctx, options))]
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let before = stats_of(ctx).await;
    let years = calendar_years(ctx, options)?;
    budget::check(
        "Wayzata jurisdictions",
        options.jurisdictions.len(),
        census_domain::UsJurisdiction::ALL.len(),
    )?;
    let walk = walk::Walk::new();
    let walk = walk.run(ctx, options, &years).await?;
    walk.finish(ctx, before).await
}

fn calendar_years(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<Vec<i16>> {
    budget::check(
        "Wayzata calendar years",
        options.years.len(),
        budget::MAX_YEARS,
    )?;
    let mut years = Vec::new();
    budget::reserve(&mut years, options.years.len().max(2), budget::MAX_YEARS)?;
    if options.years.is_empty() {
        let year = i16::try_from(ctx.performance_as_of.year())
            .map_err(|_| budget::arithmetic("Wayzata snapshot calendar year"))?;
        years.push(year);
        years.push(
            year.checked_sub(1)
                .ok_or_else(|| budget::arithmetic("Wayzata previous calendar year"))?,
        );
    } else {
        years.extend_from_slice(&options.years);
    }
    if years.iter().any(|year| !(1..=9999).contains(year)) {
        return Err(crate::CrawlError::Schema {
            url: BASE.to_string(),
            detail: "calendar years must be in 1..=9999".to_string(),
        });
    }
    years.sort_unstable();
    years.dedup();
    Ok(years)
}

async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.physical_requests(), stats.cache_hits)
}

#[cfg(test)]
mod tests;
