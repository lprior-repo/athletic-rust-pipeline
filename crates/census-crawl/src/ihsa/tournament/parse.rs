//! Published-payload decoding: the five tournament payloads become wire rows, and the fields the
//! census reads (marks, grades, round labels, terms) are turned into canonical values.
//!
//! No store access and no canonical entity mapping: rows are decoded here and minted into entities
//! by the mapping layer.
use super::wire::{
    ErrorEnvelope, EventRow, EventSummary, EventsEnvelope, FinisherRow, MeetRow, MeetsEnvelope,
    QualifiersEnvelope, RelayMember, TermsEnvelope,
};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{Grade, Mark};

// ---------------------------------------------------------------------------
// Envelope decoders
// ---------------------------------------------------------------------------

/// Decode one JSON payload, naming it in the error so a run's log says which shape failed.
fn decode<T: serde::de::DeserializeOwned>(body: &str, what: &str) -> CrawlResult<T> {
    serde_json::from_str(body).map_err(|source| CrawlError::Decode {
        url: what.to_string(),
        source,
    })
}

/// Parse `GET /v1/track-field/meets`.
pub fn parse_meets(body: &str) -> CrawlResult<Vec<MeetRow>> {
    let envelope: MeetsEnvelope = decode(body, "IHSA track-field meets index")?;
    Ok(envelope.data)
}

/// Parse `GET /v1/track-field/meets/{year}/events?gender=...`.
pub fn parse_events(body: &str) -> CrawlResult<EventsEnvelope> {
    decode(body, "IHSA track-field events index")
}

/// Parse `GET /v1/track-field/events/{eventId}/summary`.
pub fn parse_summary(body: &str) -> CrawlResult<EventSummary> {
    decode(body, "IHSA track-field event summary")
}

/// Parse `GET /v1/{term}/statefinal/cc-qualifiers?tournamentId={id}`.
///
/// An empty archive (a term with no entered lists) decodes to an envelope with three empty arrays -
/// the measured body for `2024-25` is `{"tournamentId": "691", "boxAssignments": [], ...}`. The
/// archive API's error body is a different shape and fails here; callers check [`parse_error`]
/// first so a missing archive is a note rather than a decode error.
pub fn parse_qualifiers(body: &str) -> CrawlResult<QualifiersEnvelope> {
    decode(body, "IHSA cross-country qualifiers")
}

/// Parse `GET /v1/terms`.
pub fn parse_terms(body: &str) -> CrawlResult<TermsEnvelope> {
    decode(body, "IHSA terms")
}

/// `Some(message)` when the body is the archive API's error envelope
/// (`{"error": "Archive not available for term 2026-27"}`).
pub fn parse_error(body: &str) -> Option<String> {
    let envelope: ErrorEnvelope = serde_json::from_str(body).ok()?;
    Some(envelope.error)
}

/// The newest term the archive API holds: the terms list is descending, so the first row is the
/// newest completed school year. The response's `currentTerm` is the year in progress and has no
/// cross-country archive yet (measured: `2026-27` answers `Archive not available for term 2026-27`).
pub fn newest_term(body: &str) -> CrawlResult<Option<String>> {
    let envelope: TermsEnvelope = parse_terms(body)?;
    Ok(envelope.terms.first().map(|row| row.term.clone()))
}

// ---------------------------------------------------------------------------
// Field parsers
// ---------------------------------------------------------------------------

/// Parse a published mark into a canonical [`Mark`].
///
/// The published forms are the metric field notation with a unit suffix (`"2.02m"`), a running time
/// (`"7:51.37"`, `"10.94"`), and the qualifying suffix a prelim adds (`"1.88mq"`, `"10.94Q"`). A
/// mark with no distance suffix and no digits (`"NH"`, `"NM"`, `"DNF"`, `"FOUL"`) is not a mark and
/// yields `None`; so does an empty or absent value. Nothing is invented for a row that publishes
/// none.
pub fn parse_mark(published: &str) -> Option<Mark> {
    let trimmed = published.trim();
    let stripped = trimmed
        .strip_suffix('q')
        .or_else(|| trimmed.strip_suffix('Q'))
        .unwrap_or(trimmed)
        .trim();
    if stripped.is_empty() {
        return None;
    }
    if let Some(metric) = stripped.strip_suffix('m') {
        let value = metric.trim().parse::<f64>().ok()?;
        return Some(Mark::DistanceMetres(value));
    }
    if let Some((minutes, seconds)) = stripped.split_once(':') {
        let minutes = minutes.trim().parse::<f64>().ok()?;
        let seconds = seconds.trim().parse::<f64>().ok()?;
        return Some(Mark::TimeSeconds(minutes.mul_add(60.0, seconds)));
    }
    stripped.parse::<f64>().ok().map(Mark::TimeSeconds)
}

/// Map a published in-school grade (`"11"`) to a canonical [`Grade`] (9..=12).
///
/// The T&F surfaces publish the grade as `finishers[].year` (and again as
/// `finishers[].members[].athlete.year` for relay legs); the cross-country lists publish it as
/// `athletes[].yearInSchool`. Anything outside 9..=12 is dropped rather than rounded into a scale
/// the payload did not state.
pub fn parse_grade(published: Option<&str>) -> Option<Grade> {
    let level = published?.trim().parse::<u8>().ok()?;
    Grade::new(level)
}

/// The round label both result paths agree on.
///
/// The summary publishes `roundLabel` ("Finals"), but the events index that lists the same rounds
/// publishes only the code, so the label is derived from the code and the index and summary paths
/// stay equal. The summary's own field is deliberately not read.
pub fn round_label(round: Option<&str>) -> Option<&'static str> {
    match round {
        Some("P") => Some("Prelims"),
        Some("F") => Some("Finals"),
        _ => None,
    }
}

/// The grade a finisher row publishes, preferring the row's own `year` over the nested
/// `athlete.year` (the two agree in every captured row).
pub fn finisher_grade(row: &FinisherRow) -> Option<Grade> {
    parse_grade(row.year.as_deref()).or_else(|| parse_grade(row.athlete.as_ref()?.year.as_deref()))
}

/// One relay leg's grade, from `members[].athlete.year` (relay rows carry no row-level grade).
pub fn member_grade(member: &RelayMember) -> Option<Grade> {
    parse_grade(member.athlete.as_ref()?.year.as_deref())
}

/// The class token an event publishes (`"1A"`, `"2A"`, `"3A"`, `"WD"`), as cohort evidence. The
/// token is read from `classDivision`, not from the event name: the four wheelchair events name
/// themselves `"Boys Discus Throw WD Wheelchair"`, which no suffix rule recovers as a class.
pub fn class_token(class_division: &str) -> Option<&str> {
    let token = class_division.trim();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

/// The event label the ontology maps, read from the name both result paths publish.
///
/// The published name is `<gender> <event> <class> - <round>` (`"Boys High Jump 1A - Finals"`), and
/// the class and the round are read from their own fields, so both are stripped here and the index
/// and summary paths yield the same label. A name the rule cannot reduce (`"Boys Discus Throw WD
/// Wheelchair"`, where the class token is a word inside the name) is returned whole rather than
/// mangled, and the ontology files it as `Unmapped` under the name it does publish.
pub fn event_label<'a>(event_name: &'a str, class_division: &str) -> Option<&'a str> {
    let published = event_name.trim();
    if published.is_empty() {
        return None;
    }
    let without_gender = ["Boys ", "Girls "]
        .into_iter()
        .find_map(|prefix| published.strip_prefix(prefix))
        .unwrap_or(published);
    let without_round = without_gender
        .split_once(" - ")
        .map_or(without_gender, |(head, _)| head);
    let class = class_division.trim();
    let without_class = if class.is_empty() {
        without_round
    } else {
        without_round
            .trim_end()
            .strip_suffix(class)
            .map_or(without_round, str::trim_end)
    };
    let label = without_class.trim();
    Some(if label.is_empty() { published } else { label })
}

/// The date part of a published ISO timestamp (`"2026-05-28T00:00:00.000Z"` -> `"2026-05-28"`).
pub fn date_part(timestamp: &str) -> Option<&str> {
    let (date, _) = timestamp.split_once('T')?;
    if date.is_empty() {
        None
    } else {
        Some(date)
    }
}

/// The date range the event rows of one meet span, as `(earliest, latest)`.
pub fn event_date_range(rows: &[EventRow]) -> (Option<String>, Option<String>) {
    let mut dates: Vec<String> = rows
        .iter()
        .filter_map(|row| row.scheduled_date.as_deref())
        .filter_map(date_part)
        .map(str::to_string)
        .collect();
    dates.sort();
    (dates.first().cloned(), dates.last().cloned())
}
