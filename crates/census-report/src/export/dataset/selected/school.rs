use super::invalid;
use crate::export::dataset::Loaded;
use crate::export::qualification::participation::school_projection::SchoolProjection;
use crate::report::ReportResult;
use census_store::Entity;

pub(super) fn extend(loaded: &mut Loaded, projection: SchoolProjection) -> ReportResult<()> {
    let count = projection.rows.len();
    reserve(&mut loaded.events, count)?;
    reserve(&mut loaded.meets, count)?;
    reserve(&mut loaded.teams, count)?;
    reserve(&mut loaded.performances, count)?;
    for row in projection.rows {
        loaded.events.push(row.event);
        loaded.meets.push(row.meet);
        loaded.teams.push(row.team);
        loaded.performances.push(row.performance);
    }
    merge(&mut loaded.events, false)?;
    merge(&mut loaded.meets, false)?;
    merge(&mut loaded.teams, false)?;
    merge(&mut loaded.performances, true)
}

fn reserve<T>(rows: &mut Vec<T>, count: usize) -> ReportResult<()> {
    rows.try_reserve(count)
        .map_err(|_| invalid("school-proved projection allocation refused"))
}

fn merge<T: Entity + PartialEq>(rows: &mut Vec<T>, exact: bool) -> ReportResult<()> {
    if rows.is_empty() {
        return Ok(());
    }
    rows.sort_unstable_by(|left, right| left.entity_id().cmp(right.entity_id()));
    let mut merged: Vec<T> = Vec::new();
    reserve(&mut merged, rows.len())?;
    for row in std::mem::take(rows) {
        if let Some(prior) = merged
            .last_mut()
            .filter(|prior| prior.entity_id() == row.entity_id())
        {
            if exact && *prior != row {
                return Err(invalid(
                    "school-proved performance conflicts with a physical source",
                ));
            }
            if !exact {
                prior.merge(row);
            }
        } else {
            merged.push(row);
        }
    }
    for row in &mut merged {
        row.publish();
    }
    *rows = merged;
    Ok(())
}
