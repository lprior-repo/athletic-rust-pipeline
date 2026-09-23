//! TFRRS HTML adapter — the per-state performance lists and team rosters.
//!
//! TFRRS (tfrrs.org, DirectAthletics' results host) publishes one *instance per state*
//! (`indiana.tfrrs.org`, `nh.tfrrs.org`, `florida.tfrrs.org`, …), each serving the same
//! server-rendered markup. Two shapes are read, one request each, no pagination:
//!
//! | route | unit | requests | capture |
//! |---|---|---|---|
//! | `/lists/<id>/<slug>/<year>/<i\|o>`, optionally `?year=<TOKEN>` | one list *view* | 1 | `indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i`, 1,880,743 B, HTTP 200 (`tests/fixtures/tfrrs/`) |
//! | `/teams/<tf\|xc>/<slug>_<m\|f>.html` | one team season | 1 | `nh.tfrrs.org/teams/xc/Pembroke_Academy_m.html`, 64,461 B (`tests/fixtures/tfrrs/`) |
//!
//! A list page is the host's own ranking view: rows are `div.performance-list-row` chunks whose
//! cells are addressed by `data-label`, so a column the host adds later cannot shift a value. A
//! team page publishes its `ROSTER` table (`NAME`/`YEAR` per athlete) and the season its own
//! `config_hnd` control states.
//!
//! # What this adapter refuses to guess
//!
//! * **The state.** TFRRS's instances are per-state and the markup names no state: the page's own
//!   host is the jurisdiction (`parse::jurisdiction_of_url`). A page on a host that names no state
//!   (`www`, the national/sport instances) is refused rather than minted, because school identity
//!   keys on state + name and `Central` in two states is two schools.
//! * **A relay's athletes.** A relay row prints its members' surnames only, with no class year
//!   anywhere in the row: a surname is not an identity, so relay rows are counted, never minted.
//! * **A grade below high school.** `8`, `7` and `6` parse (the site's own filter offers them) but
//!   no canonical grade exists below 9, so such a row is counted rather than widened into one.
//! * **A mark in no shape the reader can place.** A field mark with no `Conv` column and no
//!   feet–inches notation stays `Mark::Raw` and is counted, so nothing is invented.
//! * **A season the page does not state.** A path without `<year>/<i|o>` states no season, and a
//!   list row whose date is absent cannot be dated: both skip with a counter.
//!
//! # Requests
//!
//! One request per supplied URL, spaced by the fetcher's own per-host gate (the crate's default
//! is one request per second per host), journaled per URL under the private `PHASE` key so a re-run
//! resumes instead of re-reading. Nothing here discovers URLs: the operator supplies each list or
//! team page, which is what keeps the crawl to the pages an operator asked for.
//!
//! # Layout
//!
//! `parse` reads the published markup into rows, `map` turns those rows into canonical entities,
//! `run` walks the supplied URLs (fetch → parse → absorb → journal), and `report` writes the
//! run's notes. `classify` and [`Options`] are the way in.

use crate::sources::{CrawlError, CrawlResult};

mod map;
mod parse;
mod report;
mod run;

#[cfg(test)]
mod tests;

// The page readers `xtask replay` drives over the committed captures. The module's own tests call
// the same two functions on the same fixtures; the route types below stay exported for the walk.
pub use parse::{
    parse_list_page, parse_list_path, parse_team_page, parse_team_path, ListPath, TeamPath,
};

use crate::school_index::SchoolIndex;
use crate::sources::{AdapterContext, AdapterReport};
use census_domain::model::CanonicalSchool;
use census_domain::UsJurisdiction;

use run::Run;

/// The adapter's name in the journal and the report.
const ADAPTER: &str = "tfrrs";

/// The journal phase of the page walk; the version is in the name, so a parser change that alters
/// what an already-journaled page yields bumps it and those pages are read again.
const PHASE: &str = "tfrrs_pages_v1";

/// Adapter options.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Page URLs to read, in the order supplied: a performance list
    /// (`/lists/<id>/<slug>/<year>/<i|o>`, optionally `?year=<TOKEN>`) or a team page
    /// (`/teams/<tf|xc>/<slug>_<m|f>.html`). Each URL is one request.
    pub urls: Vec<String>,
    /// Cap the number of pages processed (smoke runs).
    pub limit: Option<usize>,
    /// ISO date stamped into evidence.
    pub observed_on: String,
}

/// The page shape a URL names, with the route it states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    List(ListPath),
    Team(TeamPath),
}

/// Read the route a page URL names, either spelling the host publishes (the relative path its own
/// navigation prints, the absolute URL its links carry).
pub fn classify(url: &str) -> Option<Route> {
    parse_list_path(url)
        .map(Route::List)
        .or_else(|| parse_team_path(url).map(Route::Team))
}

/// The source id of one state's instance, the way every other adapter names a per-state source
/// (`milesplit_oh`): `tfrrs_in`, `tfrrs_nh`.
fn source_id(state: UsJurisdiction) -> String {
    format!("tfrrs_{}", state.code().to_ascii_lowercase())
}

/// Read every supplied page into the canonical store.
///
/// Strategy: one request per URL, in the order supplied, journaled per URL so a re-run resumes. The
/// append happens before the journal, so a page the journal calls done is a page whose rows the
/// store already holds. The pages are read sequentially because the fetcher's per-host gate admits
/// one request per host at a time anyway, and the order is what makes the journal and the report
/// reproducible.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new(ADAPTER, "rows");
    if options.urls.is_empty() {
        report.note("no page URLs supplied; nothing was requested".to_string());
        return Ok(report);
    }
    let (requests_before, cache_before) = stats_of(ctx).await;
    let index = SchoolIndex::from_schools(&consolidated_schools(ctx)?);
    let mut run = Run::new(&index, ctx.store.journal_keys(PHASE)?);
    for url in options.urls.iter().take(limit_of(options)) {
        run.read(ctx, url).await;
    }
    let counts = run.append(ctx)?;
    run.journal(ctx);
    let (requests_after, cache_after) = stats_of(ctx).await;
    report.rows = run
        .absorb
        .stats
        .rows_absorbed
        .saturating_add(run.absorb.stats.roster_rows_absorbed);
    report.requests = requests_after.saturating_sub(requests_before);
    report.from_cache = cache_after.saturating_sub(cache_before);
    report.errors = u64::try_from(run.failures.len()).map_err(|_| CrawlError::Arithmetic {
        detail: "failure count does not fit in u64".to_string(),
    })?;
    report::note_pages(&mut report, &run);
    report::note_rows(&mut report, &run.absorb.stats);
    report::note_rosters(&mut report, &run.absorb.stats);
    report::note_resolution(&mut report, &run.absorb.stats);
    report::note_entities(&mut report, &counts);
    report::note_failures(&mut report, &run.failures);
    Ok(report)
}

/// The number of URLs a run is capped to.
fn limit_of(options: &Options) -> usize {
    options.limit.unwrap_or(usize::MAX)
}

/// The fetcher's request and cache counters, sampled before and after the walk.
async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.requests, stats.cache_hits)
}

/// The consolidated schools this adapter resolves names against. A miss mints instead of skipping
/// (TFRRS spans the whole country while the index covers the platform's states), so an absent file
/// is an empty index rather than an error.
fn consolidated_schools(ctx: &AdapterContext<'_>) -> CrawlResult<Vec<CanonicalSchool>> {
    let path = ctx.store.out_dir().join("schools.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let schools: Vec<CanonicalSchool> = crate::store::read::read_rows(&path)?;
    Ok(schools)
}
