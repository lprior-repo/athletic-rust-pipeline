//! The Wayzata schedule walk: fetch each sport/season page, tally its competition rows, resolve
//! their venues and mint the core meets the run writes.
//!
//! Split out of the parent module's `collect` when that file outgrew the repository's 300-line
//! budget; the adapter entry point and the shared request counters stay in `mod.rs`.

use super::map::{level_of, resolve_venue, VenueResolution};
use super::parse::{schedule_rows, schedule_url, MeetRow, ScheduleSport};
use super::{stats_of, Options, ADAPTER_ID, BASE, PARSE_VERSION, PROVIDER};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{CanonicalMeet, Evidence, SourceIdentity, SourceNamespace, SourceRef};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use census_store::Table;
use serde_json::json;
use std::collections::{BTreeMap, HashMap, HashSet};

/// What one schedule walk counted, per page, row, venue resolution and bucket.
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

/// The schedule URLs this parser version has already journalled, so they need no second walk.
pub(super) fn completed_pages(ctx: &AdapterContext<'_>) -> CrawlResult<HashSet<String>> {
    Ok(ctx
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
        .collect())
}

/// The mutable run state one schedule walk threads through its page and row helpers.
pub(super) struct Walk {
    observed_on: String,
    stats: Stats,
    meets: BTreeMap<String, CanonicalMeet>,
    venue_cache: HashMap<String, VenueResolution>,
    /// The page entries this run has earned, committed in `finish` with the meets they minted.
    pending: Vec<(String, serde_json::Value)>,
    report: AdapterReport,
}

impl Walk {
    /// A walk minting meets observed on `observed_on`.
    pub(super) fn new(observed_on: String) -> Self {
        Self {
            observed_on,
            stats: Stats::default(),
            meets: BTreeMap::new(),
            venue_cache: HashMap::new(),
            pending: Vec::new(),
            report: AdapterReport::new(ADAPTER_ID, "meet-schedule rows"),
        }
    }

    /// Walk both sports' seasons, minting one core meet per competition row.
    pub(super) async fn run(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        done: &HashSet<String>,
        years: &[i16],
        index: &SchoolIndex,
    ) -> CrawlResult<()> {
        'sport: for sport in [ScheduleSport::Track, ScheduleSport::CrossCountry] {
            for year in years {
                let url = schedule_url(sport, *year);
                if done.contains(&url) {
                    continue;
                }
                if options.limit.is_some_and(|limit| self.stats.rows >= limit) {
                    break 'sport;
                }
                let fetched = ctx.fetcher.get(&url, &ctx.fetch_options()).await?;
                let rows = schedule_rows(&fetched.text(), *year)?;
                self.stats.pages = self.stats.pages.saturating_add(1);
                self.report
                    .note(format!("{url}: {} competition rows", rows.len()));

                for row in &rows {
                    if options.limit.is_some_and(|limit| self.stats.rows >= limit) {
                        break 'sport;
                    }
                    let month = month_of(&row.date);
                    let state = self.tally_row(row, sport, index, month);
                    self.mint_meet(row, sport, state, month, &url);
                }

                // The entry is buffered, not written: `finish` commits it in the same page as the meets
                // these rows minted. Written here it would mark the page read before its meets are
                // durable, and a run that stopped in between would skip the page with nothing stored.
                self.pending.push((
                    url.clone(),
                    json!({
                        "url": url,
                        "parser": PARSE_VERSION,
                        "sport": sport.as_str(),
                        "year": year,
                        "rows": rows.len(),
                    }),
                ));
            }
        }
        Ok(())
    }

    /// Count one competition row: its venue resolution, level bucket and sport bucket. The venue's
    /// state comes back for the meet the row mints.
    fn tally_row(
        &mut self,
        row: &MeetRow,
        sport: ScheduleSport,
        index: &SchoolIndex,
        month: u8,
    ) -> Option<UsJurisdiction> {
        self.stats.rows = self.stats.rows.saturating_add(1);
        let resolution = resolve_venue(index, &mut self.venue_cache, &row.location);
        let state = resolution.state();
        match resolution {
            VenueResolution::Site(_) => {
                self.stats.states_resolved = self.stats.states_resolved.saturating_add(1);
            }
            VenueResolution::School(_) => {
                self.stats.states_from_school = self.stats.states_from_school.saturating_add(1);
            }
            VenueResolution::Unknown => {
                self.stats.states_unknown = self.stats.states_unknown.saturating_add(1);
                let slot = self
                    .stats
                    .unresolved_venues
                    .entry(row.location.clone())
                    .or_default();
                *slot = slot.saturating_add(1);
            }
        }
        let level = level_of(&row.name);
        let slot = self.stats.levels.entry(format!("{level:?}")).or_default();
        *slot = slot.saturating_add(1);
        let slot = self
            .stats
            .sports
            .entry(format!("{:?}", sport.sport_for(month)))
            .or_default();
        *slot = slot.saturating_add(1);
        state
    }

    /// Mint the core meet one competition row describes, keeping the first minted for its identity.
    fn mint_meet(
        &mut self,
        row: &MeetRow,
        sport: ScheduleSport,
        state: Option<UsJurisdiction>,
        month: u8,
        url: &str,
    ) {
        let level = level_of(&row.name);
        // A venue that resolved to no jurisdiction mints an unplaced meet: the report spells that
        // bucket `MEET_STATE_UNRESOLVED`, and a real meet row is never filed under a guess.
        let mut meet = CanonicalMeet::new(state, &row.name, &row.date, level);
        meet.location = Some(row.location.clone());
        meet.sports.push(sport.sport_for(month));
        if let Some(slug) = &row.slug {
            meet.source_urls.push(format!("{BASE}/links/{slug}"));
        }
        meet.source_urls.push(url.to_string());
        meet.source_identities.push(SourceIdentity::new(
            SourceNamespace::TimerMeet {
                provider: PROVIDER.to_string(),
            },
            row.slug
                .clone()
                .unwrap_or_else(|| format!("{}|{}", row.date, meet.normalized_name)),
        ));
        let mut evidence = Evidence::parsed(
            SourceRef::new(ADAPTER_ID, Some(url.to_string())),
            &self.observed_on,
        );
        evidence.note = Some(match &row.aria_label {
            Some(label) => format!("provider schedule row: {label}"),
            None => format!("provider schedule row at {}", row.location),
        });
        meet.evidence.push(evidence);
        self.meets
            .entry(meet.id.as_str().to_string())
            .or_insert(meet);
    }

    /// Write the minted meets and emit the run's summary lines.
    pub(super) async fn finish(
        self,
        ctx: &AdapterContext<'_>,
        (requests_before, cache_before): (u64, u64),
    ) -> CrawlResult<AdapterReport> {
        let Walk {
            stats,
            meets,
            pending,
            mut report,
            ..
        } = self;
        let meets: Vec<CanonicalMeet> = meets.into_values().collect();
        // The minted meets and the entries naming the pages they came from commit together, so a page
        // counts as read only once the meets its rows produced are durable.
        let mut batch = ctx.write_batch();
        batch.append_many(Table::Meets, &meets)?;
        for (url, payload) in pending {
            batch.journal_done(ADAPTER_ID, &url, &payload)?;
        }
        batch.commit()?;

        let (requests_after, cache_after) = stats_of(ctx).await;
        report.rows = count(stats.rows);
        report.requests = requests_after.saturating_sub(requests_before);
        report.from_cache = cache_after.saturating_sub(cache_before);
        note_summary(&mut report, &stats, meets.len());
        Ok(report)
    }
}

/// The row date's month, or `0` for a date that is not `YYYY-MM-DD`.
fn month_of(date: &str) -> u8 {
    date.get(5..7)
        .and_then(|month| month.parse::<u8>().ok())
        .unwrap_or(0)
}

/// The four end-of-run summary lines: page/row/meet counts, venue states, levels and sports.
fn note_summary(report: &mut AdapterReport, stats: &Stats, meets: usize) {
    report.note(format!(
        "schedules: {} pages read, {} competition rows, {meets} core meets minted",
        stats.pages, stats.rows
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
}
