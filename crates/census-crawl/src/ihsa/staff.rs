
use census_domain::model::{CoachRole, Gender, Sport};

pub fn strip_honorific(value: &str) -> String {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let stripped = parts
        .iter()
        .take_while(|part| {
            matches!(
                part.trim_end_matches('.').to_ascii_lowercase().as_str(),
                "mr" | "mrs" | "ms" | "miss" | "dr" | "coach" | "sir" | "rev"
            )
        })
        .count();
    match parts.get(stripped..) {
        Some(kept) if !kept.is_empty() => kept.join(" "),
        _ => value.trim().to_string(),
    }
}

pub fn parse_coach_title(title: &str) -> Option<(Sport, Gender)> {
    let lowered = title.to_ascii_lowercase();
    if lowered.trim().is_empty() {
        return None;
    }

    const NON_COACHING: [&str; 10] = [
        "secretary",
        "administrative assistant",
        "trainer",
        "principal",
        "superintendent",
        "business manager",
        "tech director",
        "custodian",
        "activities director",
        "athletic director",
    ];
    if NON_COACHING.iter().any(|t| lowered.contains(t)) {
        return None;
    }

    if !lowered.contains("coach") {
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

    let gender = if lowered.contains("girls") || lowered.contains("women") {
        Gender::Girls
    } else if lowered.contains("boys") || lowered.contains("men") {
        Gender::Boys
    } else {
        Gender::Mixed
    };

    Some((sport, gender))
}

pub fn parse_role(title: &str) -> Option<CoachRole> {
    let lowered = title.to_ascii_lowercase();

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

    if lowered.contains("athletic director") && !lowered.contains("assistant") {
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
