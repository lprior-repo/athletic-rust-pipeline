use std::collections::HashMap;

use census_domain::model::{CanonicalAthlete, CanonicalSchool};
use census_review::athlete_flags::{key, IdentityKey};

use crate::grouping::{add, Admission, GroupingError};
use crate::report::ReportResult;

use super::super::StoreRows;

pub(super) fn inputs(rows: &StoreRows<'_>) -> ReportResult<()> {
    let mut admission = Admission::new("review");
    admission.rows(rows.schools)?;
    admission.rows(rows.meets)?;
    admission.rows(rows.athletes)?;
    admission.rows(rows.coaches)?;
    admission.rows(rows.verdicts)?;
    admission.rows(rows.review_cases)?;
    Ok(())
}

pub(super) fn conflicts(rows: &StoreRows<'_>, cohort: &[CanonicalAthlete]) -> ReportResult<()> {
    inputs(rows)?;
    let mut admission = Admission::new("conflicts");
    admission.rows(cohort)?;
    athlete_expansion(cohort, &mut admission)?;
    school_expansion(rows.schools, &mut admission)?;
    Ok(())
}

fn athlete_expansion(cohort: &[CanonicalAthlete], admission: &mut Admission) -> ReportResult<()> {
    let mut groups: HashMap<IdentityKey, (usize, usize)> = HashMap::new();
    cohort.iter().try_for_each(|athlete| -> ReportResult<()> {
        reserve(&mut groups)?;
        accumulate(
            groups.entry(key(athlete)).or_insert((0, 0)),
            athlete.id.as_str(),
        )
    })?;
    groups
        .values()
        .try_for_each(|group| expansion(*group, admission))
}

fn school_expansion(schools: &[CanonicalSchool], admission: &mut Admission) -> ReportResult<()> {
    let mut groups: HashMap<(&str, &str), (usize, usize)> = HashMap::new();
    schools.iter().try_for_each(|school| -> ReportResult<()> {
        reserve(&mut groups)?;
        let state = school.state.map_or("", |state| state.code());
        accumulate(
            groups
                .entry((state, &school.normalized_name))
                .or_insert((0, 0)),
            school.id.as_str(),
        )
    })?;
    groups
        .values()
        .try_for_each(|group| expansion(*group, admission))
}

fn reserve<K, V>(groups: &mut HashMap<K, V>) -> Result<(), GroupingError>
where
    K: Eq + std::hash::Hash,
{
    groups
        .try_reserve(1)
        .map_err(|_| GroupingError::Allocation { stage: "conflicts" })
}

fn accumulate(group: &mut (usize, usize), id: &str) -> ReportResult<()> {
    group.0 = add("conflicts", "group rows", group.0, 1)?;
    let bytes = add("conflicts", "group detail bytes", id.len(), 256)?;
    group.1 = add("conflicts", "group detail bytes", group.1, bytes)?;
    Ok(())
}

fn expansion((rows, bytes): (usize, usize), admission: &mut Admission) -> ReportResult<()> {
    if rows < 2 {
        return Ok(());
    }
    let projected = rows.checked_mul(bytes).ok_or(GroupingError::Overflow {
        stage: "conflicts",
        resource: "group detail bytes",
    })?;
    let work = rows.checked_mul(rows).ok_or(GroupingError::Overflow {
        stage: "conflicts",
        resource: "group work",
    })?;
    admission.charge(projected, work)?;
    Ok(())
}

#[cfg(test)]
mod tests;
