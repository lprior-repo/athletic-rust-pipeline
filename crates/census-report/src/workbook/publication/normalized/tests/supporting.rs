use super::{field, fixture::Population, read, TestResult};
use census_domain::model::CanonicalPerformance;
use std::collections::BTreeSet;

fn population(with_evidence: bool) -> TestResult<Population> {
    let mut given = Population::new("Ada Runner", &["50.00", "51.00"])?;
    given.dataset.lineage.policy_revision = 20;
    let mut supporting = given
        .dataset
        .performances
        .iter()
        .find(|row| row.source_key == "result-0")
        .ok_or("missing selected performance")?
        .clone();
    supporting.source_key = "independent-supporting-publication".into();
    supporting.id = CanonicalPerformance::mint(
        &supporting.athlete,
        &supporting.meet,
        &supporting.event,
        &supporting.date,
        &supporting.source_key,
    );
    if !with_evidence {
        supporting.evidence.clear();
        supporting.source_athlete = None;
    }
    given.dataset.performances.push(supporting);
    given.bests = crate::bests::build_from_dataset(
        &given.dataset,
        &crate::bests::Options {
            scope: given.options.scope,
            grad_year: given.options.grad_year,
            limit: given.options.limit,
            jurisdictions: given.options.jurisdictions.clone(),
        },
    )?;
    Ok(given)
}

#[test]
fn normalized_v4_writer_enumerates_all_supporting_ids_and_excludes_worse_candidates() -> TestResult
{
    let given = population(true)?;
    let directory = tempfile::tempdir()?;
    super::super::write(
        directory.path(),
        &given.dataset,
        &given.options,
        &given.bests,
        &[],
    )?;
    let marks = read(directory.path(), "best_marks.csv")?;
    let best = marks.first().ok_or("missing best")?;
    let ids: BTreeSet<String> =
        serde_json::from_str(field(best, "supporting_tied_candidate_ids")?)?;
    check!(eq; ids.len(), 2);
    check!(eq; field(best, "tied_performance_count")?, "1");
    check!(eq; field(best, "eligible_candidate_count")?, "2");
    let provenance = read(directory.path(), "provenance.csv")?;
    let references = provenance.iter().filter(|row| {
        row.get("evidence_type").map(String::as_str) == Some("retained_performance_reference")
    });
    let owners = references
        .map(|row| field(row, "entity_id").map(str::to_owned))
        .collect::<TestResult<BTreeSet<_>>>()?;
    check!(eq; owners, ids);
    let worse = given
        .dataset
        .performances
        .iter()
        .find(|row| row.source_key == "result-1")
        .ok_or("missing worse candidate")?;
    check!(!provenance
        .iter()
        .any(|row| row.get("entity_id").map(String::as_str) == Some(worse.id.as_str())));
    Ok(())
}

#[test]
fn normalized_v4_writer_keeps_reference_without_source_evidence_and_is_order_stable() -> TestResult
{
    let mut given = population(false)?;
    let first = tempfile::tempdir()?;
    let second = tempfile::tempdir()?;
    super::super::write(
        first.path(),
        &given.dataset,
        &given.options,
        &given.bests,
        &[],
    )?;
    given.dataset.performances.reverse();
    given.bests = crate::bests::build_from_dataset(
        &given.dataset,
        &crate::bests::Options {
            scope: given.options.scope,
            grad_year: given.options.grad_year,
            limit: given.options.limit,
            jurisdictions: given.options.jurisdictions.clone(),
        },
    )?;
    super::super::write(
        second.path(),
        &given.dataset,
        &given.options,
        &given.bests,
        &[],
    )?;
    for file in ["best_marks.csv", "provenance.csv"] {
        check!(eq; std::fs::read(first.path().join(file))?, std::fs::read(second.path().join(file))?);
    }
    let supporting = given
        .dataset
        .performances
        .iter()
        .find(|row| row.source_key == "independent-supporting-publication")
        .ok_or("missing supporting candidate")?;
    let provenance = read(first.path(), "provenance.csv")?;
    let references = provenance
        .iter()
        .filter(|row| row.get("entity_id").map(String::as_str) == Some(supporting.id.as_str()))
        .collect::<Vec<_>>();
    check!(eq; references.len(), 1);
    let reference = references.first().ok_or("missing retained reference")?;
    check!(eq; field(reference, "evidence_type")?, "retained_performance_reference");
    for column in [
        "capture_id",
        "capture_time",
        "interpretation_version",
        "source_url",
    ] {
        check!(eq; field(reference, column)?, "unavailable");
    }
    Ok(())
}
