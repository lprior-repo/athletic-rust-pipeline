use super::EntityCounts;
use crate::athleticnet::map::Accumulator;
use crate::{AdapterContext, CrawlResult};
use census_domain::model::SourceNamespace;
use census_store::Table;
use serde::Serialize;

const PHASE: &str = "net_profile_projection_v4";

pub(super) fn persist(ctx: &AdapterContext<'_>, rows: Accumulator) -> CrawlResult<EntityCounts> {
    let schools = write_rows(ctx, Table::Schools, rows.schools.values())?;
    rows.schools.values().try_for_each(|school| {
        write_rows(
            ctx,
            Table::SourceObservations,
            ctx.school_observation(&SourceNamespace::athletic_net(super::SCHOOL_KIND), school)
                .iter(),
        )
        .map(|_| ())
    })?;
    let meets = write_rows(ctx, Table::Meets, rows.meets.values())?;
    let teams = write_rows(ctx, Table::Teams, rows.teams.values())?;
    let athletes = write_rows(ctx, Table::Athletes, rows.athletes.values())?;
    rows.athletes.values().try_for_each(|athlete| {
        write_rows(
            ctx,
            Table::SourceObservations,
            ctx.athlete_observations(std::slice::from_ref(athlete), rows.schools.values())
                .iter(),
        )
        .map(|_| ())
    })?;
    let events = write_rows(ctx, Table::Events, rows.events.values())?;
    let performances = write_rows(ctx, Table::Performances, rows.performances.values())?;
    let unsupported_cohorts = rows.unsupported.len();
    let (reviews, observations) = rows.unsupported.into_parts();
    write_rows(
        ctx,
        Table::ReviewCases,
        reviews.iter().chain(rows.profile_reviews.iter()),
    )?;
    write_rows(
        ctx,
        Table::SourceObservations,
        observations.iter().chain(rows.profile_observations.iter()),
    )?;
    Ok(EntityCounts {
        schools,
        meets,
        teams,
        athletes,
        events,
        performances,
        unsupported_cohorts,
    })
}

fn write_rows<'a, T: Serialize + 'a>(
    ctx: &AdapterContext<'_>,
    table: Table,
    mut rows: impl Iterator<Item = &'a T>,
) -> CrawlResult<usize> {
    rows.try_fold(0usize, |count, row| {
        let written = ctx.append_row_once(PHASE, table, row)?;
        count
            .checked_add(usize::from(written))
            .ok_or_else(super::walk::counter_error)
    })
}
