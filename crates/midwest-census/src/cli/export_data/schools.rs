//! Canonical schools CSV export.

use crate::cli::export_data::{csv::write_csv, helpers::*};
use serde_json::Value;
use std::collections::BTreeSet;

/// Compute identity fields (source namespaces, evidence sources, identity count) for a school.
fn build_identity_fields(s: &Value) -> (String, String, usize) {
    let source_ns: String = identities(s)
        .iter()
        .map(|(k, _, _)| k.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(";");
    let evidence_src = sources(s);
    let ident_count = s
        .get("source_identities")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    (source_ns, evidence_src, ident_count)
}

/// Build a single school row from a JSON Value.
fn build_school_row(s: &Value) -> Vec<String> {
    let id = s.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let state = s.get("state").and_then(|v| v.as_str()).unwrap_or("");
    let city = s.get("city").and_then(|v| v.as_str()).unwrap_or("");
    let association = s.get("association").and_then(|v| v.as_str()).unwrap_or("");
    let classification = s
        .get("classification")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let enrollment = match s.get("enrollment") {
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    };
    let co_op = s.get("co_op").and_then(|v| v.as_bool()).unwrap_or(false);
    let athletics_website = s
        .get("athletics_website")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let school_website = s
        .get("school_website")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let an_team = pick(s, "legacy_athletic_net", Some("team"));
    let ms_school = pick(s, "milesplit_school", None);
    let aliases = if let Some(aliases) = s.get("aliases").and_then(|v| v.as_array()) {
        aliases
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>()
            .join(";")
    } else {
        String::new()
    };
    let (source_ns, evidence_src, ident_count) = build_identity_fields(s);

    vec![
        id.to_string(),
        name.to_string(),
        state.to_string(),
        city.to_string(),
        association.to_string(),
        classification.to_string(),
        enrollment,
        if co_op {
            "true".to_string()
        } else {
            "false".to_string()
        },
        athletics_website.to_string(),
        school_website.to_string(),
        an_team,
        ms_school,
        aliases,
        source_ns,
        evidence_src,
        ident_count.to_string(),
    ]
}

/// Sort schools by state then name.
fn sort_schools(mut schools: Vec<Value>) -> Vec<Value> {
    schools.sort_by(|a, b| {
        let sa = a
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let sa_name = a
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let sb = b
            .get("state")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let sb_name = b
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        (sa, sa_name).cmp(&(sb, sb_name))
    });
    schools
}

/// Build and write canonical-schools.csv.
pub fn write_canonical_schools(schools: &[Value], data: &std::path::Path) -> anyhow::Result<()> {
    let sorted_schools = sort_schools(schools.to_vec());
    let school_rows: Vec<Vec<String>> = sorted_schools.iter().map(build_school_row).collect();

    write_csv(
        &data.join("canonical-schools.csv"),
        &[
            "school_id",
            "name",
            "state",
            "city",
            "association",
            "classification",
            "enrollment",
            "co_op",
            "athletics_website",
            "school_website",
            "athleticnet_team_id",
            "milesplit_school_id",
            "aliases",
            "source_namespaces",
            "evidence_sources",
            "identity_count",
        ],
        &school_rows,
    )?;

    Ok(())
}
