use crate::export::ExportDataset;
use crate::grouping::Admission;
use crate::report::ReportResult;
use census_domain::model::{CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam};

pub(super) fn canonical(
    dataset: &ExportDataset,
    meet: &CanonicalMeet,
    event: &CanonicalEvent,
    team: &CanonicalTeam,
    performance: &CanonicalPerformance,
    school: &census_domain::model::SchoolId,
    budget: &mut Admission,
) -> ReportResult<()> {
    if event.meet != meet.id {
        return Err(super::super::invalid("school-projected event differs from its meet").into());
    }
    if performance.event != event.id || performance.meet != meet.id || performance.team != team.id {
        return Err(
            super::super::invalid("school-projected performance differs from its parents").into(),
        );
    }
    if &team.school != school {
        return Err(super::super::invalid("school-projected team differs from its school").into());
    }
    parents(dataset, meet, event, team, school, budget)?;
    for candidate in &dataset.performances {
        budget.charge(0, 1)?;
        if candidate.id == performance.id {
            if candidate != performance {
                return Err(super::super::invalid(
                    "school-projected performance conflicts with a physical source",
                )
                .into());
            }
            return Ok(());
        }
    }
    Err(super::super::invalid("school-projected performance absent from selected dataset").into())
}

fn parents(
    dataset: &ExportDataset,
    expected_meet: &CanonicalMeet,
    expected_event: &CanonicalEvent,
    expected_team: &CanonicalTeam,
    school: &census_domain::model::SchoolId,
    budget: &mut Admission,
) -> ReportResult<()> {
    budget.charge(0, 1)?;
    let team = dataset
        .teams
        .get(&expected_team.id)
        .ok_or_else(|| super::super::invalid("school-projected dataset team absent"))?;
    let meet = find(&dataset.meets, expected_meet.id.as_str(), budget)?;
    let event = find(&dataset.events, expected_event.id.as_str(), budget)?;
    if &team.school != school || event.meet != meet.id || meet.date != expected_meet.date {
        return Err(super::super::invalid(
            "school-projected dataset parents differ from checked authority",
        )
        .into());
    }
    if !team.retained_conflicts.is_empty()
        || !meet.retained_conflicts.is_empty()
        || !event.retained_conflicts.is_empty()
    {
        return Err(
            super::super::invalid("school-projected dataset parents retain a conflict").into(),
        );
    }
    Ok(())
}

fn find<'a, T: census_store::Entity>(
    rows: &'a [T],
    id: &str,
    budget: &mut Admission,
) -> ReportResult<&'a T> {
    for row in rows {
        budget.charge(0, 1)?;
        if row.entity_id() == id {
            return Ok(row);
        }
    }
    Err(super::super::invalid("school-projected dataset parent absent").into())
}

#[cfg(test)]
mod tests;
