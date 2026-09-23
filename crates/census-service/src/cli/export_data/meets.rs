//! Canonical meets CSV export.

use crate::cli::export_data::{csv::write_csv, helpers::*};
use serde_json::Value;

/// Build a single meet row from a JSON Value.
fn build_meet_row(m: &Value) -> Vec<String> {
    let an = pick(m, "legacy_athletic_net", Some("meet"));
    let an_url = m
        .get("source_urls")
        .and_then(|v| v.as_array())
        .and_then(|urls| {
            urls.iter()
                .find_map(|u| u.as_str())
                .filter(|u| u.contains("athletic.net/meet/"))
        })
        .unwrap_or("");
    let id = m.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let state = m.get("state").and_then(|v| v.as_str()).unwrap_or("");
    let date = m.get("date").and_then(|v| v.as_str()).unwrap_or("");
    let end_date = m.get("end_date").and_then(|v| v.as_str()).unwrap_or("");
    let name = m.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let location = m.get("location").and_then(|v| v.as_str()).unwrap_or("");
    let level = m.get("level").and_then(|v| v.as_str()).unwrap_or("");
    let sports_str = sports(m);
    let ident_count = m
        .get("source_identities")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let evidence_src = sources(m);

    vec![
        id.to_string(),
        state.to_string(),
        date.to_string(),
        end_date.to_string(),
        name.to_string(),
        location.to_string(),
        level.to_string(),
        sports_str,
        an,
        an_url.to_string(),
        ident_count.to_string(),
        evidence_src,
    ]
}

/// Build and write canonical-meets.csv.
pub fn write_canonical_meets(meets: &[Value], data: &std::path::Path) -> anyhow::Result<()> {
    let meet_rows: Vec<Vec<String>> = meets.iter().map(build_meet_row).collect();

    write_csv(
        &data.join("canonical-meets.csv"),
        &[
            "meet_id",
            "state",
            "date",
            "end_date",
            "name",
            "location",
            "level",
            "sports",
            "athleticnet_meet_id",
            "athleticnet_url",
            "source_identities",
            "evidence_sources",
        ],
        &meet_rows,
    )?;

    Ok(())
}
