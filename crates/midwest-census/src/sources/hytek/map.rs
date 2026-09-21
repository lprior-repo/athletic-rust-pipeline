//! Meet, event and mark mapping for Hy-Tek reports.
//!
//! The meet header, the event labels and the published marks are what a Hy-Tek report states in
//! prose instead of in fixed columns; the readers here turn each into the canonical shape the
//! model carries: an ISO date, an event kind, a round marker, or a `Mark` with its wind and heat.

use crate::sources::result_file::ParsedEvent;
use census_domain::model::{EventKind, Gender, Mark};
use regex::Regex;
use std::sync::LazyLock;

static EVENT_HEADER: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r"^(?:Event\s+\d+\s+)?(Boys|Girls)\s+(.+?)(?:\s+(Division\s+[0-9A-Za-z]+))?\s*$")
});

static MARK_TOKEN: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^[0-9][0-9:.\-]*[A-Za-z]?$"));

static DATED: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"^(.*?)\s+-\s+(\d{1,2})/(\d{1,2})/(\d{4})(?:\s+to\s+(\d{1,2})/(\d{1,2})/(\d{4}))?\s*$",
    )
});

// Accessors for the literal patterns above: a failed compile is a programming error, so it comes
// back as a typed error that the readers answer as "this file carries no meet" — never a panic.
fn event_header_regex() -> anyhow::Result<&'static Regex> {
    EVENT_HEADER
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn mark_token_regex() -> anyhow::Result<&'static Regex> {
    MARK_TOKEN
        .as_ref()
        .map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn dated_regex() -> anyhow::Result<&'static Regex> {
    DATED.as_ref().map_err(|e| anyhow::anyhow!("regex: {e}"))
}

/// Plain-text marks that are results rather than numbers.
pub(crate) const NO_MARK: [&str; 8] = ["DNF", "DNS", "SCR", "NH", "FOUL", "NM", "DQ", "X"];

/// The meet name and dates are published on one header line, either as a single day
/// (`Name - 6/6/2025`) or as a range (`Name - 6/6/2025 to 6/7/2025`).
pub(super) fn header_meet(lines: &[String]) -> Option<(String, String, Option<String>)> {
    let dated = dated_regex().ok()?;
    for line in lines.iter().take(40) {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("Licensed to") {
            continue;
        }
        let Some(captures) = dated.captures(trimmed) else {
            continue;
        };
        let name = captures.get(1)?.as_str().trim().to_string();
        if name.is_empty() {
            continue;
        }
        let iso = |month: &str, day: &str, year: &str| -> Option<String> {
            Some(format!(
                "{:04}-{:02}-{:02}",
                year.parse::<i32>().ok()?,
                month.parse::<u32>().ok()?,
                day.parse::<u32>().ok()?
            ))
        };
        let start = iso(
            captures.get(2)?.as_str(),
            captures.get(3)?.as_str(),
            captures.get(4)?.as_str(),
        )?;
        let end = match (captures.get(5), captures.get(6), captures.get(7)) {
            (Some(month), Some(day), Some(year)) => {
                iso(month.as_str(), day.as_str(), year.as_str())
            }
            _ => None,
        };
        return Some((name, start, end));
    }
    None
}

pub(super) fn event_header(trimmed: &str) -> Option<ParsedEvent> {
    let captures = event_header_regex().ok()?.captures(trimmed)?;
    let gender = match captures.get(1)?.as_str() {
        "Boys" => Gender::Boys,
        _ => Gender::Girls,
    };
    let label = captures.get(2)?.as_str().trim().to_string();
    let kind = hytek_event_kind(&label);
    // The pattern also matches prose (`Boys of summer 2025`), so only labels the ontology knows, or
    // labels shaped like a track event, become events.
    if matches!(kind, EventKind::Unmapped { .. }) && !looks_like_event(&label) {
        return None;
    }
    Some(ParsedEvent {
        label,
        kind,
        gender,
        division: captures.get(3).map(|m| m.as_str().trim().to_string()),
        round: None,
        rows: Vec::new(),
    })
}

fn looks_like_event(label: &str) -> bool {
    let lowered = label.to_ascii_lowercase();
    [
        "meter", "metre", "jump", "vault", "put", "discus", "relay", "hurdle", "steeple", "medley",
        "athlon", "throw", "javelin", "hammer", "run", "dash", "walk",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

/// Map a Hy-Tek event label onto the canonical ontology.
///
/// Hy-Tek spells events out (`100 Meter Dash`, `3200 Meter Run`, `4x200 Meter Relay`), which the
/// ontology's compact keys do not cover, so the noise words are removed first.
pub fn hytek_event_kind(label: &str) -> EventKind {
    let compact: String = label
        .chars()
        .filter(|ch| !ch.is_whitespace() && *ch != '-' && *ch != '_')
        .collect::<String>()
        .to_ascii_lowercase()
        .replace("meters", "m")
        .replace("meter", "m")
        .replace("metres", "m")
        .replace("metre", "m");
    let direct = EventKind::from_source_label(&compact);
    if !matches!(direct, EventKind::Unmapped { .. }) {
        return direct;
    }
    // `4x800 Relay` drops the unit that `4x200 Meter Relay` keeps, and `Sprint Medley Relay`
    // spells the medley out, so the relay word is tried like the other noise words.
    for suffix in ["dash", "run", "throw", "relay"] {
        if let Some(stripped) = compact.strip_suffix(suffix) {
            let candidate = EventKind::from_source_label(stripped);
            if !matches!(candidate, EventKind::Unmapped { .. }) {
                return candidate;
            }
        }
    }
    EventKind::Unmapped {
        label: label.trim().to_string(),
    }
}

/// `preliminaries` / `finals` / `semi-finals` markers that open a section inside an event.
pub fn round_marker(trimmed: &str) -> Option<&'static str> {
    match trimmed {
        "Preliminaries" | "Prelims" => Some("preliminaries"),
        "Finals" => Some("finals"),
        "Semi-Finals" | "Semifinals" | "Semis" => Some("semi-finals"),
        _ => None,
    }
}

/// Convert a published Hy-Tek time (`10.56`, `1:54.32`, `15:32.1`, `1:05:12.34`) to seconds.
pub fn parse_time(token: &str) -> Option<f64> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    let parts: Vec<&str> = token.split(':').collect();
    match parts.as_slice() {
        [seconds] => {
            let value: f64 = seconds.parse().ok()?;
            (value.is_finite() && value >= 0.0).then_some(value)
        }
        [minutes, seconds] => {
            let minutes: f64 = minutes.parse().ok()?;
            let seconds: f64 = seconds.parse().ok()?;
            (minutes >= 0.0 && (0.0..60.0).contains(&seconds)).then_some(minutes * 60.0 + seconds)
        }
        [hours, minutes, seconds] => {
            let hours: f64 = hours.parse().ok()?;
            let minutes: f64 = minutes.parse().ok()?;
            let seconds: f64 = seconds.parse().ok()?;
            (hours >= 0.0 && (0.0..60.0).contains(&minutes) && (0.0..60.0).contains(&seconds))
                .then_some(hours * 3600.0 + minutes * 60.0 + seconds)
        }
        _ => None,
    }
}

/// A published field mark: imperial (`61-03.50`, `5' 4"`) kept verbatim plus its metric value, or a
/// bare metre figure. The leading `J` Hy-Tek prints for a jump tie-break is not part of the mark.
pub fn parse_field_mark(token: &str) -> Option<Mark> {
    let token = token.trim().trim_start_matches(['J', 'j']).trim();
    if token.is_empty() {
        return None;
    }
    if let Some((feet, inches)) = token.split_once('-') {
        let feet: f64 = feet.trim().parse().ok()?;
        let inches: f64 = inches.trim().parse().ok()?;
        if !(0.0..12.0).contains(&inches) || feet < 0.0 {
            return None;
        }
        return Some(Mark::FieldImperial {
            feet_mark: token.to_string(),
            metres: (feet * 12.0 + inches) * 0.0254,
        });
    }
    if let Some((feet, rest)) = token.split_once('\'') {
        let feet: f64 = feet.trim().parse().ok()?;
        let inches_text = rest.trim().trim_end_matches('"').trim();
        let inches: f64 = if inches_text.is_empty() {
            0.0
        } else {
            inches_text.parse().ok()?
        };
        if !(0.0..12.0).contains(&inches) {
            return None;
        }
        return Some(Mark::FieldImperial {
            feet_mark: token.to_string(),
            metres: (feet * 12.0 + inches) * 0.0254,
        });
    }
    let metres: f64 = token.parse().ok()?;
    (metres.is_finite() && metres > 0.0).then_some(Mark::DistanceMetres(metres))
}

/// Mark plus the wind, heat and points published beside it.
/// `(mark, wind m/s, timing label, place)` — what one published mark token resolves to.
pub(super) type ParsedMark = (Mark, Option<f64>, Option<String>, Option<f64>);

pub(super) fn parse_marks(kind: &EventKind, mark_token: &str, tail: &str) -> Option<ParsedMark> {
    let upper = mark_token.to_ascii_uppercase();
    let mark = if NO_MARK.contains(&upper.as_str()) {
        Mark::Raw(mark_token.to_string())
    } else {
        // `Q`/`P` mark a qualifier, `J` a jump tie-break; neither belongs to the mark itself.
        let numeric = mark_token
            .trim_end_matches(['Q', 'q', 'P', 'p'])
            .trim_start_matches(['J', 'j'])
            .to_string();
        let Ok(mark_pattern) = mark_token_regex() else {
            return None;
        };
        if !mark_pattern.is_match(&numeric) {
            return None;
        }
        if kind.is_field() {
            parse_field_mark(&numeric)?
        } else {
            Mark::TimeSeconds(parse_time(&numeric)?)
        }
    };

    let mut wind = None;
    let mut heat = None;
    let mut points = None;
    for token in tail.split_whitespace() {
        let is_decimal = token.contains('.') || token.starts_with('-');
        if is_decimal && !kind.is_field() && wind.is_none() {
            if let Ok(value) = token.parse::<f64>() {
                if value.abs() <= 6.0 {
                    wind = Some(value);
                    continue;
                }
            }
        }
        if let Ok(value) = token.parse::<f64>() {
            if kind.is_field() {
                if heat.is_none() {
                    heat = Some(token.to_string());
                } else if points.is_none() {
                    points = Some(value);
                }
                continue;
            }
            if heat.is_none() {
                heat = Some(token.to_string());
                continue;
            }
            if points.is_none() {
                points = Some(value);
            }
        }
    }
    Some((mark, wind, heat, points))
}
