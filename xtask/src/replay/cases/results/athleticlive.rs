use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{Context, Result};
use census_crawl::athleticlive;
use census_crawl::athleticlive_athletes::{AthleteHit, HitTeam};
use std::collections::BTreeSet;

const OBSERVED_ON: &str = "2026-09-20";

const MEETS_CSV_LABEL: &str = "athleticlive_meets_csv";

pub(super) fn athlete_hits(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    let envelope: serde_json::Value =
        serde_json::from_str(body).with_context(|| format!("{file} is not JSON"))?;
    let sources: Vec<serde_json::Value> = envelope
        .pointer("/hits/hits")
        .and_then(serde_json::Value::as_array)
        .map(|hits| {
            hits.iter()
                .map(|hit| {
                    hit.get("_source")
                        .cloned()
                        .map_or(serde_json::Value::Null, core::convert::identity)
                })
                .collect()
        })
        .map_or(Default::default(), core::convert::identity);
    let hits: Vec<AthleteHit> = serde_json::from_value(serde_json::Value::Array(sources))
        .with_context(|| format!("{file}: the recorded hits do not decode"))?;
    ensure_rows(file, hits.len(), "athlete rows")?;
    let meets: BTreeSet<u64> = hits.iter().filter_map(AthleteHit::meet_id).collect();
    let athletes: BTreeSet<u64> = hits
        .iter()
        .filter_map(AthleteHit::athletic_net_athlete_id)
        .collect();
    let teams: BTreeSet<u64> = hits
        .iter()
        .filter_map(|hit| hit.t.as_ref()?.athletic_net_team_id())
        .collect();
    let cross_country = hits
        .iter()
        .filter(|hit| hit.t.as_ref().is_some_and(HitTeam::is_cross_country))
        .count();
    let grade_tokens = hits.iter().filter(|hit| hit.y.is_some()).count();
    Ok(format!(
        "athlete_hits rows={} meets={meets:?} athletic_net_athletes={} athletic_net_teams={} \
         cross_country={cross_country} grade_tokens={grade_tokens}",
        hits.len(),
        athletes.len(),
        teams.len()
    ))
}

pub(super) fn event_document(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    let Some(event_id) = event_id(file) else {
        return unmapped("athleticlive_results", file);
    };
    let doc = athleticlive::parse_event_document(&athleticlive::event_doc_url(event_id), body)?;
    ensure_rows(file, doc.rows.len(), "result rows")?;
    Ok(format!(
        "event_doc event_id={:?} meet_id={:?} label={:?} kind={:?} rows={}",
        doc.event_id(),
        doc.meet_id(),
        doc.label(),
        doc.kind(),
        doc.rows.len()
    ))
}

pub(super) fn meets_csv(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    let rows = athleticlive::parse_meets_csv(body)?;
    ensure_rows(file, rows.len(), "meet rows")?;
    let meets = athleticlive::build_meets(&rows, OBSERVED_ON, MEETS_CSV_LABEL);
    Ok(format!(
        "meets_csv rows={} meets={}",
        rows.len(),
        meets.len()
    ))
}

fn event_id(file: &str) -> Option<u64> {
    file.strip_suffix(".json")?.rsplit('-').next()?.parse().ok()
}
