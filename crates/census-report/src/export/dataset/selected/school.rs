use super::invalid;
use crate::export::dataset::Loaded;
use crate::export::qualification::participation::school_projection::SchoolProjection;
use crate::report::ReportResult;
use census_store::Entity;

pub(super) fn extend(loaded: &mut Loaded, projection: SchoolProjection) -> ReportResult<()> {
    let count = projection.rows.len();
    let physical_event_count = loaded.events.len();
    loaded
        .events
        .sort_unstable_by(|left, right| left.id.cmp(&right.id));
    reserve(&mut loaded.events, count)?;
    reserve(&mut loaded.meets, count)?;
    reserve(&mut loaded.teams, count)?;
    reserve(&mut loaded.performances, count)?;
    for row in projection.rows {
        if let Some(original) = &row.original_event_label {
            let physical = loaded
                .events
                .get_mut(..physical_event_count)
                .ok_or_else(|| invalid("selected school event inventory unavailable"))?;
            let first = physical.partition_point(|event| event.id < row.event.id);
            for existing in physical
                .get_mut(first..)
                .ok_or_else(|| invalid("selected school event inventory unavailable"))?
                .iter_mut()
                .take_while(|event| event.id == row.event.id)
            {
                existing.source_labels.retain(|label| label != original);
            }
        }
        loaded.events.push(row.event);
        loaded.meets.push(row.meet);
        loaded.teams.push(row.team);
        loaded.performances.push(row.performance);
    }
    merge(&mut loaded.events, |_, _| Ok(()))?;
    merge(&mut loaded.meets, |_, _| Ok(()))?;
    merge(&mut loaded.teams, |_, _| Ok(()))?;
    merge(&mut loaded.performances, |left, right| {
        let same =
            census_crawl::participation_evidence::physical_result::matches_physical_observation(
                left, right,
            )
            .map_err(crate::report::documentary_source_error)?;
        if same {
            Ok(())
        } else {
            Err(invalid(
                "school-proved performance conflicts with a physical source",
            ))
        }
    })
}

fn reserve<T>(rows: &mut Vec<T>, count: usize) -> ReportResult<()> {
    rows.try_reserve(count)
        .map_err(|_| invalid("school-proved projection allocation refused"))
}

fn merge<T: Entity>(
    rows: &mut Vec<T>,
    validate_duplicate: impl Fn(&T, &T) -> ReportResult<()>,
) -> ReportResult<()> {
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
            validate_duplicate(prior, &row)?;
            prior.merge(row);
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
