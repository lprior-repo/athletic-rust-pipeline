//! The flush: what one tournament run's accumulated rows become in the store.
//!
//! Split out of [`super::map`] to keep both files inside the source-length budget; `super::map` owns
//! the mapping and the run state, this file owns the one page that lands it — the six tables, the
//! school and athlete observations, and the journal entries the walk earned.

use super::journal::PHASE;
use super::map::{Mapper, ASSOCIATION};
use crate::{AdapterContext, CrawlResult};
use census_domain::model::SourceNamespace;
use census_store::Table;
use serde_json::Value;
use std::collections::HashMap;

/// What one run appended, per table.
#[derive(Debug, Default)]
pub(super) struct EntityCounts {
    pub(super) schools: usize,
    pub(super) meets: usize,
    pub(super) teams: usize,
    pub(super) athletes: usize,
    pub(super) events: usize,
    pub(super) performances: usize,
}

impl Mapper<'_> {
    /// Append every table the run accumulated, in the order the neighbouring adapters write them, then
    /// the journal entries the run earned.
    ///
    /// An empty table is skipped by `append_many` itself, so the calls stand unconditionally. The
    /// observations are direct writes, as the other arms write them: each is re-derived from these rows
    /// on every pass, so it is not what a resume decision reads. The tables and the entries commit in
    /// one page, so a meet counts as read only once the rows its walk filled are durable.
    pub(super) fn store(
        mut self,
        ctx: &AdapterContext<'_>,
        entries: Vec<(String, Value)>,
    ) -> CrawlResult<EntityCounts> {
        let schools = drain(&mut self.accumulated.schools);
        let meets = drain(&mut self.accumulated.meets);
        let teams = drain(&mut self.accumulated.teams);
        let athletes = drain(&mut self.accumulated.athletes);
        let events = drain(&mut self.accumulated.events);
        let performances = drain(&mut self.accumulated.performances);
        let mut performances = performances;
        crate::stamp_source_athletes(
            &SourceNamespace::AssociationAthlete {
                association: ASSOCIATION.to_string(),
            },
            &athletes,
            &mut performances,
        );
        let mut batch = ctx.store.write_batch();
        batch.append_many(Table::Schools, &schools)?;
        ctx.observe_schools(&SourceNamespace::association_school(ASSOCIATION), &schools)?;
        batch.append_many(Table::Meets, &meets)?;
        batch.append_many(Table::Teams, &teams)?;
        batch.append_many(Table::Athletes, &athletes)?;
        // Three identity channels and one athlete object each: the association's own entry number, the
        // net id, and the Live id, each under the namespace that id belongs to. `of_athlete` files
        // nothing for an athlete that carries none of them.
        ctx.observe_athletes(
            &SourceNamespace::AssociationAthlete {
                association: ASSOCIATION.to_string(),
            },
            &athletes,
            &schools,
        )?;
        ctx.observe_athletes(
            &SourceNamespace::athletic_net("athlete"),
            &athletes,
            &schools,
        )?;
        ctx.observe_athletes(&SourceNamespace::athletic_net("live"), &athletes, &schools)?;
        batch.append_many(Table::Events, &events)?;
        batch.append_many(Table::Performances, &performances)?;
        for (key, payload) in entries {
            batch.journal_done(PHASE, &key, &payload)?;
        }
        batch.commit()?;
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

/// Take a table's rows out of the accumulator.
fn drain<T>(rows: &mut HashMap<String, T>) -> Vec<T> {
    rows.drain().map(|(_, row)| row).collect()
}
