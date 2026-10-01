use census_domain::model::{EventKind, Gender, Sport};

use super::RawSection;

pub(super) fn section_of(line: &str, sport: Option<Sport>) -> RawSection {
    let label = line.trim().to_string();
    let (gender, division) = split_prefix(&label);
    let kind = event_kind(&label, sport);
    let round = label
        .split_whitespace()
        .find_map(round_label)
        .map(str::to_string);
    RawSection {
        label,
        kind,
        gender,
        division,
        round,
        rows: Vec::new(),
    }
}

pub(super) fn event_label(line: &str, sport: Option<Sport>) -> bool {
    if line.starts_with(char::is_whitespace) {
        return false;
    }
    let (gender, _) = split_prefix(line);
    gender != Gender::Unknown || !matches!(event_kind(line, sport), EventKind::Unmapped { .. })
}

pub(super) fn round_label(line: &str) -> Option<&str> {
    let label = line.trim();
    crate::hytek::round_marker(label).map(|_| label)
}

fn event_kind(label: &str, sport: Option<Sport>) -> EventKind {
    let mut rest = label.trim();
    loop {
        let kind = EventKind::from_source_label(rest);
        if !matches!(kind, EventKind::Unmapped { .. }) {
            return if sport == Some(Sport::CrossCountry) {
                EventKind::CrossCountry
            } else {
                kind
            };
        }
        match rest.split_once(' ') {
            Some((_, tail)) => rest = tail,
            None => return EventKind::from_source_label(label),
        }
    }
}

fn split_prefix(label: &str) -> (Gender, Option<String>) {
    let side = label.split_whitespace().rev().find_map(|token| {
        let gender = gender_of(token);
        (gender != Gender::Unknown).then_some((token, gender))
    });
    if let Some((token, gender)) = side {
        let tail = label
            .rsplit_once(token)
            .map(|(_, tail)| tail)
            .unwrap_or_default();
        let level = tail
            .split_whitespace()
            .take_while(|token| !token.starts_with(|ch: char| ch.is_numeric()))
            .collect::<Vec<_>>()
            .join(" ");
        return (gender, (!level.is_empty()).then_some(level));
    }
    (Gender::Unknown, None)
}

fn gender_of(token: &str) -> Gender {
    let token = token.trim_end_matches(['\'', '’']);
    if ["boys", "boy", "mens", "men", "male", "men's", "men’s"]
        .iter()
        .any(|label| token.eq_ignore_ascii_case(label))
    {
        Gender::Boys
    } else if [
        "girls",
        "girl",
        "womens",
        "women",
        "female",
        "women's",
        "women’s",
    ]
    .iter()
    .any(|label| token.eq_ignore_ascii_case(label))
    {
        Gender::Girls
    } else {
        Gender::Unknown
    }
}
