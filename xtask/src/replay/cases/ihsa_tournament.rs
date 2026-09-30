use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::Result;
use census_crawl::ihsa::tournament;

pub(super) fn ihsa_tournament(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file == "track_field_meets.json" {
        let meets = tournament::parse::parse_meets(body)?;
        ensure_rows(file, meets.len(), "meets")?;
        return Ok(format!("meets rows={}", meets.len()));
    }
    if file == "track_field_2026_boys_events.json" {
        let events = tournament::parse::parse_events(body)?;
        ensure_rows(file, events.data.len(), "events")?;
        return Ok(format!(
            "events meet_id={} counted={} rows={}",
            events.meet_id,
            events.count,
            events.data.len()
        ));
    }
    if file.starts_with("event_") {
        let summary = tournament::parse::parse_summary(body)?;
        return Ok(format!(
            "event_summary event_id={} has_results={} finishers={}",
            summary.event_id,
            summary.has_results,
            summary.finishers.len()
        ));
    }
    if file.starts_with("cc_qualifiers_") {
        if let Some(error) = tournament::parse::parse_error(body) {
            return Ok(format!("cc_qualifiers archive_error={error:?}"));
        }
        let qualifiers = tournament::parse::parse_qualifiers(body)?;
        return Ok(format!(
            "cc_qualifiers tournament_id={} boxes={} teams={} individuals={}",
            qualifiers.tournament_id,
            qualifiers.box_assignments.len(),
            qualifiers.team_qualifiers.len(),
            qualifiers.individual_qualifiers.len()
        ));
    }
    if file == "terms.json" {
        let terms = tournament::parse::parse_terms(body)?;
        ensure_rows(file, terms.terms.len(), "terms")?;
        return Ok(format!(
            "terms current={} terms={}",
            terms.current_term,
            terms.terms.len()
        ));
    }
    unmapped("ihsa_tournament", file)
}
