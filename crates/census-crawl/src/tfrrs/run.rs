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

#[path = "run_receipts.rs"]
mod run_receipts;
struct Fetched {
    jurisdiction: UsJurisdiction,
    route: Route,
    body: String,
    observed_on: String,
    body_sha256: String,
}

struct Claim {
    url: String,
    kind: &'static str,
    rows: u64,
    body_sha256: String,
}

pub(super) struct Run<'a> {
    pub(super) absorb: Absorb<'a>,
    pub(super) failures: Vec<String>,
    pub(super) unfinished: Vec<String>,
    claims: Vec<Claim>,
    pub(super) pages: u64,
    pub(super) resumed: u64,
}

impl<'a> Run<'a> {
    pub(super) fn new(index: &'a SchoolIndex) -> Self {
        Self {
            absorb: Absorb::new(index),
            failures: Vec::new(),
            unfinished: Vec::new(),
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
                observed_on: page.fetched_at,
                body_sha256: crate::net::cache::content_digest(&page.body),
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
        let Some(Fetched {
            jurisdiction,
            route,
            body,
            observed_on,
            body_sha256,
        }) = self.fetched(ctx, url).await
        else {
            self.unfinished.push(url.to_string());
            return;
        };
        let source = SourceRef::new(source_id(jurisdiction), Some(url.to_string()));
        let page = Page {
            source: &source,
            observed_on: &observed_on,
            performance_as_of: ctx.performance_as_of,
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
                if let Err(error) = self.absorb.absorb_list(&context, &parsed) {
                    self.failures
                        .push(format!("{url}: {error}; projection remains unfinished"));
                    self.unfinished.push(url.to_string());
                    return;
                }
                let rows =
                    u64::try_from(parsed.sections.iter().map(|s| s.rows.len()).sum::<usize>())
                        .map_or(u64::MAX, |value| value);
                ("list", rows)
            }
            Route::Team(team) => {
                let parsed = parse_team_page(&body);
                let context = RosterContext { page, team: &team };
                self.absorb.absorb_roster(&context, &parsed);
                let rows = u64::try_from(parsed.athletes.len()).map_or(u64::MAX, |value| value);
                ("team", rows)
            }
        };
        self.pages = self.pages.saturating_add(1);
        self.claims.push(Claim {
            url: url.to_string(),
            kind,
            rows,
            body_sha256,
        });
    }

    fn journal(
        &self,
        batch: &mut crate::recording::RowBatch<'_>,
        context: &str,
    ) -> CrawlResult<()> {
        self.claims.iter().try_for_each(|claim| {
            let key = run_receipts::key(context, &claim.url, &claim.body_sha256)?;
            let payload = json!({ "url": claim.url, "kind": claim.kind, "rows": claim.rows,
                "body_sha256": claim.body_sha256, "projection_context": context });
            batch.journal_done(run_receipts::PHASE, &key, &payload)
        })
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
        let school_count = append_rows(ctx, Table::Schools, &schools)?;
        let meet_count = append_rows(ctx, Table::Meets, &meets)?;
        let team_count = append_rows(ctx, Table::Teams, &teams)?;
        let athlete_count = append_rows(ctx, Table::Athletes, &athletes)?;
        append_rows(
            ctx,
            Table::SourceObservations,
            &ctx.athlete_observations(&athletes, schools.iter().chain(consolidated.iter())),
        )?;
        let event_count = append_rows(ctx, Table::Events, &events)?;
        let performance_count = append_rows(ctx, Table::Performances, &performances)?;
        self.journal(&mut batch, &run_receipts::context(ctx, consolidated)?)?;
        batch.commit()?;
        self.claims.clear();
        Ok(EntityCounts {
            schools: school_count,
            meets: meet_count,
            teams: team_count,
            athletes: athlete_count,
            events: event_count,
            performances: performance_count,
            unsupported_cohorts: accumulated.unsupported.len(),
        })
    }
}
fn append_rows<T: serde::Serialize>(
    ctx: &AdapterContext<'_>,
    table: Table,
    rows: &[T],
) -> CrawlResult<usize> {
    rows.iter().try_fold(0usize, |count, row| {
        if !ctx.append_row_once(PHASE, table, row)? {
            return Ok(count);
        }
        count
            .checked_add(1)
            .ok_or_else(|| crate::CrawlError::Arithmetic {
                detail: "TFRRS admitted row count overflow".into(),
            })
    })
}
