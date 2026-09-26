//! The page walk: one supplied URL per request, read by the route it names, absorbed and journaled.
//!
//! Split out of [`super`] to keep both files inside the source-length budget: this file owns the
//! run state (the writer, the resume set, the page counters) and the per-page read; `super` owns
//! the entry point and `report` the notes.
//!
//! Ordering is deliberate: a page is journaled only *after* every entity it minted is in the store
//! (`collect` appends, then calls [`Run::journal`]). A kill between the two leaves pages unclaimed
//! rather than claimed-without-rows, so the next run re-reads them from cache and the journal never
//! says a page is done whose rows are absent. A failed journal write is recorded rather than
//! dropped, so the page stays unfinished.

use super::map::{Absorb, ListContext, Page, RosterContext};
use super::parse::{jurisdiction_of_url, list_filter, parse_list_page, parse_team_page};
use super::report::EntityCounts;
use super::{classify, source_id, Route, PHASE};
use crate::{AdapterContext, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, SourceNamespace, SourceRef,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use census_store::Table;
use serde_json::json;
use std::collections::HashSet;

/// One supplied URL, placed and fetched, ready to be read by its route.
struct Fetched {
    jurisdiction: UsJurisdiction,
    route: Route,
    body: String,
}

/// What one page's journal record will say once its rows are in the store.
struct Claim {
    url: String,
    kind: &'static str,
    rows: u64,
}

/// What one walk carries across pages.
pub(super) struct Run<'a> {
    /// The run's writer: the accumulator, the school index and the counters.
    pub(super) absorb: Absorb<'a>,
    /// URLs a previous run already journaled at the current phase.
    pub(super) done: HashSet<String>,
    /// Pages that were refused, could not be fetched, or could not be journaled.
    pub(super) failures: Vec<String>,
    /// Pages read this run, held back until their rows are appended.
    claims: Vec<Claim>,
    /// Pages read this run.
    pub(super) pages: u64,
    /// Pages a previous run had already journaled.
    pub(super) resumed: u64,
}

impl<'a> Run<'a> {
    pub(super) fn new(index: &'a SchoolIndex, done: HashSet<String>) -> Self {
        Self {
            absorb: Absorb::new(index),
            done,
            failures: Vec::new(),
            claims: Vec::new(),
            pages: 0,
            resumed: 0,
        }
    }

    /// Place and fetch one supplied URL, recording why it could not be read.
    async fn fetched(&mut self, ctx: &AdapterContext<'_>, url: &str) -> Option<Fetched> {
        let Some(jurisdiction) = jurisdiction_of_url(url) else {
            self.failures.push(format!(
                "{url}: the host names no state this reader can place"
            ));
            return None;
        };
        let Some(route) = classify(url) else {
            self.failures
                .push(format!("{url}: not a TFRRS performance-list or team page"));
            return None;
        };
        match ctx.fetcher.get(url, &ctx.fetch_options()).await {
            Ok(page) => Some(Fetched {
                jurisdiction,
                route,
                body: page.text(),
            }),
            Err(error) => {
                self.absorb.stats.fetches_failed =
                    self.absorb.stats.fetches_failed.saturating_add(1);
                self.failures.push(format!("{url}: {error}"));
                None
            }
        }
    }

    /// Read one supplied URL: classify it, fetch it, read it by its route, claim it.
    pub(super) async fn read(&mut self, ctx: &AdapterContext<'_>, url: &str) {
        if self.done.contains(url) {
            self.resumed = self.resumed.saturating_add(1);
            return;
        }
        let Some(Fetched {
            jurisdiction,
            route,
            body,
        }) = self.fetched(ctx, url).await
        else {
            return;
        };
        let source = SourceRef::new(source_id(jurisdiction), Some(url.to_string()));
        let page = Page {
            source: &source,
            observed_on: &ctx.observed_on,
            jurisdiction,
        };
        let (kind, rows) = match route {
            Route::List(list) => {
                let parsed = parse_list_page(&body);
                let context = ListContext {
                    page,
                    list: &list,
                    filter: list_filter(url),
                };
                self.absorb.absorb_list(&context, &parsed);
                let rows =
                    u64::try_from(parsed.sections.iter().map(|s| s.rows.len()).sum::<usize>())
                        .unwrap_or(u64::MAX);
                ("list", rows)
            }
            Route::Team(team) => {
                let parsed = parse_team_page(&body);
                let context = RosterContext { page, team: &team };
                self.absorb.absorb_roster(&context, &parsed);
                let rows = u64::try_from(parsed.athletes.len()).unwrap_or(u64::MAX);
                ("team", rows)
            }
        };
        self.pages = self.pages.saturating_add(1);
        self.claims.push(Claim {
            url: url.to_string(),
            kind,
            rows,
        });
    }

    /// Journal every page this run read, once the rows it minted are in the store.
    ///
    /// Called after [`Run::append`]: a page claimed here is one whose rows the store already holds,
    /// which is what makes a resumed run's `done` set sound.
    pub(super) fn journal(&mut self, ctx: &AdapterContext<'_>) {
        for claim in std::mem::take(&mut self.claims) {
            let payload = json!({ "url": claim.url, "kind": claim.kind, "rows": claim.rows });
            if let Err(error) = ctx.store.journal_done(PHASE, &claim.url, &payload) {
                self.failures
                    .push(format!("{}: journal {error}", claim.url));
            }
        }
    }

    /// Append every entity the walk accumulated and count what was written.
    ///
    /// The accumulator is taken rather than the run consumed: the report reads the run's counters
    /// after the append, and an `Accumulator` defaults to empty, so the take leaves it consistent.
    pub(super) fn append(
        &mut self,
        ctx: &AdapterContext<'_>,
        consolidated: &[CanonicalSchool],
    ) -> CrawlResult<EntityCounts> {
        let accumulated = std::mem::take(&mut self.absorb.accumulator);
        let schools: Vec<CanonicalSchool> = accumulated.schools.into_values().collect();
        let meets: Vec<CanonicalMeet> = accumulated.meets.into_values().collect();
        let teams: Vec<CanonicalTeam> = accumulated.teams.into_values().collect();
        let athletes: Vec<CanonicalAthlete> = accumulated.athletes.into_values().collect();
        let events: Vec<CanonicalEvent> = accumulated.events.into_values().collect();
        let performances: Vec<CanonicalPerformance> =
            accumulated.performances.into_values().collect();
        let mut performances = performances;
        crate::stamp_source_athletes(&SourceNamespace::TfrrsAthlete, &athletes, &mut performances);
        ctx.store.append_many(Table::Schools, &schools)?;
        ctx.store.append_many(Table::Meets, &meets)?;
        ctx.store.append_many(Table::Teams, &teams)?;
        ctx.store.append_many(Table::Athletes, &athletes)?;
        ctx.observe_athletes(
            &SourceNamespace::TfrrsAthlete,
            &athletes,
            schools.iter().chain(consolidated.iter()),
        )?;
        ctx.store.append_many(Table::Events, &events)?;
        ctx.store.append_many(Table::Performances, &performances)?;
        Ok(EntityCounts {
            schools: schools.len(),
            meets: meets.len(),
            teams: teams.len(),
            athletes: athletes.len(),
            events: events.len(),
            performances: performances.len(),
        })
    }
}
