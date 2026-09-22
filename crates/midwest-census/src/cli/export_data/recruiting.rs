//! Recruiting projection: Co2027 athlete x school x coach.

use crate::cli::export_data::{csv::write_csv, helpers::*};
use serde_json::Value;
use std::collections::HashMap;

/// Build coach index: school -> sport:role -> best coach record.
pub(super) fn build_coach_index(coaches: &[Value]) -> HashMap<&str, HashMap<String, &Value>> {
    let mut coach_index: HashMap<&str, HashMap<String, &Value>> = HashMap::new();
    for c in coaches {
        let sch = c.get("school").and_then(|v| v.as_str()).unwrap_or("");
        let sport = c.get("sport").and_then(|v| v.as_str()).unwrap_or("school");
        let role = c.get("role").and_then(|v| v.as_str()).unwrap_or("unknown");
        let key = format!("{sport}:{role}");

        let entry = coach_index.entry(sch).or_default();
        if let Some(prior) = entry.get(&key) {
            let has_email = c
                .get("professional_email")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false);
            let prior_has_email = prior
                .get("professional_email")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false);
            if has_email && !prior_has_email {
                entry.insert(key, c);
            }
        } else {
            entry.insert(key, c);
        }
    }
    coach_index
}

/// Coach lookup results for an athlete's school.
struct SchoolCoaches<'a> {
    track: Option<&'a Value>,
    xc: Option<&'a Value>,
    ad: Option<&'a Value>,
    has_email: bool,
    has_coach: bool,
}

fn lookup_coaches<'a>(
    coach_index: &'a HashMap<&'a str, HashMap<String, &'a Value>>,
    school_id: &'a str,
) -> SchoolCoaches<'a> {
    let track = find_coach(
        coach_index,
        school_id,
        &["outdoor_track:head_coach", "indoor_track:head_coach"],
    );
    let xc = find_coach(coach_index, school_id, &["cross_country:head_coach"]);
    let ad = find_coach(
        coach_index,
        school_id,
        &[
            "school:athletic_director",
            "outdoor_track:athletic_director",
        ],
    );

    let emails: Vec<&str> = [&track, &xc, &ad]
        .iter()
        .filter_map(|r| r.and_then(|c| c.get("professional_email").and_then(|v| v.as_str())))
        .collect();
    let has_email = emails.iter().any(|e| !e.is_empty());
    let has_coach = track.is_some() || xc.is_some();

    SchoolCoaches {
        track,
        xc,
        ad,
        has_email,
        has_coach,
    }
}

/// Find a coach record for a given index and key priority list.
fn find_coach<'a>(
    index: &'a HashMap<&str, HashMap<String, &Value>>,
    school_id: &str,
    keys: &[&str],
) -> Option<&'a Value> {
    for key in keys {
        if let Some(entry) = index.get(school_id) {
            if let Some(record) = entry.get(*key) {
                return Some(record);
            }
        }
    }
    None
}

/// Get the first non-empty source URL from a coach record.
fn coach_source_url(record: Option<&Value>) -> String {
    record
        .and_then(|c| {
            c.get("evidence")
                .and_then(|v| v.as_array())
                .and_then(|evidence| {
                    evidence.iter().find_map(|e| {
                        e.get("source")
                            .and_then(|s| s.get("url"))
                            .and_then(|u| u.as_str())
                            .filter(|u| !u.is_empty())
                    })
                })
        })
        .unwrap_or("")
        .to_string()
}

/// Extract coach name from an optional Value.
fn coach_name(c: Option<&Value>) -> String {
    c.and_then(|c| c.get("name").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string()
}

/// Extract coach email from an optional Value.
fn coach_email(c: Option<&Value>) -> String {
    c.and_then(|c| c.get("professional_email").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string()
}

/// Get a school field as string.
fn school_str(sch: &Value, field: &str) -> String {
    sch.get(field)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Extract a string field from a Value.
fn field_str(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Build a single recruiting row from an athlete and pre-looked-up school coaches.
fn build_recruit_row(a: &Value, sch: &Value, coaches: &SchoolCoaches) -> Vec<String> {
    let source_url = coach_source_url(coaches.track.or(coaches.xc).or(coaches.ad));

    vec![
        field_str(a, "canonical_name"),
        a.get("grad_year")
            .and_then(|v| v.as_i64())
            .map(|g| g.to_string())
            .unwrap_or_default(),
        field_str(a, "gender"),
        school_str(sch, "state"),
        school_str(sch, "name"),
        school_str(sch, "city"),
        sports(a),
        extract_an_url(a),
        extract_ms_url(a),
        coach_name(coaches.track),
        coach_email(coaches.track),
        coach_name(coaches.xc),
        coach_email(coaches.xc),
        coach_name(coaches.ad),
        coach_email(coaches.ad),
        school_str(sch, "athletics_website"),
        source_url,
        a.get("identity_confidence")
            .map_or(String::new(), |v| v.to_string()),
        sources(a),
    ]
}

/// Extract athletic.net athlete URL from public_profile_urls.
fn extract_an_url(a: &Value) -> String {
    a.get("public_profile_urls")
        .and_then(|v| v.as_array())
        .and_then(|urls| {
            urls.iter()
                .find_map(|u| u.as_str().filter(|s| s.contains("athletic.net/athlete/")))
        })
        .unwrap_or("")
        .to_string()
}

/// Extract milesplit athlete URL from public_profile_urls.
fn extract_ms_url(a: &Value) -> String {
    a.get("public_profile_urls")
        .and_then(|v| v.as_array())
        .and_then(|urls| {
            urls.iter()
                .find_map(|u| u.as_str().filter(|s| s.contains("milesplit.com/athletes/")))
        })
        .unwrap_or("")
        .to_string()
}

/// Build and write recruiting-co2027.csv.
///
/// Returns (with_coach_count, with_email_count).
pub fn write_recruiting(
    athletes: &[Value],
    coach_index: &HashMap<&str, HashMap<String, &Value>>,
    by_school: &HashMap<&str, &Value>,
    data: &std::path::Path,
) -> anyhow::Result<(usize, usize)> {
    let rec_path = data.join("recruiting-co2027.csv");
    let rec_header = [
        "name",
        "grad_year",
        "gender",
        "state",
        "school",
        "school_city",
        "sports",
        "athleticnet_url",
        "milesplit_url",
        "head_track_coach",
        "head_track_coach_email",
        "head_xc_coach",
        "head_xc_coach_email",
        "athletic_director",
        "athletic_director_email",
        "athletics_website",
        "coach_source_url",
        "identity_confidence",
        "evidence_sources",
    ];
    let mut with_coach = 0usize;
    let mut with_email = 0usize;
    let mut rec_rows: Vec<Vec<String>> = Vec::new();

    for a in athletes {
        if a.get("grad_year").and_then(|v| v.as_i64()) != Some(2027) {
            continue;
        }
        let sid = a.get("school").and_then(|v| v.as_str()).unwrap_or("");
        let sch = by_school.get(sid).copied().unwrap_or(&Value::Null);
        let coaches = lookup_coaches(coach_index, sid);

        if coaches.has_email {
            with_email += 1;
        }
        if coaches.has_coach {
            with_coach += 1;
        }

        rec_rows.push(build_recruit_row(a, sch, &coaches));
    }

    write_csv(&rec_path, &rec_header, &rec_rows)?;

    Ok((with_coach, with_email))
}
