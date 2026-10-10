use census_domain::model::CentiMetres;
use census_domain::model::CentiPoints;
use census_domain::model::ExactSeconds;
use census_domain::model::{EventKind, Mark};

use super::super::NO_MARK;
use super::mark_token_regex;

pub fn hytek_event_kind(label: &str) -> EventKind {
    let direct = EventKind::from_source_label(label);
    if !matches!(direct, EventKind::Unmapped { .. }) {
        return direct;
    }
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

pub fn round_marker(trimmed: &str) -> Option<&'static str> {
    match trimmed {
        "Preliminaries" | "Prelims" => Some("preliminaries"),
        "Finals" => Some("finals"),
        "Semi-Finals" | "Semifinals" | "Semis" => Some("semi-finals"),
        _ => None,
    }
}
pub fn parse_time(token: &str) -> Option<ExactSeconds> {
    ExactSeconds::parse_clock(token.trim()).ok()
}

pub fn parse_time_with_hand(token: &str) -> Option<ExactSeconds> {
    let trimmed = token.trim();
    if let Some(stripped) = trimmed.strip_suffix(['h', 'H']) {
        if let Ok(seconds) = ExactSeconds::parse_clock(stripped) {
            return Some(seconds);
        }
    }
    ExactSeconds::parse_clock(trimmed).ok()
}

pub fn time_is_hand(token: &str) -> bool {
    let trimmed = token.trim();
    if let Some(stripped) = trimmed.strip_suffix(['h', 'H']) {
        if !stripped.trim().is_empty() && ExactSeconds::parse_clock(stripped).is_ok() {
            return true;
        }
    }
    false
}

pub(crate) fn parse_points(token: &str) -> Option<Mark> {
    let points: f64 = token.trim().parse().ok()?;
    if !points.is_finite() || points < 0.0 {
        return None;
    }
    CentiPoints::try_from_points_f64(points).map(Mark::Points)
}

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
            metres: CentiMetres::try_from_metres_f64(metres)?,
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
            metres: CentiMetres::try_from_metres_f64(metres)?,
        });
    }
    let metres: f64 = token.parse().ok()?;
    (metres.is_finite() && metres > 0.0)
        .then_some(CentiMetres::try_from_metres_f64(metres))
        .flatten()
        .map(Mark::DistanceMetres)
}

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
        let numeric = mark_token
            .trim_end_matches(['Q', 'q', 'P', 'p'])
            .trim_start_matches(['J', 'j']);
        let Ok(mark_pattern) = mark_token_regex() else {
            return None;
        };
        if !mark_pattern.is_match(numeric) {
            return None;
        }
        match kind {
            EventKind::Decathlon | EventKind::Pentathlon | EventKind::Heptathlon => {
                parse_points(numeric)?
            }
            kind if kind.is_field() => parse_field_mark(numeric)?,
            _ => Mark::TimeSeconds(parse_time_with_hand(numeric)?),
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
