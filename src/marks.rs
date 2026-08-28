use crate::model::Mark;
use regex::Regex;
use std::{cmp::Ordering, collections::BTreeMap, sync::LazyLock};

pub fn normalize_mark(mut mark: Mark) -> Mark {
    mark.canonical_event = canonical_event(&mark.event);
    mark.parsed_value = parse_mark_value(&mark.canonical_event, &mark.mark);
    mark.valid = !mark.canonical_event.is_empty() && mark.parsed_value.is_some();
    mark
}

pub fn best_marks(marks: &[Mark]) -> BTreeMap<String, Mark> {
    let mut best = BTreeMap::new();
    for mark in marks.iter().filter(|mark| mark.valid) {
        let key = mark.canonical_event.clone();
        match best.get(&key) {
            None => {
                best.insert(key, mark.clone());
            }
            Some(previous) if is_better(mark, previous) => {
                best.insert(key, mark.clone());
            }
            _ => {}
        }
    }
    best
}

fn is_better(candidate: &Mark, previous: &Mark) -> bool {
    let Some(candidate_value) = candidate.parsed_value else {
        return false;
    };
    let Some(previous_value) = previous.parsed_value else {
        return true;
    };
    let ordering = match candidate_value.partial_cmp(&previous_value) {
        Some(ordering) => ordering,
        None => Ordering::Equal,
    };
    if is_timed_event(&candidate.canonical_event) {
        ordering == Ordering::Less
    } else {
        ordering == Ordering::Greater
    }
}

fn is_timed_event(event: &str) -> bool {
    matches!(
        event,
        "55m"
            | "60m"
            | "100m"
            | "200m"
            | "300m"
            | "400m"
            | "600m"
            | "800m"
            | "1000m"
            | "1500m"
            | "1600m"
            | "mile"
            | "3000m"
            | "3200m"
            | "5000m"
            | "100h"
            | "110h"
            | "300h"
            | "400h"
    )
}

pub fn canonical_event(value: &str) -> String {
    let normalized = value
        .to_lowercase()
        .replace(['-', '_'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let compact = normalized.replace(' ', "");
    match compact.as_str() {
        "55m" | "55meters" | "55meter" => "55m",
        "60m" | "60meters" | "60meter" => "60m",
        "100m" | "100meters" | "100meter" => "100m",
        "200m" | "200meters" | "200meter" => "200m",
        "300m" | "300meters" | "300meter" => "300m",
        "400m" | "400meters" | "400meter" => "400m",
        "600m" | "600meters" | "600meter" => "600m",
        "800m" | "800meters" | "800meter" => "800m",
        "1000m" | "1000meters" | "1000meter" => "1000m",
        "1500m" | "1500meters" | "1500meter" => "1500m",
        "1600m" | "1600meters" | "1600meter" => "1600m",
        "mile" | "1mile" => "mile",
        "3000m" | "3000meters" | "3000meter" => "3000m",
        "3200m" | "3200meters" | "3200meter" => "3200m",
        "5000m" | "5000meters" | "5000meter" | "5k" => "5000m",
        "100h" | "100mh" | "100mhurdles" | "100meterhurdles" => "100h",
        "110h" | "110mh" | "110mhurdles" | "110meterhurdles" => "110h",
        "300h" | "300mh" | "300mhurdles" | "300meterhurdles" => "300h",
        "400h" | "400mh" | "400mhurdles" | "400meterhurdles" => "400h",
        "highjump" | "hj" => "high_jump",
        "longjump" | "lj" => "long_jump",
        "triplejump" | "tj" => "triple_jump",
        "polevault" | "pv" => "pole_vault",
        "shotput" | "shot" => "shot_put",
        "discus" | "discusthrow" => "discus",
        "javelin" | "javelinthrow" => "javelin",
        _ => "",
    }
    .to_owned()
}

pub fn parse_mark_value(event: &str, value: &str) -> Option<f64> {
    if is_timed_event(event) {
        parse_time_seconds(value)
    } else {
        parse_distance(value)
    }
}

fn parse_time_seconds(value: &str) -> Option<f64> {
    let cleaned = value.trim().trim_end_matches(['a', 'A', 'h', 'H']);
    if cleaned.contains(':') {
        let parts: Vec<&str> = cleaned.split(':').collect();
        match parts.as_slice() {
            [minutes, seconds] => {
                Some(minutes.parse::<f64>().ok()? * 60.0 + seconds.parse::<f64>().ok()?)
            }
            [hours, minutes, seconds] => Some(
                hours.parse::<f64>().ok()? * 3600.0
                    + minutes.parse::<f64>().ok()? * 60.0
                    + seconds.parse::<f64>().ok()?,
            ),
            _ => None,
        }
    } else {
        cleaned.parse::<f64>().ok()
    }
}

fn parse_distance(value: &str) -> Option<f64> {
    static IMPERIAL: LazyLock<Option<Regex>> = LazyLock::new(|| {
        Regex::new(r#"(?i)^\s*(\d+)\s*(?:-|')\s*(\d+(?:\.\d+)?)\s*(?:\"|in)?\s*$"#).ok()
    });
    static METRIC: LazyLock<Option<Regex>> =
        LazyLock::new(|| Regex::new(r"(?i)^\s*(\d+(?:\.\d+)?)\s*m(?:eters?)?\s*$").ok());
    let Some(imperial) = IMPERIAL.as_ref() else {
        return parse_metric(value, METRIC.as_ref());
    };
    if let Some(captures) = imperial.captures(value) {
        let feet = captures.get(1)?.as_str().parse::<f64>().ok()?;
        let inches = captures.get(2)?.as_str().parse::<f64>().ok()?;
        return Some(feet * 12.0 + inches);
    }
    parse_metric(value, METRIC.as_ref())
}

fn parse_metric(value: &str, metric: Option<&Regex>) -> Option<f64> {
    let captures = metric?.captures(value)?;
    let meters = captures.get(1)?.as_str().parse::<f64>().ok()?;
    Some(meters * 39.370_078_740_2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_times() {
        assert_eq!(parse_mark_value("100m", "12.41"), Some(12.41));
        assert_eq!(parse_mark_value("1600m", "4:58.22"), Some(298.22));
    }

    #[test]
    fn parses_distances() {
        assert_eq!(parse_mark_value("long_jump", "18-4.25"), Some(220.25));
        let metric = parse_mark_value("long_jump", "5.62m").unwrap();
        assert!((metric - 221.2598).abs() < 0.001);
    }

    #[test]
    fn canonicalizes_events() {
        assert_eq!(canonical_event("100 Meters"), "100m");
        assert_eq!(canonical_event("Long Jump"), "long_jump");
    }
}
