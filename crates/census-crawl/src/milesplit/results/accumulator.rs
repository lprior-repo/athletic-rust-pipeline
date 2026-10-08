use super::{budget, Accumulator};
use crate::CrawlResult;

impl Accumulator {
    pub(super) fn reserved(rows: usize) -> CrawlResult<Self> {
        let mut held = Self::default();
        held.meets.try_reserve(1).map_err(budget::reserve)?;
        held.events.try_reserve(rows).map_err(budget::reserve)?;
        held.teams.try_reserve(rows).map_err(budget::reserve)?;
        held.athletes.try_reserve(rows).map_err(budget::reserve)?;
        held.performances
            .try_reserve(rows)
            .map_err(budget::reserve)?;
        held.observations
            .try_reserve(rows)
            .map_err(budget::reserve)?;
        held.retained.try_reserve(rows).map_err(budget::reserve)?;
        Ok(held)
    }
}
