//! The Athletic.net arm: the adapter's three published documents, decoded with its own wire types.
//!
//! The documents are told apart by the file-name suffixes the corpus names them with
//! (`_allresults.json`, `_eventdiv.json`, `_meetdata`), the same suffixes the parity harnesses select
//! fixtures by, so a renamed capture is an unhandled file rather than a silently empty decode.

use crate::replay::{unmapped, Capture};
use anyhow::{Context, Result};
use midwest_census::sources::athleticnet::{AllResults, EventDivisions, MeetData};

/// An Athletic.net document, decoded with the adapter's own published wire types - the read
/// `athleticnet_meet_parity` makes of these captures.
pub(super) fn document(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file.ends_with("_allresults.json") {
        let results: AllResults = decode(file, "results document", body)?;
        let rows: usize = results.blocks.iter().map(|block| block.results.len()).sum();
        return Ok(format!(
            "all_results blocks={} rows={} legs={} teams={}",
            results.blocks.len(),
            rows,
            results.legs.len(),
            results.teams.len()
        ));
    }
    if file.ends_with("_eventdiv.json") {
        let divisions: EventDivisions = decode(file, "divisions document", body)?;
        return Ok(format!("event_divisions events={}", divisions.events.len()));
    }
    if file.contains("_meetdata") {
        let data: MeetData = decode(file, "meet document", body)?;
        return Ok(format!(
            "meet_data id={} name={:?} date={} divisions={}",
            data.meet.id,
            data.meet.name,
            data.meet.date,
            data.divisions.len()
        ));
    }
    unmapped("athleticnet", file)
}

/// Decode one published document into its wire type; a body that is not that document is an error
/// naming the file, never an empty parse.
fn decode<T: serde::de::DeserializeOwned>(file: &str, what: &str, body: &str) -> Result<T> {
    serde_json::from_str(body).with_context(|| format!("{file}: the {what} did not decode"))
}
