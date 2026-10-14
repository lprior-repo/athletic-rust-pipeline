use super::super::input::Inputs;
use crate::report::ReportResult;
use std::collections::BTreeSet;

pub(super) fn verify(inputs: &Inputs<'_>) -> ReportResult<()> {
    let dataset = inputs.dataset;
    let mut budget = crate::grouping::Admission::new("normalized readback inputs");
    budget.rows(&dataset.athletes)?;
    dataset
        .schools
        .values()
        .try_for_each(|row| budget.record(row))?;
    dataset
        .teams
        .values()
        .try_for_each(|row| budget.record(row))?;
    budget.rows(&dataset.meets)?;
    budget.rows(&dataset.performances)?;
    budget.rows(inputs.derivation.coach_observations())?;
    budget.rows(&dataset.coverage_evidence.rows)?;
    budget.rows(&dataset.coverage_evidence.coach_lanes)?;
    budget.rows(&dataset.source_access)?;
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

fn unique<'a>(mut ids: impl Iterator<Item = &'a str>, kind: &str) -> ReportResult<()> {
    ids.try_fold(BTreeSet::new(), |mut seen, id| {
        if id.is_empty() || !seen.insert(id) {
            return Err(super::super::defect(format!(
                "normalized input duplicate or empty {kind} key {id}"
            )));
        }
        Ok(seen)
    })
    .map(|_| ())
}
