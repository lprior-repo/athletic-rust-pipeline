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
//!   [`Sport::IndoorTrack`], everything else as [`Sport::OutdoorTrack`]; the cross-country schedule
//!   is [`Sport::CrossCountry`].
//! * Dates arrive as a weekday and a day number under a month heading, so a schedule page is read
//!   against the year in its own URL.
//!
//! # Robots
//!
//! `https://www.wayzataresults.com/robots.txt` disallows `/reports/`, `/admin/`, `/action/`,
//! `/cgi-bin/` and the FrontPage `_vti_*` directories for `User-agent: *`, with `Crawl-delay: 10`.
//! The schedule pages sit outside every disallowed path, and the shared fetcher applies the host's
//! ten-second floor to each request.

use crate::model::{
    CanonicalMeet, CompetitionLevel, Evidence, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use crate::school_index::SchoolIndex;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};
use regex::Regex;
use serde_json::json;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

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

/// The two schedules this adapter walks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleSport {
    Track,
    CrossCountry,
}

impl ScheduleSport {
    fn path(self) -> &'static str {
        match self {
            ScheduleSport::Track => "track",
            ScheduleSport::CrossCountry => "xc",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            ScheduleSport::Track => "track",
            ScheduleSport::CrossCountry => "xc",
        }
    }

    /// Which sport a row of this schedule belongs to, from its month alone.
    fn sport_for(self, month: u8) -> Sport {
        match self {
            ScheduleSport::CrossCountry => Sport::CrossCountry,
            // The published "track" schedule opens in the indoor season and runs into the outdoor
            // one; the month is the only field a row carries that separates them.
            ScheduleSport::Track if month <= 3 || month >= 11 => Sport::IndoorTrack,
            ScheduleSport::Track => Sport::OutdoorTrack,
        }
    }
}

/// Schedule page for one sport and season year.
pub fn schedule_url(sport: ScheduleSport, year: i16) -> String {
    format!("{BASE}/sports/{}/{year}/schedule", sport.path())
}

pub struct Options {
    /// Season years to walk. Empty means the current and previous year, read from the run date.
    pub years: Vec<i16>,
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: Option<String>,
}

/// One row of a schedule table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetRow {
    /// `YYYY-MM-DD`, from the row's own date cell and the month heading above it.
    pub date: String,
    pub name: String,
    pub location: String,
    /// Provider key: the last segment of the row's `/links/<slug>` target, when it has one.
    pub slug: Option<String>,
    /// The link's own `aria-label`, which repeats month, day, name and venue.
    pub aria_label: Option<String>,
}

static ROW: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<tr\b[^>]*>.*?</tr>"));
static DATE_CELL: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<td[^>]*class="[^"]*date[^"]*"[^>]*>(.*?)</td>"#));
static NAME_CELL: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<td[^>]*class="[^"]*awayteam[^"]*"[^>]*>(.*?)</td>"#));
static VENUE_CELL: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<td[^>]*class="[^"]*hometeam[^"]*"[^>]*>(.*?)</td>"#));
static TITLE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<span[^>]*title="([^"]*)""#));
static LINK: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<a[^>]*href="/links/([^"/?#]+)""#));
static ARIA: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<a[^>]*aria-label="([^"]*)""#));
static TAGS: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"(?is)<[^>]*>"));
static DAY: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"(\d{1,2})"));

/// The compiled pattern in one slot above, or the compile error it carries.
///
/// Every pattern here is a literal, so a failure is a programming mistake rather than something a
/// page can cause; it is reported as an error instead of panicking at first use.
fn compiled(slot: &'static LazyLock<Result<Regex, regex::Error>>) -> Result<&'static Regex> {
    slot.as_ref().map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn row() -> Result<&'static Regex> {
    compiled(&ROW)
}

fn date_cell() -> Result<&'static Regex> {
    compiled(&DATE_CELL)
}

fn name_cell() -> Result<&'static Regex> {
    compiled(&NAME_CELL)
}

fn venue_cell() -> Result<&'static Regex> {
    compiled(&VENUE_CELL)
}

fn title() -> Result<&'static Regex> {
    compiled(&TITLE)
}

fn link() -> Result<&'static Regex> {
    compiled(&LINK)
}

fn aria() -> Result<&'static Regex> {
    compiled(&ARIA)
}

fn tags() -> Result<&'static Regex> {
    compiled(&TAGS)
}

fn day() -> Result<&'static Regex> {
    compiled(&DAY)
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Read every competition row from one schedule page.
///
/// Rows are published under month headings; the heading is the only month a row carries, so it is
/// tracked as the table is walked. A row without a date, a name or a venue is skipped: the platform
/// cannot mint an identity for it.
pub fn schedule_rows(body: &str, year: i16) -> Result<Vec<MeetRow>> {
    let mut rows = Vec::new();
    let mut month: Option<u8> = None;
    let day_pattern = day()?;
    for found in row()?.find_iter(body) {
        let row = found.as_str();
        // The month heading is a row of its own (`<tr class="month-title …">`) whose text is the
        // month name; every competition row beneath it belongs to that month until the next heading.
        if row.contains("month-title") {
            month = month_from_text(&tags()?.replace_all(row, " "));
            continue;
        }
        if !row.contains("event-row") {
            continue;
        }
        let Some(month) = month else {
            continue;
        };
        let Some(day) = date_cell()?
            .captures(row)
            .and_then(|cell| cell.get(1))
            .and_then(|cell| day_pattern.captures(cell.as_str()))
            .and_then(|day| day.get(1))
            .and_then(|day| day.as_str().parse::<u8>().ok())
        else {
            continue;
        };
        let name = cell_text(name_cell()?, row)?;
        let location = cell_text(venue_cell()?, row)?;
        if name.is_empty() || location.is_empty() {
            continue;
        }
        rows.push(MeetRow {
            date: format!("{year:04}-{month:02}-{day:02}"),
            name,
            location,
            slug: link()?
                .captures(row)
                .and_then(|capture| capture.get(1))
                .map(|slug| slug.as_str().to_string())
                .filter(|slug| !slug.is_empty()),
            aria_label: aria()?
                .captures(row)
                .and_then(|capture| capture.get(1))
                .map(|label| normalize_whitespace(label.as_str()))
                .filter(|label| !label.is_empty()),
        });
    }
    Ok(rows)
}

/// A table cell's label: its `<span title="…">` when it has one, otherwise its stripped text.
fn cell_text(cell: &Regex, row: &str) -> Result<String> {
    let Some(captures) = cell.captures(row) else {
        return Ok(String::new());
    };
    let inner = captures
        .get(1)
        .map(|cell| cell.as_str())
        .unwrap_or_default();
    if let Some(title) = title()?.captures(inner).and_then(|title| title.get(1)) {
        return Ok(normalize_whitespace(title.as_str()));
    }
    Ok(normalize_whitespace(&tags()?.replace_all(inner, " ")))
}

fn month_from_text(text: &str) -> Option<u8> {
    let text = text.trim().to_ascii_lowercase();
    MONTHS
        .iter()
        .position(|month| text.starts_with(&month.to_ascii_lowercase()))
        .and_then(|index| index.checked_add(1))
        // `MONTHS` holds twelve names, so the 1-based month always fits a `u8`.
        .and_then(|month| u8::try_from(month).ok())
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Venue markers that identify one of the three states the provider times in.
///
/// Only unambiguous markers are listed: a venue that is not recognised stays unresolved rather than
/// being attributed to the state the provider happens to be based in, and ambiguous names
/// ("Augustana College" exists in Illinois and South Dakota) are deliberately absent.
const VENUE_STATES: [(&str, &str); 34] = [
    ("university of minnesota", "MN"),
    ("macalester", "MN"),
    ("st. olaf", "MN"),
    ("st olaf", "MN"),
    ("carleton college", "MN"),
    ("hamline", "MN"),
    ("gustavus", "MN"),
    ("bethel university", "MN"),
    ("university of st. thomas", "MN"),
    ("minnesota state mankato", "MN"),
    ("bemidji state", "MN"),
    ("concordia college moorhead", "MN"),
    ("university of iowa", "IA"),
    ("northern iowa", "IA"),
    ("iowa state", "IA"),
    ("wartburg", "IA"),
    ("drake university", "IA"),
    ("luther college", "IA"),
    ("simpson college", "IA"),
    ("coe college", "IA"),
    ("central college", "IA"),
    ("grinnell", "IA"),
    ("cornell college", "IA"),
    ("loras", "IA"),
    ("buena vista university", "IA"),
    ("dubuque", "IA"),
    ("mount mercy", "IA"),
    ("uw-", "WI"),
    ("university of wisconsin", "WI"),
    ("-la crosse", "WI"),
    ("eau claire", "WI"),
    ("oshkosh", "WI"),
    ("stevens point", "WI"),
    ("whitewater", "WI"),
];

/// Resolve a venue to a state, or `None` when no unambiguous marker matches.
pub fn venue_state(location: &str) -> Option<&'static str> {
    let location = location.to_ascii_lowercase();
    VENUE_STATES
        .iter()
        .find(|(marker, _)| location.contains(marker))
        .map(|(_, state)| *state)
}

/// The states this provider operates in. It is asked only about school-shaped venues, and only
/// inside its own region: seeking a venue name nationally turns "Austin HS" into a three-way tie
/// with Indiana and Michigan, while the provider's Austin is the Minnesota one. A name that still
/// answers in two region states is left unresolved.
const REGION_STATES: [&str; 3] = ["MN", "IA", "WI"];

/// How one schedule row's venue became a state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenueResolution {
    /// A recurring site the provider publishes ([`venue_state`]).
    Site(&'static str),
    /// A venue that names a school, resolved through the consolidated school snapshot.
    School(&'static str),
    /// Answers in more than one state, or in none: never guessed.
    Unknown,
}

impl VenueResolution {
    pub const fn state(self) -> Option<&'static str> {
        match self {
            VenueResolution::Site(state) | VenueResolution::School(state) => Some(state),
            VenueResolution::Unknown => None,
        }
    }
}

/// The readings of a venue cell worth asking the school snapshot for.
///
/// Always the cell itself, plus - when it ends in a school suffix - the spelling with that suffix
/// written out (`"Albany HS"` -> `"Albany High School"`). The shared resolver needs at least two
/// tokens to tell "Albany" from the next Albany, so a truncated `"Albany"` would never match; the
/// expanded spelling is what a canonical school name actually looks like.
pub fn venue_candidates(location: &str) -> Vec<String> {
    let trimmed = location.trim();
    let mut candidates = vec![trimmed.to_string()];
    for (suffix, expansion) in [
        (" H.S.", " High School"),
        (" H.S", " High School"),
        (" HS.", " High School"),
        (" HS", " High School"),
        (" Middle School", " Middle School"),
        (" School", " School"),
    ] {
        let Some(base) = trimmed.strip_suffix(suffix) else {
            continue;
        };
        let base = base.trim();
        if base.is_empty() {
            continue;
        }
        let expanded = format!("{base}{expansion}");
        if !candidates.contains(&expanded) {
            candidates.push(expanded);
        }
    }
    candidates
}

/// Resolve a venue cell to a state: the venue table first, the school snapshot second.
///
/// Only a single answering state is accepted, and answers are cached per venue string because a
/// schedule repeats its sites.
pub fn resolve_venue(
    index: &SchoolIndex,
    cache: &mut HashMap<String, VenueResolution>,
    location: &str,
) -> VenueResolution {
    if let Some(cached) = cache.get(location) {
        return *cached;
    }
    let resolution = match venue_state(location) {
        Some(state) => VenueResolution::Site(state),
        None => {
            let candidates = venue_candidates(location);
            let mut hits: Vec<&'static str> = REGION_STATES
                .iter()
                .copied()
                .filter(|state| {
                    candidates
                        .iter()
                        .any(|label| index.resolve(state, label).is_some())
                })
                .collect();
            hits.dedup();
            match hits.as_slice() {
                [only] => VenueResolution::School(only),
                _ => VenueResolution::Unknown,
            }
        }
    };
    cache.insert(location.to_string(), resolution);
    resolution
}

/// Competition level from the meet name the provider publishes.
pub fn level_of(name: &str) -> CompetitionLevel {
    let name = name.to_ascii_lowercase();
    let has = |needle: &str| name.contains(needle);
    if has("state") {
        CompetitionLevel::State
    } else if has("sectional") || has("section ") {
        CompetitionLevel::Sectional
    } else if has("regional") {
        CompetitionLevel::Regional
    } else if has("conference") || has("conf.") {
        CompetitionLevel::Conference
    } else if has("district") {
        CompetitionLevel::District
    } else if has("national") {
        CompetitionLevel::National
    } else if has("dual") {
        CompetitionLevel::Dual
    } else if has("invitational") || has("invite") || has("relays") || has("classic") || has("meet")
    {
        CompetitionLevel::Invitational
    } else {
        CompetitionLevel::Unknown
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    const TRACK_2026: &str = include_str!("../../tests/fixtures/wayzata/track_2026_schedule.html");
    const XC_2026: &str = include_str!("../../tests/fixtures/wayzata/xc_2026_schedule.html");
    const OBSERVED_ON: &str = "2026-09-20";

    #[test]
    fn schedule_rows_read_the_track_table_with_its_month_heading_and_provider_slug() {
        let rows = schedule_rows(TRACK_2026, 2026).expect("the track schedule parses");
        assert_eq!(rows.len(), 13, "one row per competition day in the excerpt");
        let first = &rows[0];
        assert_eq!(first.date, "2026-01-04");
        assert_eq!(first.name, "USATF Minnesota All-Comers Meet #3");
        assert_eq!(first.location, "University of Minnesota");
        assert_eq!(first.slug.as_deref(), Some("7vqvs7"));
        assert!(
            first
                .aria_label
                .as_deref()
                .is_some_and(|label| label.contains("USATF Minnesota All-Comers Meet #3")),
            "the link's own label repeats the event: {:?}",
            first.aria_label
        );
        assert!(
            rows.windows(2).all(|pair| pair[0].date <= pair[1].date),
            "the schedule is published in date order"
        );
    }

    #[test]
    fn schedule_rows_read_the_cross_country_table() {
        let rows = schedule_rows(XC_2026, 2026).expect("the cross-country schedule parses");
        assert_eq!(rows.len(), 10, "one row per competition day in the excerpt");
        assert_eq!(rows[0].date, "2026-08-27");
        assert_eq!(rows[0].name, "River Falls Extreme Meet");
        assert_eq!(rows[0].location, "UW-River Falls");
        assert_eq!(rows[0].slug.as_deref(), Some("v70kmd"));
    }

    #[test]
    fn venue_state_only_answers_for_unambiguous_venues() {
        assert_eq!(venue_state("University of Minnesota"), Some("MN"));
        assert_eq!(venue_state("Wartburg College"), Some("IA"));
        assert_eq!(venue_state("UW-River Falls"), Some("WI"));
        assert_eq!(venue_state("St. Croix Falls HS"), None);
        assert_eq!(
            venue_state("Augustana College"),
            None,
            "exists in two states"
        );
    }

    #[test]
    fn level_of_reads_the_round_out_of_the_meet_name() {
        assert_eq!(
            level_of("WIAA State Championships"),
            CompetitionLevel::State
        );
        assert_eq!(level_of("D1 Sectional 4"), CompetitionLevel::Sectional);
        assert_eq!(level_of("Regional Final"), CompetitionLevel::Regional);
        assert_eq!(
            level_of("Mississippi Valley Conference"),
            CompetitionLevel::Conference
        );
        assert_eq!(
            level_of("Ron Kretsch Invitational"),
            CompetitionLevel::Invitational
        );
        assert_eq!(
            level_of("Milaca Early Bird Invite"),
            CompetitionLevel::Invitational
        );
        assert_eq!(level_of("Zzz"), CompetitionLevel::Unknown);
    }

    #[test]
    fn a_track_row_lands_in_the_season_its_month_belongs_to() {
        assert_eq!(ScheduleSport::Track.sport_for(1), Sport::IndoorTrack);
        assert_eq!(ScheduleSport::Track.sport_for(3), Sport::IndoorTrack);
        assert_eq!(ScheduleSport::Track.sport_for(12), Sport::IndoorTrack);
        assert_eq!(ScheduleSport::Track.sport_for(5), Sport::OutdoorTrack);
        assert_eq!(
            ScheduleSport::CrossCountry.sport_for(9),
            Sport::CrossCountry
        );
    }

    fn seed_cache(cache_dir: &std::path::Path, url: &str, body: &str) {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"GET");
        hasher.update([0x1f]);
        hasher.update(url.as_bytes());
        hasher.update([0x1f]);
        let key: String = hasher.finalize()[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let meta = json!({
            "url": url,
            "method": "GET",
            "status": 200,
            "sha256": format!("{:x}", Sha256::digest(body.as_bytes())),
            "bytes": body.len(),
            "fetched_at": "2026-09-20T14:39:00Z",
        });
        std::fs::write(
            cache_dir.join(format!("{key}.meta.json")),
            serde_json::to_string(&meta).expect("meta json"),
        )
        .expect("write meta");
        std::fs::write(cache_dir.join(format!("{key}.body")), body).expect("write body");
    }

    #[tokio::test]
    async fn collect_mints_core_meets_from_the_provider_schedule_without_touching_the_links() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cache = dir.path().join("http");
        std::fs::create_dir_all(&cache).expect("cache dir");
        seed_cache(
            &cache,
            &schedule_url(ScheduleSport::Track, 2026),
            TRACK_2026,
        );
        seed_cache(
            &cache,
            &schedule_url(ScheduleSport::CrossCountry, 2026),
            XC_2026,
        );

        let store = crate::store::Store::open(dir.path().join("store")).expect("store");
        let fetcher = crate::net::Fetcher::new(
            &cache,
            None,
            std::time::Duration::from_millis(1),
            std::collections::HashMap::new(),
            Vec::new(),
        )
        .expect("fetcher");
        let ctx = AdapterContext {
            fetcher: &fetcher,
            store: &store,
            refresh: false,
            school_year: crate::model::SchoolYear(2026),
            observed_on: OBSERVED_ON.to_string(),
        };
        let options = Options {
            years: vec![2026],
            limit: None,
            refresh: false,
            observed_on: Some(OBSERVED_ON.to_string()),
        };
        let report = collect(&ctx, &options)
            .await
            .expect("collect returns a report");

        assert_eq!(report.rows, 23, "13 track rows plus 10 cross-country rows");
        assert_eq!(report.errors, 0);
        assert_eq!(
            report.requests, 0,
            "both schedules came from the seeded cache"
        );
        assert_eq!(report.from_cache, 2);
        assert!(
            report
                .notes
                .iter()
                .any(|note| note.contains("venue state resolution") && note.contains("sites=12")),
            "the runner states its own venue resolution: {:?}",
            report.notes
        );

        // The meets are in the store with core evidence and the provider's own key.
        let appended: Vec<CanonicalMeet> = store.scan(Table::Meets).expect("scan meets from Fjall");
        assert!(
            appended.len() >= 20,
            "most rows mint a distinct meet; {} were appended",
            appended.len()
        );

        let opener = appended
            .iter()
            .find(|meet| meet.name == "USATF Minnesota All-Comers Meet #3")
            .expect("the first track row is a meet");
        assert_eq!(opener.state, "MN");
        assert_eq!(opener.date, "2026-01-04");
        assert_eq!(opener.location.as_deref(), Some("University of Minnesota"));
        assert_eq!(opener.sports, vec![Sport::IndoorTrack]);
        assert_eq!(opener.level, CompetitionLevel::Invitational);
        assert!(
            opener
                .source_urls
                .iter()
                .any(|url| url.ends_with("/links/7vqvs7")),
            "the provider link is retained as a key, not fetched: {:?}",
            opener.source_urls
        );
        assert!(
            opener
                .evidence
                .iter()
                .all(|evidence| evidence.source.id == ADAPTER_ID),
            "core evidence only: {:?}",
            opener.evidence
        );
        assert_eq!(
            opener.source_identities,
            vec![SourceIdentity::new(
                SourceNamespace::TimerMeet {
                    provider: PROVIDER.to_string(),
                },
                "7vqvs7",
            )]
        );

        let xc = appended
            .iter()
            .find(|meet| meet.name == "River Falls Extreme Meet")
            .expect("the first cross-country row is a meet");
        assert_eq!(xc.state, "WI");
        assert_eq!(xc.date, "2026-08-27");
        assert_eq!(xc.sports, vec![Sport::CrossCountry]);

        // A venue the table does not claim lands under the explicit unknown bucket.
        assert!(
            appended
                .iter()
                .filter(|meet| meet.state == UNKNOWN_STATE)
                .all(|meet| meet.state != "MN"),
            "unresolved venues are never filed under the provider's home state"
        );
    }

    #[tokio::test]
    async fn a_journaled_schedule_is_skipped_on_the_next_run() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cache = dir.path().join("http");
        std::fs::create_dir_all(&cache).expect("cache dir");
        seed_cache(
            &cache,
            &schedule_url(ScheduleSport::Track, 2026),
            TRACK_2026,
        );
        seed_cache(
            &cache,
            &schedule_url(ScheduleSport::CrossCountry, 2026),
            XC_2026,
        );
        let store = crate::store::Store::open(dir.path().join("store")).expect("store");
        let fetcher = crate::net::Fetcher::new(
            &cache,
            None,
            std::time::Duration::from_millis(1),
            std::collections::HashMap::new(),
            Vec::new(),
        )
        .expect("fetcher");
        let ctx = AdapterContext {
            fetcher: &fetcher,
            store: &store,
            refresh: false,
            school_year: crate::model::SchoolYear(2026),
            observed_on: OBSERVED_ON.to_string(),
        };
        let options = Options {
            years: vec![2026],
            limit: None,
            refresh: false,
            observed_on: Some(OBSERVED_ON.to_string()),
        };
        collect(&ctx, &options).await.expect("first run");
        let second = collect(&ctx, &options).await.expect("second run");
        assert_eq!(second.rows, 0, "both schedules are already journaled");
        assert_eq!(second.from_cache, 0, "and are not even read again");
    }

    #[test]
    fn a_school_shaped_venue_is_read_with_its_suffix_written_out() {
        assert_eq!(
            venue_candidates("Albany HS"),
            vec!["Albany HS", "Albany High School"]
        );
        assert_eq!(
            venue_candidates("St. Croix Falls H.S."),
            vec!["St. Croix Falls H.S.", "St. Croix Falls High School"]
        );
        assert_eq!(venue_candidates("Blake School"), vec!["Blake School"]);
        assert_eq!(
            venue_candidates("Bassett Creek Park"),
            vec!["Bassett Creek Park"]
        );
        assert_eq!(venue_candidates(" HS"), vec!["HS"]);
    }

    #[test]
    fn a_school_venue_resolves_only_where_exactly_one_state_owns_it() {
        // Canonical schools carry a normalized name, exactly as the store writes them.
        let school = |state: &str, name: &str| {
            crate::model::CanonicalSchool::new(state, name, crate::model::normalize_name(name)).0
        };
        let mut cache = HashMap::new();

        // The same school name in two states is never guessed at.
        let both = SchoolIndex::from_schools(&[
            school("MN", "Albany High School"),
            school("WI", "Albany High School"),
        ]);
        assert_eq!(
            resolve_venue(&both, &mut cache, "Albany HS"),
            VenueResolution::Unknown
        );

        // With one owner it resolves, including for the punctuated spelling a timer may print.
        let one = SchoolIndex::from_schools(&[
            school("MN", "Albany High School"),
            school("WI", "River Falls High School"),
        ]);
        let mut cache = HashMap::new();
        assert_eq!(
            resolve_venue(&one, &mut cache, "Albany HS"),
            VenueResolution::School("MN")
        );
        assert_eq!(
            resolve_venue(&one, &mut cache, "Albany H.S."),
            VenueResolution::School("MN")
        );
        assert_eq!(
            resolve_venue(&one, &mut cache, "River Falls HS"),
            VenueResolution::School("WI")
        );
        assert_eq!(
            resolve_venue(&one, &mut cache, "Bassett Creek Park"),
            VenueResolution::Unknown
        );
        assert_eq!(
            resolve_venue(&one, &mut cache, "University of Minnesota"),
            VenueResolution::Site("MN")
        )
    }
}
