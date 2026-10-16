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
    for source in [&expected.eligible, &expected.excluded] {
        let retained = performances
            .iter()
            .find(|row| row.id == source.id)
            .ok_or("retained original performance")?;
        check!(eq; retained, source);
        check!(eq; source.evidence.len(), 3);
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
