//! The AthleticLIVE arms: a recorded athlete-batch envelope, a published event document, and the
//! harvest's meets CSV.
//!
//! The three captures share one adapter and one naming habit: the event document's file name carries
//! the event id its URL is built from ([`event_id`]), and the meets CSV is the only capture the verb
//! builds canonical meets from, which is why the two constants the build needs live here.

use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::{Context, Result};
use census_crawl::athleticlive;
use census_crawl::athleticlive_athletes::{AthleteHit, HitTeam};
use std::collections::BTreeSet;

/// The capture date the corpus's harnesses stamp rows with (`parity_*::OBSERVED_ON`).
const OBSERVED_ON: &str = "2026-09-20";

/// The source label `athleticlive::build_meets` stamps its meets with, as its own fixtures do.
const MEETS_CSV_LABEL: &str = "athleticlive_meets_csv";

/// One AthleticLIVE athlete batch response: the recorded Elasticsearch envelope, decoded through
/// the adapter's own published row type.
///
/// The envelope unwrap (`hits.hits[]._source`) is the same two operations `batches::page_sources`
/// performs and `sources/athleticlive_athletes/tests.rs` repeats inline; the decode itself is
/// `AthleteHit`, the type the adapter's `page_hits` decodes, so this arm re-runs the adapter's read
/// rather than a second parser beside it. A capture whose envelope drifts decodes to nothing and
/// fails here by name.
///
/// Entity minting is out of reach for this fixture: `build_entities` needs each row's `MeetTarget`,
/// whose tenant, name, state and date come from the harvest CSV, and this fixture directory carries
/// none. The arm reports what the rows themselves publish.
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
                        .unwrap_or(serde_json::Value::Null)
                })
                .collect()
        })
        .unwrap_or_default();
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

/// One AthleticLIVE event document: the reader takes the URL the document was published at, which
/// the adapter's own route builder derives from the event id the capture's file name carries.
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

/// The AthleticLIVE meets CSV: its rows, then the canonical meets the adapter builds from them.
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

/// The event id a capture's file name carries (`event-doc-2150205.json` → `2150205`).
fn event_id(file: &str) -> Option<u64> {
    file.strip_suffix(".json")?.rsplit('-').next()?.parse().ok()
}
