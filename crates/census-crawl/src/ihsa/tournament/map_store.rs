use super::journal::PHASE;
use super::map::{Mapper, ASSOCIATION};
use crate::recording::{projection::append_new, RowBatch};
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Evidence, SourceAthleteObservation, SourceNamespace,
    SourceObservation, SourceSchoolObservation,
};
use census_store::Table;
use serde_json::Value;
use std::collections::HashMap;

const APPLICATION_PHASE: &str = "ihsa_tournament_fact_effects_v2";

#[derive(Debug, Default)]
pub(super) struct EntityCounts {
    pub(super) schools: usize,
    pub(super) meets: usize,
    pub(super) teams: usize,
    pub(super) athletes: usize,
    pub(super) events: usize,
    pub(super) performances: usize,
    pub(super) relay_results: usize,
    pub(super) unsupported_cohorts: usize,
}

impl Mapper<'_> {
    pub(super) fn store(
        mut self,
        ctx: &AdapterContext<'_>,
        entries: Vec<(String, Value)>,
    ) -> CrawlResult<EntityCounts> {
        let mut batch = ctx.write_batch();
        let counts = stage(ctx, &mut batch, std::mem::take(&mut self.accumulated))?;
        for (key, payload) in entries {
            batch.journal_done(PHASE, &key, &payload)?;
        }
        batch.commit()?;
        Ok(counts)
    }
}

fn drain<T>(rows: HashMap<String, T>) -> CrawlResult<Vec<T>> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(rows.len())
        .map_err(|_| allocation(rows.len()))?;
    values.extend(rows.into_values());
    Ok(values)
}

fn stage(
    ctx: &AdapterContext<'_>,
    batch: &mut RowBatch<'_>,
    held: super::map::Accumulator,
) -> CrawlResult<EntityCounts> {
    let super::map::Accumulator {
        schools,
        meets,
        teams,
        athletes,
        events,
        performances,
        relay_results,
        unsupported,
    } = held;
    let schools = drain(schools)?;
    let athletes = drain(athletes)?;
    let unsupported_cohorts = stage_review(ctx, batch, unsupported)?;
    append_new(
        ctx,
        batch,
        Table::SourceObservations,
        observations(&athletes, &schools)?,
        APPLICATION_PHASE,
    )?;
    Ok(EntityCounts {
        schools: append_new(ctx, batch, Table::Schools, schools, APPLICATION_PHASE)?,
        meets: append_new(ctx, batch, Table::Meets, drain(meets)?, APPLICATION_PHASE)?,
        teams: append_new(ctx, batch, Table::Teams, drain(teams)?, APPLICATION_PHASE)?,
        athletes: append_new(ctx, batch, Table::Athletes, athletes, APPLICATION_PHASE)?,
        events: append_new(ctx, batch, Table::Events, drain(events)?, APPLICATION_PHASE)?,
        performances: append_new(
            ctx,
            batch,
            Table::Performances,
            drain(performances)?,
            APPLICATION_PHASE,
        )?,
        relay_results: append_new(
            ctx,
            batch,
            Table::RelayResults,
            drain(relay_results)?,
            APPLICATION_PHASE,
        )?,
        unsupported_cohorts,
    })
}

fn stage_review(
    ctx: &AdapterContext<'_>,
    batch: &mut RowBatch<'_>,
    unsupported: crate::cohort::UnsupportedCohortRows,
) -> CrawlResult<usize> {
    let (cases, unsupported) = unsupported.into_parts();
    let unsupported_cohorts = cases.len();
    append_new(ctx, batch, Table::ReviewCases, cases, APPLICATION_PHASE)?;
    append_new(
        ctx,
        batch,
        Table::SourceObservations,
        unsupported,
        APPLICATION_PHASE,
    )?;
    Ok(unsupported_cohorts)
}

fn observations(
    athletes: &[CanonicalAthlete],
    schools: &[CanonicalSchool],
) -> CrawlResult<Vec<SourceObservation>> {
    let capacity = athletes
        .len()
        .checked_add(schools.len())
        .ok_or_else(missing_capture)?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(capacity)
        .map_err(|_| allocation(capacity))?;
    for school in schools {
        let row = SourceSchoolObservation::of_school(
            &SourceNamespace::association_school(ASSOCIATION),
            school,
            stamp(&school.evidence)?,
        )
        .ok_or_else(missing_capture)?;
        rows.push(SourceObservation::School(row));
    }
    for athlete in athletes {
        let source = athlete.source.as_ref().ok_or_else(missing_capture)?;
        let school = schools
            .iter()
            .find(|school| school.id == athlete.school)
            .map(|school| school.name.clone());
        let row = SourceAthleteObservation::of_athlete(
            &source.namespace,
            athlete,
            school,
            stamp(&athlete.evidence)?,
        )
        .ok_or_else(missing_capture)?;
        rows.push(SourceObservation::Athlete(row));
    }
    Ok(rows)
}

fn stamp(evidence: &[Evidence]) -> CrawlResult<&str> {
    evidence
        .iter()
        .map(|evidence| evidence.observed_on.as_str())
        .min()
        .ok_or_else(missing_capture)
}

fn missing_capture() -> CrawlError {
    CrawlError::Invariant {
        detail: "IHSA fact has no captured physical source identity".into(),
    }
}

fn allocation(requested: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "IHSA fact allocation",
        requested,
        limit: requested,
    }
}
