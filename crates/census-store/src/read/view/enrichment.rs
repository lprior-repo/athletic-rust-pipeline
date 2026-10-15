#[path = "enrichment/stream.rs"]
mod stream;
pub(crate) use stream::RetainedEnrichment;

use std::collections::HashMap;

use census_domain::model::{CanonicalAthlete, ObservedGrade, SourceNamespace, SourceObservation};

use super::super::{ReadBudget, ReadCounts, StoreSnapshot};
use crate::{StoreError, StoreResult, Table};

#[path = "enrichment/budget.rs"]
mod budget;
use budget::Admission;

struct TargetEntry<'a> {
    grades: &'a mut Vec<ObservedGrade>,
    initial_len: usize,
}

type Owners<'a> = HashMap<&'a SourceNamespace, HashMap<&'a str, Vec<TargetEntry<'a>>>>;

struct Enrichment<'a> {
    owners: Owners<'a>,
    admission: Admission,
}

impl StoreSnapshot<'_> {
    pub fn enrich_athletes(&self, athletes: &mut [CanonicalAthlete]) -> StoreResult<()> {
        let mut enrichment = Enrichment::new(athletes, Admission::Legacy)?;
        let result =
            self.for_each_observation(Table::SourceObservations, |row| enrichment.accept(&row));
        if result.is_err() {
            enrichment.rollback();
        }
        result
    }

    pub fn enrich_athletes_bounded(
        &self,
        athletes: &mut [CanonicalAthlete],
        budget: ReadBudget,
    ) -> StoreResult<ReadCounts> {
        let mut enrichment = Enrichment::new(athletes, Admission::bounded(budget))?;
        let result =
            self.for_each_observation_bounded(Table::SourceObservations, budget, |row, _| {
                enrichment.accept(&row)
            });
        if result.is_err() {
            enrichment.rollback();
        }
        result
    }
}

pub(crate) fn enrich_retained_observations(
    athletes: &mut [CanonicalAthlete],
    observations: &[SourceObservation],
    budget: ReadBudget,
) -> StoreResult<()> {
    let mut enrichment = Enrichment::new(athletes, Admission::bounded(budget))?;
    let result = observations
        .iter()
        .try_for_each(|row| enrichment.accept(row));
    if result.is_err() {
        enrichment.rollback();
    }
    result
}

impl<'a> Enrichment<'a> {
    fn new(athletes: &'a mut [CanonicalAthlete], admission: Admission) -> StoreResult<Self> {
        admission.owners(athletes.len())?;
        let mut owners: Owners<'a> = HashMap::new();
        owners.try_reserve(athletes.len()).map_err(allocation)?;
        for athlete in athletes {
            let Some(source) = &athlete.source else {
                continue;
            };
            let by_id = owners.entry(&source.namespace).or_default();
            by_id.try_reserve(1).map_err(allocation)?;
            let targets = by_id.entry(source.id.as_str()).or_default();
            targets.try_reserve(1).map_err(allocation)?;
            let initial_len = athlete.observed_grades.len();
            targets.push(TargetEntry {
                grades: &mut athlete.observed_grades,
                initial_len,
            });
        }
        Ok(Self { owners, admission })
    }

    fn rollback(&mut self) {
        for namespace in self.owners.values_mut() {
            for targets in namespace.values_mut() {
                for target in targets {
                    target.grades.truncate(target.initial_len);
                }
            }
        }
    }

    fn accept(&mut self, row: &SourceObservation) -> StoreResult<()> {
        let SourceObservation::Athlete(row) = row else {
            return Ok(());
        };
        let Some(grade) = &row.observed_grade else {
            return Ok(());
        };
        let Some(targets) = self
            .owners
            .get_mut(&row.namespace)
            .and_then(|namespace| namespace.get_mut(row.source_athlete_id.as_str()))
        else {
            return Ok(());
        };
        let count = targets
            .iter()
            .filter(|target| !target.grades.contains(grade))
            .count();
        self.admission.additions(grade, count)?;
        targets
            .iter_mut()
            .filter(|target| !target.grades.contains(grade))
            .try_for_each(|target| target.grades.try_reserve(1).map_err(allocation))?;
        let mut unmatched = targets
            .iter_mut()
            .filter(|target| !target.grades.contains(grade));
        if let Some(first) = unmatched.next() {
            for target in unmatched {
                target.grades.push(grade.clone());
            }
            first.grades.push(grade.clone());
        }
        Ok(())
    }
}

fn allocation(error: std::collections::TryReserveError) -> StoreError {
    StoreError::Refused {
        detail: format!("allocating athlete grade enrichment: {error}"),
    }
}
