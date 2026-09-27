use super::journal::PHASE;
use super::map::{Mapper, ASSOCIATION};
use crate::{AdapterContext, CrawlResult};
use census_store::Table;
use serde_json::Value;
use std::collections::HashMap;

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
        let mut batch = ctx.store.write_batch();
        batch.append_many(Table::Schools, &schools)?;
        batch.append_many(Table::SourceObservations, &ctx.school_observations(&census_domain::model::SourceNamespace::association_school(ASSOCIATION), &schools))?;
        batch.append_many(Table::Meets, &meets)?;
        batch.append_many(Table::Teams, &teams)?;
        batch.append_many(Table::Athletes, &athletes)?;
        batch.append_many(Table::SourceObservations, &ctx.athlete_observations(&athletes, &schools))?;
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

fn drain<T>(rows: &mut HashMap<String, T>) -> Vec<T> {
    rows.drain().map(|(_, row)| row).collect()
}
