use super::{csv_rows, fields, Expected, TestResult};
use census_domain::model::serialized_digest;
use serde_json::Value;
use std::{collections::BTreeSet, path::Path};

pub(super) fn artifacts(directory: &Path, expected: &Expected) -> TestResult {
    let [eligible_source_url, _] = &expected.result_capture_urls;
    let normalized_ties = serde_json::to_string(&[expected.eligible.id.as_str()])?;
    let csv = csv_rows(&directory.join("best-results-co2027.csv"))?;
    check!(eq; csv.len(), 1);
    fields(
        csv.first().ok_or("eligible selected CSV result")?,
        &[
            ("athlete_id", &expected.public_id),
            ("canonical_candidate_id", expected.eligible.athlete.as_str()),
            ("best_mark", "13-9"),
            ("best_value", "4191000"),
            ("best_value_unit", "um"),
            ("performance_id", expected.eligible.id.as_str()),
            ("source_key", "milesplit_result:201782263"),
            ("date", "2026-03-27"),
            ("school_at_performance", "Detroit Catholic Central"),
            ("state_at_performance", "MI"),
            ("current_school_id", &expected.school_id),
            ("current_school", "Detroit Catholic Central"),
            ("current_state", "MI"),
            ("current_school_as_of", "2026-10-01"),
            ("eligible_result_group_count", "1"),
            ("tied_result_group_count", "1"),
            (
                "tied_result_group_representative_ids",
                expected.eligible.id.as_str(),
            ),
            ("result_group_basis", "canonical_performance_id"),
            ("physical_performance_count_status", "unknown"),
            ("selection_policy_version", "22"),
        ],
    )?;
    let normalized = csv_rows(&directory.join("best_marks.csv"))?;
    check!(eq; normalized.len(), 1);
    let refs = expected
        .eligible
        .evidence
        .iter()
        .map(|fact| serialized_digest(&("performance", expected.eligible.id.as_str(), fact)))
        .collect::<Result<BTreeSet<_>, _>>()?;
    fields(
        normalized.first().ok_or("eligible normalized result")?,
        &[
            ("athlete_id", &expected.public_id),
            ("canonical_candidate_id", expected.eligible.athlete.as_str()),
            ("performance_id", expected.eligible.id.as_str()),
            ("raw_mark", "13-9"),
            ("display_mark", "13-9"),
            ("normalized_mark", "4191000"),
            ("unit", "um"),
            ("source_url", eligible_source_url),
            ("meet_date", "2026-03-27"),
            ("school_at_performance", &expected.school_id),
            ("meet_id", expected.eligible.meet.as_str()),
            ("meet_name", "Synthetic Michigan March Meet"),
            ("evidence_refs", &serde_json::to_string(&refs)?),
            ("input_generation", &expected.input_generation),
            ("eligible_result_group_count", "1"),
            ("tied_result_group_count", "1"),
            ("tied_result_group_representative_ids", &normalized_ties),
            ("result_group_basis", "canonical_performance_id"),
            ("physical_performance_count_status", "unknown"),
            ("selection_policy_version", "22"),
        ],
    )?;
    let classes = csv_rows(&directory.join("comparison_classes.csv"))?;
    check!(eq; classes.len(), 1);
    check!(eq; normalized.first().and_then(|row| row.get("comparison_class_id")),
        classes.first().and_then(|row| row.get("comparison_class_id")));
    jsonl(directory, expected)
}

fn jsonl(directory: &Path, expected: &Expected) -> TestResult {
    let [eligible_source_url, _] = &expected.result_capture_urls;
    let bytes = std::fs::read_to_string(directory.join("best-results-co2027.jsonl"))?;
    let rows = bytes
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<Result<Vec<_>, _>>()?;
    check!(eq; rows.len(), 1);
    let row = rows.first().ok_or("eligible selected JSONL")?;
    for (pointer, value) in [
        ("/schema", "selected_identity_v2"),
        ("/athlete_id", expected.public_id.as_str()),
        (
            "/canonical_candidate_id",
            expected.eligible.athlete.as_str(),
        ),
        ("/selection/performance_id", expected.eligible.id.as_str()),
        ("/selection/source_key", "milesplit_result:201782263"),
        ("/selection/result_url", eligible_source_url.as_str()),
        ("/selection/date", "2026-03-27"),
        ("/selection/meet_id", expected.eligible.meet.as_str()),
        ("/selection/meet", "Synthetic Michigan March Meet"),
        ("/selection/athlete", "Synthetic Participant"),
        ("/current_school_id", expected.school_id.as_str()),
        ("/current_school", "Detroit Catholic Central"),
        ("/current_state", "MI"),
        ("/current_school_as_of", "2026-10-01"),
        ("/selection/result_group_basis", "canonical_performance_id"),
        ("/selection/physical_performance_count_status", "unknown"),
    ] {
        check!(eq; row.pointer(pointer).and_then(Value::as_str), Some(value));
    }
    for (pointer, value) in [
        ("/selection/value", 4_191_000),
        ("/selection/eligible_result_group_count", 1),
        ("/selection/tied_result_group_count", 1),
        ("/selection/selection_policy_version", 22),
    ] {
        check!(eq; row.pointer(pointer).and_then(Value::as_i64), Some(value));
    }
    check!(eq; row.pointer("/selection/normalized").and_then(Value::as_f64), Some(4.191));
    check!(eq; row.pointer("/selection/mark"), Some(&serde_json::to_value(&expected.eligible.mark)?));
    check!(eq; row.pointer("/selection/tied_result_group_representative_ids"),
        Some(&serde_json::to_value([expected.eligible.id.as_str()])?));
    Ok(())
}
