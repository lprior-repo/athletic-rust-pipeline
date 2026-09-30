use census_domain::model::{CoachRole, Gender, Sport};

pub(super) fn clean(value: &str) -> String {
    value.trim().to_string()
}

pub(super) fn strip_honorific(value: &str) -> String {
    let mut parts: Vec<&str> = value.split_whitespace().collect();
    while let Some(first) = parts.first() {
        let token = first.trim_end_matches('.').to_ascii_lowercase();
        if matches!(
            token.as_str(),
            "mr" | "mrs" | "ms" | "miss" | "dr" | "coach" | "coach." | "sir" | "rev"
        ) {
            parts.remove(0);
        } else {
            break;
        }
    }
    if parts.is_empty() {
        value.trim().to_string()
    } else {
        parts.join(" ")
    }
}

pub(super) fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

pub fn parse_gender(label: &str) -> Option<Gender> {
    let lowered = label.to_ascii_lowercase();
    if lowered.contains("girls") || lowered.contains("women") {
        Some(Gender::Girls)
    } else if lowered.contains("boys") || lowered.contains("men") {
        Some(Gender::Boys)
    } else {
        None
    }
}

pub fn parse_sport(label: &str) -> Option<(Sport, Gender)> {
    let lowered = label.to_ascii_lowercase();
    if lowered.trim().is_empty() {
        return None;
    }
    let sport = if lowered.contains("cross country") || lowered.contains("cross-country") {
        Sport::CrossCountry
    } else if lowered.contains("indoor") {
        Sport::IndoorTrack
    } else if lowered.contains("track") {
        Sport::OutdoorTrack
    } else {
        return None;
    };
    let gender = parse_gender(label).unwrap_or(Gender::Mixed);
    Some((sport, gender))
}

pub fn parse_role(label: &str) -> Option<CoachRole> {
    let lowered = label.to_ascii_lowercase();
    const NON_COACHING: [&str; 8] = [
        "secretary",
        "administrative assistant",
        "trainer",
        "principal",
        "superintendent",
        "business manager",
        "tech director",
        "custodian",
    ];
    if NON_COACHING.iter().any(|token| lowered.contains(token)) {
        return None;
    }
    if lowered.contains("athletic director") || lowered.contains("activities director") {
        return Some(CoachRole::AthleticDirector);
    }
    if lowered.contains("coach") {
        if lowered.contains("assistant") || lowered.contains("asst") {
            return Some(CoachRole::AssistantCoach);
        }
        if lowered.contains("head") {
            return Some(CoachRole::HeadCoach);
        }
        return Some(CoachRole::Unknown);
    }
    None
}

pub fn parse_gender(raw: &str) -> Gender {
    match raw.trim().to_lowercase().as_str() {
        "male" | "boy" => Gender::Boys,
        "female" | "girl" => Gender::Girls,
        "non-binary" | "nonbinary" | "nb" | "mixed" => Gender::Mixed,
        _ => Gender::Unknown,
    }
}
