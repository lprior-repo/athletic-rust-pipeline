use std::collections::HashMap;

use census_domain::model::{CanonicalAthlete, ObservedGrade, SourceNamespace, SourceObservation};

use super::super::{ReadBudget, ReadCounts, StoreSnapshot};
use crate::{StoreError, StoreResult, Table};

#[path = "enrichment/budget.rs"]
mod budget;
use budget::Admission;

type Owners<'a> = HashMap<&'a SourceNamespace, HashMap<&'a str, Vec<&'a mut Vec<ObservedGrade>>>>;

struct Enrichment<'a> {
    owners: Owners<'a>,
    admission: Admission,
}

impl StoreSnapshot<'_> {
    pub fn enrich_athletes(&self, athletes: &mut [CanonicalAthlete]) -> StoreResult<()> {
        let mut enrichment = Enrichment::new(athletes, Admission::Legacy)?;
        self.for_each_observation(Table::SourceObservations, |row| enrichment.accept(row))
    }

    pub fn enrich_athletes_bounded(
        &self,
        athletes: &mut [CanonicalAthlete],
        budget: ReadBudget,
    ) -> StoreResult<ReadCounts> {
        let mut enrichment = Enrichment::new(athletes, Admission::bounded(budget))?;
        self.for_each_observation_bounded(Table::SourceObservations, budget, |row, _| {
            enrichment.accept(row)
        })
    }
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
            targets.push(&mut athlete.observed_grades);
        }
        Ok(Self { owners, admission })
    }

    fn accept(&mut self, row: SourceObservation) -> StoreResult<()> {
        let SourceObservation::Athlete(row) = row else {
            return Ok(());
        };
        let Some(grade) = row.observed_grade else {
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
            .filter(|grades| !grades.contains(&grade))
            .count();
        self.admission.additions(&grade, count)?;
        targets
            .iter_mut()
            .filter(|grades| !grades.contains(&grade))
            .try_for_each(|grades| grades.try_reserve(1).map_err(allocation))?;
        let mut unmatched = targets.iter_mut().filter(|grades| !grades.contains(&grade));
        if let Some(first) = unmatched.next() {
            for grades in unmatched {
                grades.push(grade.clone());
            }
            first.push(grade);
        }
        Ok(())
    }
}

fn allocation(error: std::collections::TryReserveError) -> StoreError {
    StoreError::Refused {
        detail: format!("allocating athlete grade enrichment: {error}"),
    }
}
