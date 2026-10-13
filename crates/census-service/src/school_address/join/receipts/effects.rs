use census_domain::model::{CanonicalSchool, ReviewCase, ReviewState, SchoolId};
use census_store::{StoreError, StoreResult, Table};
use serde::Serialize;

use super::budget::TargetBudget;
use super::{digest, reserve};

pub(super) struct TargetEffects<'a> {
    pub(super) schools: Vec<&'a CanonicalSchool>,
    pub(super) cases: Vec<&'a ReviewCase>,
    budget: TargetBudget,
}

impl<'a> TargetEffects<'a> {
    pub(super) fn new(
        target: &SchoolId,
        schools: &'a [CanonicalSchool],
        cases: &'a [ReviewCase],
    ) -> StoreResult<Self> {
        let mut budget = TargetBudget::default();
        let schools = canonical_schools(target, schools, &mut budget)?;
        let cases = canonical_cases(target, cases, &mut budget)?;
        Ok(Self {
            schools,
            cases,
            budget,
        })
    }

    pub(super) fn admit_witness<T: Serialize>(&self, witness: &T) -> StoreResult<()> {
        self.budget.admit_witness(witness)
    }

    pub(super) fn admit_application<T: Serialize>(
        &self,
        witness: &T,
        operation: &str,
        digest: &str,
    ) -> StoreResult<()> {
        self.budget.admit_application(witness, operation, digest)
    }
}

fn canonical_schools<'a>(
    target: &SchoolId,
    schools: &'a [CanonicalSchool],
    budget: &mut TargetBudget,
) -> StoreResult<Vec<&'a CanonicalSchool>> {
    let mut unique: Vec<(&CanonicalSchool, String)> = Vec::new();
    for school in schools {
        if &school.id != target {
            return Err(invalid_target(
                target,
                "school payload belongs to another target",
            ));
        }
        if unique.iter().any(|(held, _)| *held == school) {
            continue;
        }
        budget.admit_row(Table::Schools, school, school.id.as_str())?;
        let payload_digest = digest(school)?;
        if unique.iter().any(|(_, held)| held == &payload_digest) {
            return Err(invalid_target(
                target,
                "distinct school payloads share a digest",
            ));
        }
        reserve(&mut unique, 1)?;
        unique.push((school, payload_digest));
    }
    let mut ordered = Vec::new();
    reserve(&mut ordered, unique.len())?;
    ordered.extend(unique.into_iter().map(|(school, _)| school));
    Ok(ordered)
}

fn canonical_cases<'a>(
    target: &SchoolId,
    cases: &'a [ReviewCase],
    budget: &mut TargetBudget,
) -> StoreResult<Vec<&'a ReviewCase>> {
    let mut unique: Vec<&ReviewCase> = Vec::new();
    for case in cases {
        if case.subject_id != target.as_str() || case.state != ReviewState::Pending {
            return Err(invalid_target(
                target,
                "review payload is not this target's Pending case",
            ));
        }
        if let Some(held) = unique.iter().find(|held| held.id == case.id) {
            if *held != case {
                return Err(invalid_target(
                    target,
                    "one review id names different payloads",
                ));
            }
            continue;
        }
        budget.admit_row(Table::ReviewCases, case, &case.id)?;
        reserve(&mut unique, 1)?;
        unique.push(case);
    }
    Ok(unique)
}

fn invalid_target(target: &SchoolId, detail: &str) -> StoreError {
    StoreError::Invariant {
        detail: format!("school-address target {}: {detail}", target.as_str()),
    }
}
