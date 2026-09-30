use census_domain::model::{CoachRole, Gender, Sport};

pub fn team_sport(label: &str) -> Option<(Sport, Gender)> {
    let mut rest = label.trim();
    let gender = if let Some(value) = rest
        .strip_prefix("Boys'")
        .or_else(|| rest.strip_prefix("Boy's"))
    {
        rest = value.trim_start();
        Gender::Boys
    } else if let Some(value) = rest
        .strip_prefix("Girls'")
        .or_else(|| rest.strip_prefix("Girl's"))
    {
        rest = value.trim_start();
        Gender::Girls
    } else {
        Gender::Mixed
    };
    for prefix in ["Unified ", "Mixed "] {
        if let Some(value) = rest.strip_prefix(prefix) {
            rest = value.trim_start();
            break;
        }
    }
    if rest.starts_with("Cross Country") {
        return Some((Sport::CrossCountry, gender));
    }
    if rest.starts_with("Track, Indoor") {
        return Some((Sport::IndoorTrack, gender));
    }
    if rest.starts_with("Track") {
        return Some((Sport::OutdoorTrack, gender));
    }
    None
}

pub fn coach_role(title: &str) -> CoachRole {
    let lowered = title.to_ascii_lowercase();
    if lowered.contains("head coach") {
        CoachRole::HeadCoach
    } else if lowered.contains("assistant coach") {
        CoachRole::AssistantCoach
    } else {
        CoachRole::Unknown
    }
}

pub fn is_director(title: &str) -> bool {
    title.to_ascii_lowercase().contains("athletic director")
}
