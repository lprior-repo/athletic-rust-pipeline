//! Athletes CSV export: canonical-athletes-co2027.csv and athleticnet-athlete-seeds.csv.

use crate::cli::export_data::helpers::*;
use anyhow::Context;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};

/// Build observed_grades string from an athlete record.
fn build_observed(a: &Value) -> String {
    if let Some(grades) = a.get("observed_grades").and_then(|v| v.as_array()) {
        let mut items: Vec<String> = grades
            .iter()
            .filter_map(|o| {
                let grade = o
                    .get("grade")
                    .and_then(|g| g.as_i64())
                    .map(|g| g.to_string())
                    .unwrap_or_default();
                let year = o
                    .get("school_year")
                    .and_then(|y| y.as_i64())
                    .map(|y| y.to_string())
                    .unwrap_or_default();
                if !grade.is_empty() && !year.is_empty() {
                    Some(format!("g{grade}@{year}"))
                } else {
                    None
                }
            })
            .collect();
        items.sort();
        items.dedup();
        items.join(";")
    } else {
        String::new()
    }
}

/// Build a seeds row from athlete data.
fn build_seed_row(an_id: &str, an_url: &str, ms_id: &str, a: &Value, sch: &Value) -> Vec<String> {
    let an_url_final = if !an_url.is_empty() {
        an_url.to_string()
    } else {
        format!("https://www.athletic.net/athlete/{}/track-and-field", an_id)
    };
    let observed_on = a
        .get("evidence")
        .and_then(|v| v.as_array())
        .and_then(|evidence| {
            evidence
                .iter()
                .filter_map(|e| e.get("observed_on").and_then(|o| o.as_str()))
                .max()
        })
        .unwrap_or("");
    vec![
        an_id.to_string(),
        an_url_final,
        field_str(a, "canonical_name"),
        grad_year_str(a),
        field_str(a, "gender"),
        school_str(sch, "state"),
        school_str(sch, "name"),
        school_str(sch, "city"),
        sports(a),
        sources(a),
        ms_id.to_string(),
        observed_on.to_string(),
    ]
}

/// Get school lookup for a school ID.
fn get_school<'a>(by_school: &'a HashMap<&str, &'a Value>, school_id: &str) -> &'a Value {
    by_school.get(school_id).copied().unwrap_or(&Value::Null)
}

/// Write a CSV file from header and rows.
fn write_csv_file(
    path: &std::path::Path,
    header: &[&str],
    rows: &[Vec<String>],
) -> anyhow::Result<()> {
    let p = path.display();
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_path(path)
        .with_context(|| format!("creating csv at {p}"))?;
    writer.write_record(header)?;
    for row in rows {
        writer.write_record(row)?;
    }
    writer.flush()?;
    Ok(())
}

/// Write the seeds CSV file.
fn write_seeds_csv(path: &std::path::Path, rows: &[Vec<String>]) -> anyhow::Result<()> {
    write_csv_file(
        path,
        &[
            "athleticnet_athlete_id",
            "athleticnet_url",
            "name",
            "grad_year",
            "gender",
            "state",
            "school_name",
            "city",
            "sports",
            "derived_from_sources",
            "milesplit_athlete_id",
            "observed_on",
        ],
        rows,
    )
}

/// Process a single athlete for seeds and co2027 rows.
struct AthleteProcess {
    multi: bool,
    is_co27: bool,
    an_id: String,
    an_url: String,
    ms_id: String,
    ms_url: String,
    namespaces: Vec<String>,
    observed: String,
}

/// Process one athlete: extract fields and check multi-source.
fn process_athlete(a: &Value) -> AthleteProcess {
    let idents = identities(a);
    let namespaces: Vec<String> = idents
        .iter()
        .map(|(k, _, _)| k.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let multi = namespaces.len() > 1;
    let an_id = pick(a, "legacy_athletic_net", Some("athlete"));
    let an_url = a
        .get("public_profile_urls")
        .and_then(|v| v.as_array())
        .and_then(|urls| {
            urls.iter()
                .find_map(|u| u.as_str().filter(|s| s.contains("athletic.net/athlete/")))
        })
        .unwrap_or("");
    let ms_id = pick(a, "milesplit_athlete", None);
    let ms_url = a
        .get("public_profile_urls")
        .and_then(|v| v.as_array())
        .and_then(|urls| {
            urls.iter()
                .find_map(|u| u.as_str().filter(|s| s.contains("milesplit.com/athletes/")))
        })
        .unwrap_or("");
    let observed = build_observed(a);
    let is_co27 = a.get("grad_year").and_then(|v| v.as_i64()) == Some(2027);
    AthleteProcess {
        multi,
        is_co27,
        an_id,
        an_url: an_url.to_string(),
        ms_id,
        ms_url: ms_url.to_string(),
        namespaces,
        observed,
    }
}

/// Build a co2027 row from athlete data.
fn build_co2027_row(a: &Value, sch: &Value, process: AthleteProcess) -> Vec<String> {
    let AthleteProcess {
        an_id,
        an_url,
        ms_id,
        ms_url,
        namespaces,
        observed,
        ..
    } = process;
    let profile_urls: String =
        if let Some(urls) = a.get("public_profile_urls").and_then(|v| v.as_array()) {
            urls.iter()
                .filter_map(|u| u.as_str())
                .collect::<Vec<_>>()
                .join(";")
        } else {
            String::new()
        };
    vec![
        a.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        field_str(a, "canonical_name"),
        grad_year_str(a),
        field_str(a, "gender"),
        school_str(sch, "state"),
        field_str(a, "school"),
        school_str(sch, "name"),
        school_str(sch, "city"),
        sports(a),
        a.get("identity_confidence")
            .map_or(String::new(), |v| v.to_string()),
        an_id,
        an_url,
        ms_id,
        ms_url,
        profile_urls,
        namespaces.join(";"),
        sources(a),
        observed,
    ]
}

/// Helper to extract a string field from a Value.
fn field_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}
fn grad_year_str(v: &Value) -> String {
    v.get("grad_year")
        .and_then(|v| v.as_i64())
        .map(|g| g.to_string())
        .unwrap_or_default()
}
fn school_str(sch: &Value, field: &str) -> String {
    sch.get(field)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Build and write co2027 and seeds CSV files.
///
/// Returns (co27_count, multi_source_count).
pub fn write_athletes(
    athletes: &[Value],
    by_school: &HashMap<&str, &Value>,
    data: &std::path::Path,
) -> anyhow::Result<(usize, usize)> {
    let header = [
        "athlete_id",
        "name",
        "grad_year",
        "gender",
        "state",
        "school_id",
        "school_name",
        "school_city",
        "sports",
        "identity_confidence",
        "athleticnet_athlete_id",
        "athleticnet_url",
        "milesplit_athlete_id",
        "milesplit_url",
        "profile_urls",
        "source_namespaces",
        "evidence_sources",
        "observed_grades",
    ];
    let co2027_path = data.join("canonical-athletes-co2027.csv");
    let seeds_path = data.join("athleticnet-athlete-seeds.csv");
    let mut co27 = 0usize;
    let mut multi = 0usize;
    let mut co2027_rows: Vec<Vec<String>> = Vec::new();
    let mut seeds_rows: Vec<Vec<String>> = Vec::new();
    for a in athletes {
        let p = process_athlete(a);
        if p.multi {
            multi = multi.saturating_add(1);
        }
        if !p.an_id.is_empty() {
            let sch = get_school(
                by_school,
                a.get("school").and_then(|v| v.as_str()).unwrap_or(""),
            );
            seeds_rows.push(build_seed_row(&p.an_id, &p.an_url, &p.ms_id, a, sch));
        }
        if !p.is_co27 {
            continue;
        }
        co27 = co27.saturating_add(1);
        let sch = get_school(
            by_school,
            a.get("school").and_then(|v| v.as_str()).unwrap_or(""),
        );
        co2027_rows.push(build_co2027_row(a, sch, p));
    }
    write_csv_file(&co2027_path, &header, &co2027_rows)?;
    write_seeds_csv(&seeds_path, &seeds_rows)?;
    Ok((co27, multi))
}
