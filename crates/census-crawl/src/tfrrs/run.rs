use super::map::{Absorb, ListContext, Page, RosterContext};
use super::parse::{jurisdiction_of_url, list_filter, parse_list_page, parse_team_page};
use super::report::EntityCounts;
use super::{classify, source_id, Route, PHASE};
use crate::{AdapterContext, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, SourceRef,
};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;
use census_store::Table;
use serde_json::json;
use std::collections::HashSet;

struct Fetched {
    jurisdiction: UsJurisdiction,
    route: Route,
    body: String,
}

struct Claim {
    url: String,
    kind: &'static str,
    rows: u64,
}

pub(super) struct Run<'a> {
    pub(super) absorb: Absorb<'a>,
    pub(super) done: HashSet<String>,
    pub(super) failures: Vec<String>,
    claims: Vec<Claim>,
    pub(super) pages: u64,
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

    fn journal(&self, batch: &mut crate::recording::RowBatch<'_>) -> CrawlResult<()> {
        for claim in &self.claims {
            let payload = json!({ "url": claim.url, "kind": claim.kind, "rows": claim.rows });
            batch.journal_done(PHASE, &claim.url, &payload)?;
        }
        Ok(())
    }

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
        let mut batch = ctx.write_batch();
        accumulated.unsupported.append_to(&mut batch)?;
        batch.append_many(Table::Schools, &schools)?;
        batch.append_many(Table::Meets, &meets)?;
        batch.append_many(Table::Teams, &teams)?;
        batch.append_many(Table::Athletes, &athletes)?;
        batch.append_many(
            Table::SourceObservations,
            &ctx.athlete_observations(&athletes, schools.iter().chain(consolidated.iter())),
        )?;
        batch.append_many(Table::Events, &events)?;
        batch.append_many(Table::Performances, &performances)?;
        self.journal(&mut batch)?;
        batch.commit()?;
        self.claims.clear();
        Ok(EntityCounts {
            schools: schools.len(),
            meets: meets.len(),
            teams: teams.len(),
            athletes: athletes.len(),
            events: events.len(),
            performances: performances.len(),
            unsupported_cohorts: accumulated.unsupported.len(),
        })
    }
}
