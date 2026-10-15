use super::super::{files, read, TestResult};
use super::fixture::{Positive, DAY};
use super::rows::{digest, json, strings};
use census_domain::model::Evidence;
use std::path::Path;

const HEADER: [&str; 15] = [
    "evidence_id",
    "entity_type",
    "entity_id",
    "source_namespace",
    "source_url",
    "source_reference",
    "capture_id",
    "capture_time",
    "interpretation_version",
    "evidence_type",
    "observed_on",
    "capture_status",
    "interpretation_status",
    "input_generation",
    "retained_fact",
];

pub(super) fn compare_and_replace(given: &Positive, directory: &Path) -> TestResult {
    let (header, mut actual) = read(directory, "provenance.csv")?;
    check!(eq; header, strings(&HEADER));
    let expected = expected(given)?;
    check!(eq; expected.len(), 4);
    for expected in expected {
        let id = expected.first().ok_or("missing evidence key")?;
        let row = actual
            .iter_mut()
            .find(|row| row.first() == Some(id))
            .ok_or("missing positive evidence row")?;
        check!(eq; row.as_slice(), expected.as_slice());
        *row = expected;
    }
    files::put(directory, "provenance.csv", &HEADER, &actual)
}

fn expected(given: &Positive) -> TestResult<Vec<Vec<String>>> {
    let generation = &given.population.dataset.lineage.input_generation;
    let mut records = Vec::new();
    for coach in &given.population.coaches {
        records.push(evidence(
            "coach",
            coach.id.as_str(),
            coach.evidence.first().ok_or("missing coach evidence")?,
            generation,
        )?);
    }
    records.push(evidence(
        "school_address",
        &digest(&given.address)?,
        given.address.evidence(),
        generation,
    )?);
    records.push(evidence(
        "athlete",
        given.canonical().as_str(),
        given.members[0]
            .evidence
            .first()
            .ok_or("missing roster evidence")?,
        generation,
    )?);
    Ok(records)
}

fn evidence(kind: &str, owner: &str, fact: &Evidence, generation: &str) -> TestResult<Vec<String>> {
    Ok(strings(&[
        &digest(&(kind, owner, fact))?,
        kind,
        owner,
        "unavailable",
        fact.source
            .url
            .as_deref()
            .ok_or("missing positive evidence URL")?,
        &fact.source.id,
        "unavailable",
        "unavailable",
        "unavailable",
        "\"parsed\"",
        DAY,
        "capture_link_not_retained",
        "interpretation_version_not_retained",
        generation,
        &json(fact)?,
    ]))
}
