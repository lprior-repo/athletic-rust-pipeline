use super::wire::{
    ErrorEnvelope, EventRow, EventSummary, EventsEnvelope, FinisherRow, MeetRow, MeetsEnvelope,
    QualifiersEnvelope, RelayMember, TermsEnvelope,
};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{CentiMetres, CentiSeconds};
use census_domain::model::{Grade, Mark};

fn decode<T: serde::de::DeserializeOwned>(body: &str, what: &str) -> CrawlResult<T> {
    serde_json::from_str(body).map_err(|source| CrawlError::Decode {
        url: what.to_string(),
        source,
    })
}

pub fn parse_meets(body: &str) -> CrawlResult<Vec<MeetRow>> {
    let envelope: MeetsEnvelope = decode(body, "IHSA track-field meets index")?;
    Ok(envelope.data)
}

pub fn parse_events(body: &str) -> CrawlResult<EventsEnvelope> {
    decode(body, "IHSA track-field events index")
}

pub fn parse_summary(body: &str) -> CrawlResult<EventSummary> {
    decode(body, "IHSA track-field event summary")
}

pub fn parse_qualifiers(body: &str) -> CrawlResult<QualifiersEnvelope> {
    decode(body, "IHSA cross-country qualifiers")
}

pub fn parse_terms(body: &str) -> CrawlResult<TermsEnvelope> {
    decode(body, "IHSA terms")
}

pub fn parse_error(body: &str) -> Option<String> {
    let envelope: ErrorEnvelope = serde_json::from_str(body).ok()?;
    Some(envelope.error)
}

pub fn newest_term(body: &str) -> CrawlResult<Option<String>> {
    let envelope: TermsEnvelope = parse_terms(body)?;
    Ok(envelope.terms.first().map(|row| row.term.clone()))
}

pub fn parse_mark(published: &str) -> Option<Mark> {
    let trimmed = published.trim();
    let stripped = trimmed
        .strip_suffix('q')
        .or_else(|| trimmed.strip_suffix('Q'))
        .map_or(trimmed, |value| value)
        .trim();
    if stripped.is_empty() {
        return None;
    }
    if let Some(metric) = stripped.strip_suffix('m') {
        let value = metric.trim().parse::<f64>().ok()?;
        return Some(Mark::DistanceMetres(CentiMetres::try_from_metres_f64(
            value,
        )?));
    }
    if let Some((minutes, seconds)) = stripped.split_once(':') {
        let minutes = minutes.trim().parse::<f64>().ok()?;
        let seconds = seconds.trim().parse::<f64>().ok()?;
        return Some(Mark::TimeSeconds(CentiSeconds::try_from_seconds_f64(
            minutes.mul_add(60.0, seconds),
        )?));
    }
    stripped
        .parse::<f64>()
        .ok()
        .and_then(CentiSeconds::try_from_seconds_f64)
        .map(Mark::TimeSeconds)
}

pub fn parse_grade(published: Option<&str>) -> Option<Grade> {
    let level = published?.trim().parse::<u8>().ok()?;
    Grade::new(level)
}

pub fn round_label(round: Option<&str>) -> Option<&'static str> {
    match round {
        Some("P") => Some("Prelims"),
        Some("F") => Some("Finals"),
        _ => None,
    }
}

pub fn finisher_grade(row: &FinisherRow) -> Option<Grade> {
    parse_grade(row.year.as_deref()).or_else(|| parse_grade(row.athlete.as_ref()?.year.as_deref()))
}

pub fn member_grade(member: &RelayMember) -> Option<Grade> {
    parse_grade(member.athlete.as_ref()?.year.as_deref())
}

pub fn class_token(class_division: &str) -> Option<&str> {
    let token = class_division.trim();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

pub fn event_label<'a>(event_name: &'a str, class_division: &str) -> Option<&'a str> {
    let published = event_name.trim();
    if published.is_empty() {
        return None;
    }
    let without_gender = ["Boys ", "Girls "]
        .into_iter()
        .find_map(|prefix| published.strip_prefix(prefix))
        .map_or(published, |value| value);
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

pub fn date_part(timestamp: &str) -> Option<&str> {
    let (date, _) = timestamp.split_once('T')?;
    if date.is_empty() {
        None
    } else {
        Some(date)
    }
}

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
