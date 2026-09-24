//! The mark readers: an event label is an [`EventKind`], a round marker is a round, and a published
//! mark is a [`Mark`] with its wind and heat.
//!
//! Every token here arrives as prose — `11.83a`, `5-06`, `1:52.4c` — so each reader answers with
//! `None` or an error rather than a guess, and the caller decides what an unreadable mark means.

use census_domain::model::{EventKind, Mark};
use census_domain::model::CentiSeconds;
use census_domain::model::CentiMetres;

use super::super::NO_MARK;
use super::mark_token_regex;

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
/// Convert a published Hy-Tek time (`10.56`, `1:54.32`, `15:32.1`, `1:05:12.34`) to centiseconds.
pub fn parse_time(token: &str) -> Option<CentiSeconds> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    let parts: Vec<&str> = token.split(':').collect();
    match parts.as_slice() {
        [seconds] => {
            let value: f64 = seconds.parse().ok()?;
            (value.is_finite() && value >= 0.0).then_some(CentiSeconds::from_seconds_f64(value))
        }
        [minutes, seconds] => {
            let minutes: f64 = minutes.parse().ok()?;
            let seconds: f64 = seconds.parse().ok()?;
            let total = minutes * 60.0 + seconds;
            (minutes >= 0.0 && (0.0..60.0).contains(&seconds) && total.is_finite())
                .then_some(CentiSeconds::from_seconds_f64(total))
        }
        [hours, minutes, seconds] => {
            let hours: f64 = hours.parse().ok()?;
            let minutes: f64 = minutes.parse().ok()?;
            let seconds: f64 = seconds.parse().ok()?;
            let total = hours * 3600.0 + minutes * 60.0 + seconds;
            (hours >= 0.0
                && (0.0..60.0).contains(&minutes)
                && (0.0..60.0).contains(&seconds)
                && total.is_finite())
                .then_some(CentiSeconds::from_seconds_f64(total))
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
        let metres = (feet * 12.0 + inches) * 0.0254;
        if !(0.0..12.0).contains(&inches) || feet < 0.0 || !metres.is_finite() {
            return None;
        }
        return Some(Mark::FieldImperial {
            feet_mark: token.to_string(),
            metres: CentiMetres::from_metres_f64(metres),
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
        let metres = (feet * 12.0 + inches) * 0.0254;
        if !(0.0..12.0).contains(&inches) || !metres.is_finite() {
            return None;
        }
        return Some(Mark::FieldImperial {
            feet_mark: token.to_string(),
            metres: CentiMetres::from_metres_f64(metres),
        });
    }
    let metres: f64 = token.parse().ok()?;
    (metres.is_finite() && metres > 0.0)
        .then_some(Mark::DistanceMetres(CentiMetres::from_metres_f64(metres)))
}

/// Mark plus the wind, heat and points published beside it.
/// `(mark, wind m/s, timing label, place)` — what one published mark token resolves to.
pub(in crate::hytek) type ParsedMark = (Mark, Option<f64>, Option<String>, Option<f64>);

pub(in crate::hytek) fn parse_marks(
    kind: &EventKind,
    mark_token: &str,
    tail: &str,
) -> Option<ParsedMark> {
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
