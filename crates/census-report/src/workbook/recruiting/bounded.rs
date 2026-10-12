use std::collections::{BTreeMap, HashMap};

use census_domain::model::{CanonicalCoach, SchoolYear};
use census_store::Entity;

use crate::grouping::{add, limit, Admission, GroupingError};
use crate::report::{ReportError, ReportResult};

use super::contact::{contacts, SchoolContacts};

pub(in crate::workbook) const OWNER_OBSERVATION_BUDGET: usize = 4_096;
pub(in crate::workbook) const SCHOOL_OBSERVATION_BUDGET: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::workbook) struct GroupBudget {
    owner_observations: usize,
    school_observations: usize,
}

impl GroupBudget {
    pub(in crate::workbook) const fn new(
        owner_observations: usize,
        school_observations: usize,
    ) -> Self {
        Self {
            owner_observations,
            school_observations,
        }
    }
}

pub(in crate::workbook) const WORKBOOK_GROUP_BUDGET: GroupBudget =
    GroupBudget::new(OWNER_OBSERVATION_BUDGET, SCHOOL_OBSERVATION_BUDGET);

fn increment<'a>(
    counts: &mut HashMap<&'a str, usize>,
    key: &'a str,
    resource: &'static str,
    budget: usize,
) -> ReportResult<()> {
    counts
        .try_reserve(1)
        .map_err(|_| GroupingError::Allocation { stage: "coaches" })?;
    let held = counts.entry(key).or_insert(0);
    let next = add("coaches", resource, *held, 1)?;
    limit("coaches", key, resource, next, budget)?;
    *held = next;
    Ok(())
}

fn check_group_budget(observations: &[CanonicalCoach], budget: GroupBudget) -> ReportResult<()> {
    let mut admission = Admission::new("coaches");
    admission.rows(observations)?;
    let mut owners = HashMap::new();
    let mut schools = HashMap::new();
    observations.iter().try_for_each(|observation| {
        increment(
            &mut schools,
            observation.school.as_str(),
            "coach observations",
            budget.school_observations,
        )?;
        increment(
            &mut owners,
            observation.id.as_str(),
            "claims",
            budget.owner_observations,
        )
    })
}

pub(in crate::workbook) fn bounded_claims(
    observations: &[CanonicalCoach],
    budget: GroupBudget,
) -> ReportResult<Vec<CanonicalCoach>> {
    check_group_budget(observations, budget)?;
    let mut positions: HashMap<&str, usize> = HashMap::new();
    let mut output: Vec<CanonicalCoach> = Vec::new();
    observations
        .iter()
        .try_for_each(|observation| -> ReportResult<()> {
            if let Some(position) = positions.get(observation.id.as_str()).copied() {
                let held = output
                    .get_mut(position)
                    .ok_or_else(|| ReportError::Invariant {
                        detail: "merged coach position is absent".to_owned(),
                    })?;
                held.merge(observation.clone());
            } else {
                positions
                    .try_reserve(1)
                    .map_err(|_| GroupingError::Allocation { stage: "coaches" })?;
                output
                    .try_reserve(1)
                    .map_err(|_| GroupingError::Allocation { stage: "coaches" })?;
                positions.insert(observation.id.as_str(), output.len());
                output.push(observation.clone());
            }
            Ok(())
        })?;
    Ok(output)
}

pub(in crate::workbook) fn bounded_contacts(
    coaches: &[CanonicalCoach],
    school_year: SchoolYear,
    budget: GroupBudget,
) -> ReportResult<BTreeMap<String, SchoolContacts>> {
    check_group_budget(coaches, budget)?;
    Ok(contacts(coaches, school_year))
}

#[cfg(test)]
#[path = "bounded/tests.rs"]
mod tests;
