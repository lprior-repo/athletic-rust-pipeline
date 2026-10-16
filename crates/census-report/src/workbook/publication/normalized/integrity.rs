use super::{comparison_classes, defect, PerformanceIndex};
use crate::bests::{Parents, SharedSelection};
use crate::export::public_identity::population::IdentityPopulation;
use crate::report::{Derivation, EventAdmission, ReportResult};
use census_domain::model::performance_comparison::PrKey;
use census_domain::model::CanonicalPerformance;
use std::collections::BTreeSet;

mod ties;

pub(super) fn validate<'a>(
    derivation: &Derivation<'a>,
    admission: &EventAdmission<'_>,
    bests: &[SharedSelection],
) -> ReportResult<PerformanceIndex<'a>> {
    let dataset = derivation.dataset();
    admission.ensure_dataset(dataset)?;
    let mut budget = input_budget(derivation)?;
    unique_ids(dataset)?;
    parents(derivation)?;
    let performances: PerformanceIndex<'_> = derivation
        .performances()
        .iter()
        .copied()
        .map(|row| (&row.id, row))
        .collect();
    let parents = Parents::of(derivation);
    let mut keys = BTreeSet::new();
    for best in bests {
        budget.record(best)?;
        budget.record(&best.supporting_tied_candidate_ids())?;
        validate_best(best, derivation, admission, &performances, &parents)?;
        let key = (best.athlete_id().as_str(), comparison_classes::id(best)?);
        if !keys.insert(key) {
            return Err(defect("duplicate athlete/comparison-class best key"));
        }
    }
    Ok(performances)
}

fn input_budget(derivation: &Derivation<'_>) -> ReportResult<crate::grouping::Admission> {
    let dataset = derivation.dataset();
    let mut budget = crate::grouping::Admission::new("normalized input records");
    budget.rows(&dataset.athletes)?;
    dataset
        .schools
        .values()
        .try_for_each(|school| budget.record(school))?;
    budget.rows(&dataset.meets)?;
    budget.rows(derivation.coach_observations())?;
    budget.rows(&dataset.performances)?;
    budget.rows(&dataset.identity_decisions)?;
    budget.rows(&dataset.coverage_evidence.rows)?;
    budget.rows(&dataset.coverage_evidence.coach_lanes)?;
    budget.rows(&dataset.source_access)?;
    Ok(budget)
}

fn unique_ids(dataset: &crate::export::ExportDataset) -> ReportResult<()> {
    unique(
        dataset.athletes.iter().map(|row| row.id.as_str()),
        "athlete",
    )?;
    unique(
        dataset.performances.iter().map(|row| row.id.as_str()),
        "performance",
    )?;
    unique(dataset.meets.iter().map(|row| row.id.as_str()), "meet")?;
    unique(
        dataset
            .coverage_evidence
            .rows
            .iter()
            .map(|row| row.id.as_str()),
        "coverage",
    )?;
    unique(
        dataset
            .coverage_evidence
            .coach_lanes
            .iter()
            .map(|row| row.id()),
        "coach lane",
    )?;
    unique(
        dataset.source_access.iter().map(|row| row.id.as_str()),
        "source access",
    )
}

fn unique<'a>(ids: impl Iterator<Item = &'a str>, kind: &str) -> ReportResult<()> {
    let mut seen = BTreeSet::new();
    for id in ids {
        if id.is_empty() || !seen.insert(id) {
            return Err(defect(format!("empty or duplicate {kind} identifier {id}")));
        }
    }
    Ok(())
}

fn parents(derivation: &Derivation<'_>) -> ReportResult<()> {
    for athlete in derivation.athletes() {
        if !derivation.dataset().schools.contains_key(&athlete.school) {
            return Err(defect(format!(
                "athlete {} references missing school {}",
                athlete.id, athlete.school
            )));
        }
    }
    let schools: BTreeSet<_> = derivation
        .schools()
        .iter()
        .map(|school| &school.id)
        .collect();
    for coach in derivation.coach_observations() {
        if !schools.contains(&coach.school) {
            return Err(defect(format!(
                "coach {} references unpublished school {}",
                coach.id, coach.school
            )));
        }
    }
    Ok(())
}

fn validate_best(
    best: &SharedSelection,
    derivation: &Derivation<'_>,
    admission: &EventAdmission<'_>,
    performances: &PerformanceIndex<'_>,
    parents: &Parents<'_>,
) -> ReportResult<()> {
    let dataset = derivation.dataset();
    let athlete = parents.athlete(best.athlete_id().as_str()).ok_or_else(|| {
        defect(format!(
            "best references unpublished athlete {}",
            best.athlete_id()
        ))
    })?;
    let performance = performances
        .get(&best.source.performance_id)
        .copied()
        .ok_or_else(|| {
            defect(format!(
                "best references missing performance {}",
                best.source.performance_id
            ))
        })?;
    if &athlete.id != best.athlete_id()
        || IdentityPopulation::of(dataset)?.canonical_candidate(performance.athlete.as_str())
            != Some(athlete.id.as_str())
        || performance.mark != best.result.mark
        || best.measure().value(&performance.mark) != Some(best.result.value)
        || athlete.gender != best.athlete.gender
        || athlete.grad_year.get() != best.athlete.grad_year
        || performance.meet != best.meet.meet_id
        || !matches_key(performance, best, admission, parents)?
        || !source_matches(performance, best, derivation, admission, parents)?
    {
        return Err(defect(format!(
            "selection differs from admitted performance {}",
            performance.id
        )));
    }
    historical_affiliation(best, performance, dataset)?;
    ties::validate(best, derivation, admission, performances, parents)
}

fn historical_affiliation(
    best: &SharedSelection,
    performance: &CanonicalPerformance,
    dataset: &crate::export::ExportDataset,
) -> ReportResult<()> {
    let school = dataset
        .teams
        .get(&performance.team)
        .and_then(|team| dataset.schools.get(&team.school));
    let name = school.map(|school| school.name.as_str());
    let state = census_domain::JurisdictionBucket::from(school.and_then(|school| school.state));
    if best.athlete.school.as_deref() != name
        || best.athlete.athlete_school != name.map_or("", core::convert::identity)
        || best.athlete.athlete_state != state
    {
        return Err(defect(format!(
            "selection historical affiliation differs from performance {}",
            performance.id
        )));
    }
    Ok(())
}

fn matches_key(
    performance: &CanonicalPerformance,
    best: &SharedSelection,
    admission: &EventAdmission<'_>,
    parents: &Parents<'_>,
) -> ReportResult<bool> {
    if admission.included_kind(&performance.event).is_none() {
        return Ok(false);
    }
    let event = admission
        .event(&performance.event)
        .ok_or_else(|| defect("admitted event disappeared"))?;
    let Some(mut key) = PrKey::from_performance(
        performance,
        event,
        parents.meet(performance.meet.as_str()),
        best.measure(),
        admission.semantics(),
    )
    .map_err(|source| crate::report::ReportError::Catalog {
        event: event.id.clone(),
        source,
    })?
    else {
        return Ok(false);
    };
    key.athlete_id = best.athlete_id().clone();
    Ok(
        key == best.key
            || key.same_course(event, admission.semantics()).as_ref() == Some(&best.key),
    )
}

fn source_matches(
    performance: &CanonicalPerformance,
    best: &SharedSelection,
    derivation: &Derivation<'_>,
    admission: &EventAdmission<'_>,
    parents: &Parents<'_>,
) -> ReportResult<bool> {
    let event = admission
        .event(&performance.event)
        .ok_or_else(|| defect("missing selected event"))?;
    let specification = event
        .resolved_specification()
        .map_err(|error| defect(format!("selected event specification: {error}")))?;
    let url = derivation
        .scope()
        .primary_evidence(performance)
        .and_then(|fact| fact.source.url.as_deref())
        .map_or("", core::convert::identity);
    Ok(best.source.specification == specification
        && best.source.source_key == performance.source_key
        && best.source.result_url == url
        && best.meet.date == performance.date
        && parents
            .meet(performance.meet.as_str())
            .is_some_and(|meet| meet.name == best.meet.name))
}

#[cfg(test)]
mod tests;
