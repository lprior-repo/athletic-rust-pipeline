use crate::report::ReportResult;
use census_domain::model::SchoolYear;
use census_store::Entity;
use serde::Deserialize;
use std::path::Path;

use super::input::Inputs;
use super::read;
use super::{defect, excerpt};

const MAX_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Audit {
    input_generation: String,
    scope: String,
    grad_year: Option<i16>,
    school_year: SchoolYear,
    athletes: Vec<String>,
    performances: Vec<String>,
    coaches: Vec<String>,
    identity_decisions: Vec<String>,
}

pub(super) fn verify(directory: &Path, inputs: &Inputs<'_>) -> ReportResult<()> {
    let path = directory.join("audit.json");
    let found: Audit = read::json_as(&path, MAX_BYTES, "audit.json")?;
    verify_scope(&path, inputs, &found)?;
    verify_years(inputs, &found)?;
    verify_population(&path, inputs, &found)?;
    verify_decisions(&path, inputs, &found)
}

fn verify_scope(path: &Path, inputs: &Inputs<'_>, found: &Audit) -> ReportResult<()> {
    let derivation = &inputs.derivation;
    expect_text(
        path,
        "input_generation",
        &inputs.dataset.lineage.input_generation,
        &found.input_generation,
    )?;
    expect_text(path, "scope", derivation.scope().as_str(), &found.scope)
}

fn verify_years(inputs: &Inputs<'_>, found: &Audit) -> ReportResult<()> {
    let derivation = &inputs.derivation;
    if found.grad_year != derivation.grad_year() {
        return Err(defect(format!(
            "audit.json grad_year: expected {:?}, found {:?}",
            derivation.grad_year(),
            found.grad_year
        )));
    }
    if found.school_year != inputs.school_year {
        return Err(defect(format!(
            "audit.json school_year: expected {}, found {}",
            inputs.school_year.get(),
            found.school_year.get()
        )));
    }
    Ok(())
}

fn verify_population(path: &Path, inputs: &Inputs<'_>, found: &Audit) -> ReportResult<()> {
    let derivation = &inputs.derivation;
    expect_ids(
        path,
        "athletes",
        &entity_ids(derivation.athletes()),
        &found.athletes,
    )?;
    expect_ids(
        path,
        "performances",
        &derivation
            .performances()
            .iter()
            .map(|row| row.id.as_str().to_owned())
            .collect::<Vec<_>>(),
        &found.performances,
    )
}

fn verify_decisions(path: &Path, inputs: &Inputs<'_>, found: &Audit) -> ReportResult<()> {
    let derivation = &inputs.derivation;
    expect_ids(
        path,
        "coaches",
        &entity_ids(derivation.coach_observations()),
        &found.coaches,
    )?;
    expect_ids(
        path,
        "identity_decisions",
        &entity_ids(&inputs.dataset.identity_decisions),
        &found.identity_decisions,
    )
}

fn entity_ids<T: Entity>(rows: &[T]) -> Vec<String> {
    rows.iter().map(|row| row.entity_id().to_owned()).collect()
}

fn expect_text(path: &Path, field: &str, expected: &str, found: &str) -> ReportResult<()> {
    if expected == found {
        Ok(())
    } else {
        Err(defect(format!(
            "{} {field}: expected {:?}, found {:?}",
            path.display(),
            excerpt(expected),
            excerpt(found)
        )))
    }
}

fn expect_ids(path: &Path, field: &str, expected: &[String], found: &[String]) -> ReportResult<()> {
    if expected.len() != found.len() {
        return Err(defect(format!(
            "{} {field} holds {} ids where the frozen selection holds {}",
            path.display(),
            found.len(),
            expected.len()
        )));
    }
    for (index, (expected, found)) in expected.iter().zip(found).enumerate() {
        if expected != found {
            return Err(defect(format!(
                "{} {field}[{index}]: expected {:?}, found {:?}",
                path.display(),
                excerpt(expected),
                excerpt(found)
            )));
        }
    }
    Ok(())
}
