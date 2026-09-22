//! Wayzata Results — a Minnesota / Iowa / Wisconsin timing provider (Tier E).
//!
//! The provider publishes one server-rendered schedule table per sport and season at
//! `https://www.wayzataresults.com/sports/{track|xc}/{year}/schedule`, with one row per competition
//! day:
//!
//! ```text
//! <tr class="event-row away">
//!   <td class="date font-weight-normal"> Sun. 4</td>
//!   <td class="team awayteam"><span title="USATF Minnesota All-Comers Meet #3">…</span></td>
//!   <td class="team hometeam"><span title="University of Minnesota">…</span></td>
//!   <td class="links"><a href="/links/7vqvs7" aria-label="track event: January 4 12:00 AM: …">…</a></td>
//! </tr>
//! ```
//!
//! That table is the provider's own publishing, so a meet listed there is **core** evidence: no
//! Athletic.net surface is involved and none is requested. Meets are minted on the platform's
//! meet identity (`state + date + normalized name`) so a meet this adapter lists reconciles with the
//! same meet arriving from an association artifact.
//!
//! # What this adapter deliberately does not do
//!
//! The `Links` column points at `/links/<slug>` pages that render the provider's live-results
//! partner, AthleticLIVE. The slugs are retained as `TimerMeet` provider keys and their URLs as
//! `source_urls` for later non-core enrichment, but this adapter never fetches them: a core entity
//! has to be reachable with Athletic.net and its mirror switched off, and the schedule row alone is
//! enough to state that a meet exists, where, and when. Result rows for these meets come from
//! official association artifacts instead (see [`super::wiaa_results`]).
//!
//! # Coverage limits, stated rather than hidden
//!
//! * The venue is a free-text `Location` cell. A venue that names one of the provider's recurring
//!   sites resolves to a state through [`venue_state`]; a school-shaped venue (`"Albany HS"`) goes
//!   through the consolidated school snapshot via [`resolve_venue`]; anything else mints an
//!   unplaced meet (`state` absent, spelled `??` only in reports) so an unresolved venue can never
//!   be attributed to the state the provider happens to sit next to, and the runner counts them.
//! * The track schedule mixes indoor and outdoor seasons. A row in November-March is published as
//!   [`Sport::IndoorTrack`](census_domain::model::Sport::IndoorTrack), everything else as
//!   [`Sport::OutdoorTrack`](census_domain::model::Sport::OutdoorTrack); the cross-country schedule is
//!   [`Sport::CrossCountry`](census_domain::model::Sport::CrossCountry).
//! * Dates arrive as a weekday and a day number under a month heading, so a schedule page is read
//!   against the year in its own URL.
//!
//! # Robots
//!
//! `https://www.wayzataresults.com/robots.txt` disallows `/reports/`, `/admin/`, `/action/`,
//! `/cgi-bin/` and the FrontPage `_vti_*` directories for `User-agent: *`, with `Crawl-delay: 10`.
//! The schedule pages sit outside every disallowed path, and the shared fetcher applies the host's
//! ten-second floor to each request.

use crate::school_index::SchoolIndex;
use crate::sources::{AdapterContext, AdapterReport, CrawlResult};

mod map;
mod parse;
mod walk;

pub use map::{level_of, resolve_venue, venue_candidates, venue_state, VenueResolution};
pub use parse::{schedule_rows, schedule_url, MeetRow, ScheduleSport};
use walk::{completed_pages, Walk};

// The module's test file reads these through `use super::*`; the walk itself imports them.
#[cfg(test)]
use crate::store::Table;
#[cfg(test)]
use census_domain::model::{CanonicalMeet, SourceIdentity, SourceNamespace};
#[cfg(test)]
use serde_json::json;

/// Bump when a parse change alters what an already-journaled schedule yields: resume entries are
/// only honoured for the current version.
const PARSE_VERSION: u32 = 3;

/// The adapter's journal namespace and evidence source id.
const ADAPTER_ID: &str = "wayzata_schedule";

/// Provider key space for `TimerMeet` identities.
pub const PROVIDER: &str = "wayzata";

/// Site root.
pub const BASE: &str = "https://www.wayzataresults.com";

pub struct Options {
    /// Season years to walk. Empty means the current and previous year, read from the run date.
    pub years: Vec<i16>,
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: Option<String>,
}

/// Walk the provider's published schedules, minting one core meet per competition row.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let observed_on = options
        .observed_on
        .clone()
        .unwrap_or_else(|| ctx.observed_on.clone());
    let (requests_before, cache_before) = stats_of(ctx).await;
    let done = completed_pages(ctx)?;
    let years = if options.years.is_empty() {
        default_years(&observed_on)
    } else {
        options.years.clone()
    };

    // The venue table covers the provider's recurring sites; school-shaped venues ("Albany HS") go
    // through the consolidated school snapshot instead, so a meet is filed in the state its host
    // school is in. Before the snapshot exists every such venue stays unplaced, and the run says so.
    let schools: Vec<census_domain::model::CanonicalSchool> =
        ctx.store.scan(crate::store::Table::Schools)?;
    let index = SchoolIndex::from_schools(&schools);

    let mut walk = Walk::new(observed_on);
    walk.run(ctx, options, &done, &years, &index).await?;
    walk.finish(ctx, (requests_before, cache_before)).await
}

/// The current and previous season year, read from the run's observation date.
fn default_years(observed_on: &str) -> Vec<i16> {
    let year = observed_on
        .split('-')
        .next()
        .and_then(|year| year.parse::<i16>().ok())
        .unwrap_or(2026);
    // The season before it: a four-digit year never reaches the saturation bound.
    vec![year, year.saturating_sub(1)]
}

async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.requests, stats.cache_hits)
}

// -------------------------------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests;
