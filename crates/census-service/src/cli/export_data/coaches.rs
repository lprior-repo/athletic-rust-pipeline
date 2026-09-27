
use crate::cli::export_data::{csv::write_csv, helpers::*};
use serde_json::Value;
use std::collections::HashMap;

fn coach_url(c: &Value) -> String {
    c.get("evidence")
        .and_then(|v| v.as_array())
        .and_then(|evidence| {
            evidence.iter().find_map(|e| {
                e.get("source")
                    .and_then(|s| s.get("url"))
                    .and_then(|u| u.as_str())
            })
        })
        .unwrap_or("")
        .to_string()
}

fn coach_observed(c: &Value) -> String {
    c.get("evidence")
        .and_then(|v| v.as_array())
        .and_then(|evidence| {
            evidence
                .iter()
                .filter_map(|e| e.get("observed_on").and_then(|o| o.as_str()))
                .max()
        })
        .unwrap_or("")
        .to_string()
}

fn school_field(sch: &Value, field: &str) -> String {
    sch.get(field)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn build_coach_row(c: &Value, by_school: &HashMap<&str, &Value>) -> Vec<String> {
    let sch = by_school
        .get(c.get("school").and_then(|v| v.as_str()).unwrap_or(""))
        .copied()
        .unwrap_or(&Value::Null);

    vec![
        field_str(c, "id"),
        field_str(c, "name"),
        field_str(c, "role"),
        field_str(c, "sport"),
        field_str(c, "gender"),
        field_str(c, "school"),
        school_field(sch, "name"),
        school_field(sch, "state"),
        field_str(c, "professional_email"),
        field_str(c, "personal_email"),
        coach_url(c),
        sources(c),
        coach_observed(c),
    ]
}

fn field_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

pub fn write_canonical_coaches(
    coaches: &[Value],
    by_school: &HashMap<&str, &Value>,
    data: &std::path::Path,
) -> anyhow::Result<()> {
    let coach_rows: Vec<Vec<String>> = coaches
        .iter()
        .map(|c| build_coach_row(c, by_school))
        .collect();

    write_csv(
        &data.join("canonical-coaches.csv"),
        &[
            "coach_id",
            "name",
            "role",
            "sport",
            "gender",
            "school_id",
            "school_state",
            "professional_email",
            "personal_email",
            "source_url",
            "evidence_sources",
            "observed_on",
        ],
        &coach_rows,
    )?;

    Ok(())
}
