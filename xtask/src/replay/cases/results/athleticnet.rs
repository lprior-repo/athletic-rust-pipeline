
use crate::replay::{unmapped, Capture};
use anyhow::{Context, Result};
use census_crawl::athleticnet::{AllResults, EventDivisions, MeetData};

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

fn decode<T: serde::de::DeserializeOwned>(file: &str, what: &str, body: &str) -> Result<T> {
    serde_json::from_str(body).with_context(|| format!("{file}: the {what} did not decode"))
}
