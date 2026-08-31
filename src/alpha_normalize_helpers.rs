use crate::alpha_model::SourceResult;
use crate::alpha_url::canonical_state;
use crate::marks;
use crate::model::Mark;

fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn parse_location(raw: &str) -> Option<(String, String)> {
    let value = raw.trim();
    if value.is_empty() || looks_like_private_location(value) {
        return None;
    }
    if let Some((city, state)) = value.split_once(',') {
        let city = normalize_whitespace(city);
        if city.is_empty() {
            return None;
        }
        return Some((city, canonical_state(state)?));
    }
    Some((normalize_whitespace(value), String::new()))
}

fn looks_like_private_location(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let digits = value.chars().filter(|ch| ch.is_ascii_digit()).count();
    let has_postal = value
        .split(|ch: char| !ch.is_ascii_digit())
        .any(|part| part.len() == 5);
    value.chars().next().is_some_and(|ch| ch.is_ascii_digit())
        || lower.contains('@')
        || digits >= 7
        || has_postal
        || lower.split_whitespace().any(|part| {
            matches!(part.trim_matches(|ch: char| !ch.is_ascii_alphabetic()),
                "street" | "avenue" | "boulevard" | "road")
        })
        || lower.contains("apt ") || lower.contains("unit ") || lower.contains("po box")
}

fn valid_season(value: &str) -> bool {
    let Some((start, end)) = value.trim().split_once('-') else {
        return false;
    };
    let Ok(start_year) = start.parse::<i32>() else {
        return false;
    };
    let Ok(end_year) = end.parse::<i32>() else {
        return false;
    };
    start.len() == 4
        && end.len() == 2
        && end_year == (start_year + 1) % 100
        && (1900..=2200).contains(&start_year)
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || !value
            .chars()
            .enumerate()
            .all(|(index, ch)| index == 4 || index == 7 || ch.is_ascii_digit())
    {
        return false;
    }
    let Ok(year) = value[0..4].parse::<i32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u32>() else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    };
    (1900..=2200).contains(&year) && day > 0 && day <= max_day
}

pub(crate) fn parse_mark_entry(
    mark_str: &str,
    source_url: &str,
    result_id: Option<u64>,
) -> Option<SourceResult> {
    let parts: Vec<&str> = mark_str.split('|').collect();
    if !(4..=6).contains(&parts.len()) {
        return None;
    }
    let event = normalize_whitespace(parts[0]);
    let season = normalize_whitespace(parts[2]);
    let date = normalize_whitespace(parts[3]);
    if !valid_season(&season) || !valid_date(&date) {
        return None;
    }
    let mut mark = Mark::default();
    mark.event = event;
    mark.mark = parts[1].trim().to_owned();
    mark.season = season;
    mark.date = date;
    if let Some(meet_name) = parts.get(4) {
        mark.meet_name = normalize_whitespace(meet_name);
    }
    if let Some(wind) = parts.get(5) {
        let wind = normalize_whitespace(wind);
        if !wind.is_empty() {
            mark.wind = Some(wind);
        }
    }
    let normalized = marks::normalize_mark(mark);
    normalized.valid.then_some(SourceResult {
        result_id,
        event: normalized.canonical_event,
        mark: normalized.mark,
        season: normalized.season,
        date: normalized.date,
        meet_name: normalized.meet_name,
        wind: normalized.wind,
        source_url: source_url.to_owned(),
        result_url: None,
    })
}
