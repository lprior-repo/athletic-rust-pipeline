use super::super::{files, population::Population, read, TestResult};
use census_domain::model::{serialized_digest, CanonicalPerformance, PerformanceId};
use std::path::Path;

pub(super) fn population(with_evidence: bool) -> TestResult<(Population, CanonicalPerformance)> {
    let mut given = Population::new()?;
    let mut supporting = given
        .performances
        .first()
        .ok_or("missing selected fixture")?
        .clone();
    let (source_key, id) = independent_key(&supporting)?;
    supporting.source_key = source_key;
    supporting.id = id;
    if !with_evidence {
        supporting.evidence.clear();
    }
    given.dataset.performances.push(supporting.clone());
    Ok((given, supporting))
}

fn independent_key(performance: &CanonicalPerformance) -> TestResult<(String, PerformanceId)> {
    (0..1024)
        .find_map(|ordinal| {
            let key = format!("manual-independent-supporting-result-{ordinal}");
            let id = CanonicalPerformance::mint(
                &performance.athlete,
                &performance.meet,
                &performance.event,
                &performance.date,
                &key,
            );
            (id > performance.id).then_some((key, id))
        })
        .ok_or_else(|| "unable to mint stable later supporting ID".into())
}

pub(super) fn ids(given: &Population, supporting: &CanonicalPerformance) -> Vec<PerformanceId> {
    let mut ids: Vec<_> = given
        .performances
        .iter()
        .take(2)
        .map(|row| row.id.clone())
        .collect();
    ids.push(supporting.id.clone());
    ids.sort();
    ids
}

pub(super) fn write(
    given: &Population,
    supporting: &CanonicalPerformance,
    directory: &Path,
) -> TestResult {
    files::write(given, directory)?;
    let (mut header, mut records) = read(directory, "best_marks.csv")?;
    header.push("supporting_tied_candidate_ids".into());
    records
        .first_mut()
        .ok_or("missing manual best")?
        .push(serde_json::to_string(&ids(given, supporting))?);
    put(directory, "best_marks.csv", &header, &records)?;
    let (header, mut records) = read(directory, "provenance.csv")?;
    for performance in given
        .performances
        .iter()
        .take(2)
        .chain(std::iter::once(supporting))
    {
        records.push(reference(given, performance)?);
    }
    for fact in &supporting.evidence {
        records.push(vec![
            serialized_digest(&("performance", supporting.id.as_str(), fact))?,
            "performance".into(),
            supporting.id.to_string(),
            "unavailable".into(),
            "https://example.test/result".into(),
            "manual-readback".into(),
            "unavailable".into(),
            "unavailable".into(),
            "unavailable".into(),
            serde_json::to_string(&fact.method)?,
            "2026-05-04".into(),
            "capture_link_not_retained".into(),
            "interpretation_version_not_retained".into(),
            given.dataset.lineage.input_generation.clone(),
            serde_json::to_string(fact)?,
        ]);
    }
    put(directory, "provenance.csv", &header, &records)
}

fn reference(given: &Population, performance: &CanonicalPerformance) -> TestResult<Vec<String>> {
    let retained = serde_json::json!({
        "performance_id": performance.id,
        "source_key": performance.source_key,
        "athlete_id": performance.athlete,
        "event_id": performance.event,
        "meet_id": performance.meet,
    });
    Ok(vec![
        serialized_digest(&(
            "retained_performance_reference",
            performance.id.as_str(),
            &retained,
        ))?,
        "performance".into(),
        performance.id.to_string(),
        "unavailable".into(),
        "unavailable".into(),
        performance.source_key.clone(),
        "unavailable".into(),
        "unavailable".into(),
        "unavailable".into(),
        "retained_performance_reference".into(),
        "unavailable".into(),
        "capture_link_not_retained".into(),
        "interpretation_version_not_retained".into(),
        given.dataset.lineage.input_generation.clone(),
        serde_json::to_string(&retained)?,
    ])
}

pub(super) fn put(
    directory: &Path,
    name: &str,
    header: &[String],
    rows: &[Vec<String>],
) -> TestResult {
    files::put(
        directory,
        name,
        &header.iter().map(String::as_str).collect::<Vec<_>>(),
        rows,
    )
}

pub(super) fn column(header: &[String], name: &str) -> TestResult<usize> {
    header
        .iter()
        .position(|field| field == name)
        .ok_or_else(|| format!("missing {name}").into())
}
