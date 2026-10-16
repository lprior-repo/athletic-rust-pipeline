use super::{csv_rows, fields, Expected, TestResult};
use census_domain::model::{serialized_digest, CanonicalPerformance};
use serde_json::Value;
use std::{collections::BTreeSet, path::Path};

pub(super) fn retained(directory: &Path, expected: &Expected) -> TestResult {
    let frozen: Value =
        serde_json::from_slice(&std::fs::read(directory.join("frozen-input.json"))?)?;
    let performances: Vec<CanonicalPerformance> = serde_json::from_value(
        frozen
            .pointer("/data/performances")
            .ok_or("frozen performances")?
            .clone(),
    )?;
    check!(eq; performances.len(), 2);
    let provenance = csv_rows(&directory.join("provenance.csv"))?;
    for ((source, capture_url), case) in [&expected.eligible, &expected.excluded]
        .into_iter()
        .zip(&expected.result_capture_urls)
        .zip(&super::super::fixture::CASES)
    {
        let retained = performances
            .iter()
            .find(|row| row.id == source.id)
            .ok_or("retained original performance")?;
        check!(eq; retained, source);
        check!(eq; source.evidence.len(), 3);
        result_sources(source, capture_url, case)?;
        let mut identifiers = BTreeSet::new();
        for fact in &source.evidence {
            let id = serialized_digest(&("performance", source.id.as_str(), fact))?;
            check!(identifiers.insert(id.clone()));
            let matching = provenance
                .iter()
                .filter(|row| row.get("evidence_id").map(String::as_str) == Some(id.as_str()))
                .collect::<Vec<_>>();
            if source.id == expected.eligible.id {
                check!(eq; matching.len(), 1);
                fields(
                    matching.first().ok_or("exact normalized source fact")?,
                    &[
                        ("entity_type", "performance"),
                        ("entity_id", source.id.as_str()),
                        (
                            "source_url",
                            fact.source.url.as_deref().ok_or("source fact URL")?,
                        ),
                        ("input_generation", &expected.input_generation),
                    ],
                )?;
                let retained_fact: census_domain::model::Evidence = serde_json::from_str(
                    matching
                        .first()
                        .and_then(|row| row.get("retained_fact"))
                        .ok_or("retained fact bytes")?,
                )?;
                check!(eq; &retained_fact, fact);
            } else {
                check!(matching.is_empty());
            }
            let note: Value = serde_json::from_str(fact.note.as_deref().ok_or("source note")?)?;
            if note.get("role").and_then(Value::as_str) == Some("independent_participation_v1") {
                check!(eq; fact.source.id, "mhsaa_synthetic_test");
                check!(eq; fact.observed_on, "2026-10-01T10:00:00Z");
                let key = note
                    .get("packet")
                    .and_then(Value::as_str)
                    .ok_or("independent packet pointer")?;
                check!(expected.packets.contains_key(key));
            } else {
                check!(eq; fact.source.id, "milesplit_mi");
                check!(eq; fact.observed_on, "2026-10-01T23:44:16Z");
            }
        }
    }
    check!(eq; expected.packets.len(), 2);
    Ok(())
}

fn result_sources(
    source: &CanonicalPerformance,
    capture_url: &str,
    case: &super::super::fixture::ResultCase,
) -> TestResult {
    let raw_url = case.url();
    check!(ne; capture_url, raw_url.as_str());
    let structured = source
        .evidence
        .first()
        .ok_or("structured result evidence")?;
    check!(eq; structured.source.url.as_deref(), Some(capture_url));
    let note: Value =
        serde_json::from_str(structured.note.as_deref().ok_or("structured source note")?)?;
    check!(eq; note.get("capture_url").and_then(Value::as_str), Some(capture_url));
    check!(eq; note.get("raw_metadata_url").and_then(Value::as_str), Some(raw_url.as_str()));
    let metadata: Vec<_> = source
        .evidence
        .iter()
        .filter(|fact| fact.source.url.as_deref() == Some(raw_url.as_str()))
        .collect();
    check!(eq; metadata.len(), 1);
    let metadata = metadata
        .first()
        .ok_or("separate raw meet metadata evidence")?;
    let note: Value = serde_json::from_str(metadata.note.as_deref().ok_or("raw metadata note")?)?;
    for (pointer, value) in [
        ("/role", "raw result file meet/season metadata only"),
        ("/owned_capture/url", capture_url),
        ("/raw_metadata_capture/url", raw_url.as_str()),
    ] {
        check!(eq; note.pointer(pointer).and_then(Value::as_str), Some(value));
    }
    Ok(())
}
