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
//!   through the consolidated school snapshot via [`resolve_venue`]; anything else is filed under
//!   the `??` state so it can never collide with a real one, and the runner reports how many.
//! * The track schedule mixes indoor and outdoor seasons. A row in November-March is published as
//!   [`Sport::IndoorTrack`](crate::model::Sport::IndoorTrack), everything else as
//!   [`Sport::OutdoorTrack`](crate::model::Sport::OutdoorTrack); the cross-country schedule is
//!   [`Sport::CrossCountry`](crate::model::Sport::CrossCountry).
//! * Dates arrive as a weekday and a day number under a month heading, so a schedule page is read
//!   against the year in its own URL.
//!
//! # Robots
//!
//! `https://www.wayzataresults.com/robots.txt` disallows `/reports/`, `/admin/`, `/action/`,
//! `/cgi-bin/` and the FrontPage `_vti_*` directories for `User-agent: *`, with `Crawl-delay: 10`.
//! The schedule pages sit outside every disallowed path, and the shared fetcher applies the host's
//! ten-second floor to each request.

use crate::model::{CanonicalMeet, Evidence, SourceIdentity, SourceNamespace, SourceRef};
use crate::school_index::SchoolIndex;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::{BTreeMap, HashMap, HashSet};

mod map;
mod parse;

pub use map::{level_of, resolve_venue, venue_candidates, venue_state, VenueResolution};
pub use parse::{schedule_rows, schedule_url, MeetRow, ScheduleSport};

/// Bump when a parse change alters what an already-journaled schedule yields: resume entries are
/// only honoured for the current version.
const PARSE_VERSION: u32 = 3;

/// The adapter's journal namespace and evidence source id.
const ADAPTER_ID: &str = "wayzata_schedule";

/// Provider key space for `TimerMeet` identities.
pub const PROVIDER: &str = "wayzata";

/// Site root.
pub const BASE: &str = "https://www.wayzataresults.com";

/// State code for a row whose venue does not resolve. Distinct from every real state on purpose: an
/// unresolved venue must never be filed under the state it happens to sit next to.
pub const UNKNOWN_STATE: &str = "??";

pub struct Options {
    /// Season years to walk. Empty means the current and previous year, read from the run date.
    pub years: Vec<i16>,
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: Option<String>,
}

#[derive(Debug, Default)]
struct Stats {
    pages: usize,
    rows: usize,
    states_resolved: usize,
    states_from_school: usize,
    states_unknown: usize,
    levels: BTreeMap<String, usize>,
    sports: BTreeMap<String, usize>,
    unresolved_venues: BTreeMap<String, usize>,
}

/// `usize` -> `u64` for the report counters, saturating where the value cannot fit.
fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

/// Walk the provider's published schedules, minting one core meet per competition row.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new(ADAPTER_ID, "meet-schedule rows");
    let observed_on = options
        .observed_on
        .clone()
        .unwrap_or_else(|| ctx.observed_on.clone());
    let (requests_before, cache_before) = stats_of(ctx).await;

    let done: HashSet<String> = ctx
        .store
        .journal_payloads(ADAPTER_ID)?
        .into_iter()
        .filter(|entry| {
            entry.get("parser").and_then(serde_json::Value::as_u64)
                == Some(u64::from(PARSE_VERSION))
        })
        .filter_map(|entry| {
            entry
                .get("url")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
        })
        .collect();

    let years = if options.years.is_empty() {
        default_years(&observed_on)
    } else {
        options.years.clone()
    };

    // The venue table covers the provider's recurring sites; school-shaped venues ("Albany HS") go
    // through the consolidated school snapshot instead, so a meet is filed in the state its host
    // school is in. Before the snapshot exists every such venue stays `??`, and the run says so.
    let schools: Vec<crate::model::CanonicalSchool> =
        ctx.store.scan(crate::store::Table::Schools)?;
    let index = SchoolIndex::from_schools(&schools);
    let mut venue_cache: HashMap<String, VenueResolution> = HashMap::new();

    let mut stats = Stats::default();
    let mut meets: BTreeMap<String, CanonicalMeet> = BTreeMap::new();

    'sport: for sport in [ScheduleSport::Track, ScheduleSport::CrossCountry] {
        for year in &years {
            let url = schedule_url(sport, *year);
            if done.contains(&url) {
                continue;
            }
            if options.limit.is_some_and(|limit| stats.rows >= limit) {
                break 'sport;
            }
            let fetched = ctx
                .fetcher
                .get(&url, &ctx.fetch_options())
                .await
                .with_context(|| format!("fetching the Wayzata Results schedule {url}"))?;
            let rows = schedule_rows(&fetched.text(), *year)?;
            stats.pages = stats.pages.saturating_add(1);
            report.note(format!("{url}: {} competition rows", rows.len()));

            for row in &rows {
                if options.limit.is_some_and(|limit| stats.rows >= limit) {
                    break 'sport;
                }
                stats.rows = stats.rows.saturating_add(1);
                let resolution = resolve_venue(&index, &mut venue_cache, &row.location);
                let state = resolution.state();
                match resolution {
                    VenueResolution::Site(_) => {
                        stats.states_resolved = stats.states_resolved.saturating_add(1);
                    }
                    VenueResolution::School(_) => {
                        stats.states_from_school = stats.states_from_school.saturating_add(1);
                    }
                    VenueResolution::Unknown => {
                        stats.states_unknown = stats.states_unknown.saturating_add(1);
                        let slot = stats
                            .unresolved_venues
                            .entry(row.location.clone())
                            .or_default();
                        *slot = slot.saturating_add(1);
                    }
                }
                let month = row
                    .date
                    .get(5..7)
                    .and_then(|month| month.parse::<u8>().ok())
                    .unwrap_or(0);
                let level = level_of(&row.name);
                let slot = stats.levels.entry(format!("{level:?}")).or_default();
                *slot = slot.saturating_add(1);
                let slot = stats
                    .sports
                    .entry(format!("{:?}", sport.sport_for(month)))
                    .or_default();
                *slot = slot.saturating_add(1);

                let mut meet =
                    CanonicalMeet::new(state.unwrap_or(UNKNOWN_STATE), &row.name, &row.date, level);
                meet.location = Some(row.location.clone());
                meet.sports.push(sport.sport_for(month));
                if let Some(slug) = &row.slug {
                    meet.source_urls.push(format!("{BASE}/links/{slug}"));
                }
                meet.source_urls.push(url.clone());
                meet.source_identities.push(SourceIdentity::new(
                    SourceNamespace::TimerMeet {
                        provider: PROVIDER.to_string(),
                    },
                    row.slug
                        .clone()
                        .unwrap_or_else(|| format!("{}|{}", row.date, meet.normalized_name)),
                ));
                let mut evidence =
                    Evidence::parsed(SourceRef::new(ADAPTER_ID, Some(url.clone())), &observed_on);
                evidence.note = Some(match &row.aria_label {
                    Some(label) => format!("provider schedule row: {label}"),
                    None => format!("provider schedule row at {}", row.location),
                });
                meet.evidence.push(evidence);
                meets.entry(meet.id.as_str().to_string()).or_insert(meet);
            }

            ctx.store.journal_done(
                ADAPTER_ID,
                &url,
                &json!({
                    "url": url,
                    "parser": PARSE_VERSION,
                    "sport": sport.as_str(),
                    "year": year,
                    "rows": rows.len(),
                }),
            )?;
        }
    }

    let meets: Vec<CanonicalMeet> = meets.into_values().collect();
    ctx.store.append_many(Table::Meets, &meets)?;

    let (requests_after, cache_after) = stats_of(ctx).await;
    report.rows = count(stats.rows);
    report.requests = requests_after.saturating_sub(requests_before);
    report.from_cache = cache_after.saturating_sub(cache_before);
    report.note(format!(
        "schedules: {} pages read, {} competition rows, {} core meets minted",
        stats.pages,
        stats.rows,
        meets.len()
    ));
    report.note(format!(
        "venue state resolution: sites={} schools={} unresolved={} ({:?})",
        stats.states_resolved,
        stats.states_from_school,
        stats.states_unknown,
        stats.unresolved_venues
    ));
    report.note(format!("levels: {:?}", stats.levels));
    report.note(format!("sports: {:?}", stats.sports));
    Ok(report)
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
