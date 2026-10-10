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
    let gender = if lowered.contains("girls") || lowered.contains("women") {
        Gender::Girls
    } else if lowered.contains("boys") || lowered.contains("men") {
        Gender::Boys
    } else {
        Gender::Mixed
    };
    Some((sport, gender))
}

const NON_COACHING: [&str; 16] = [
    "secretary",
    "administrative assistant",
    "admin assistant",
    "athletic admin",
    "athletic trainer",
    "trainer",
    "principal",
    "superintendent",
    "business manager",
    "tech director",
    "custodian",
    "medical official",
    "central office",
    "assistant ad",
    "district ad",
    "office manager",
];

const NON_COACHING_EXACT: [&str; 5] = ["other", "none", "n/a", "na", "unknown"];

const DIRECTOR_TITLES: [&str; 3] = [
    "athletic director",
    "athletics director",
    "director of athletics",
];

const HEAD_TITLES: [&str; 3] = ["head coach", "co-head coach", "head varsity coach"];

const ASSISTANT_TITLES: [&str; 6] = [
    "assistant coach",
    "asst coach",
    "assistant varsity coach",
    "varsity assistant coach",
    "assistant coach varsity",
    "coach assistant",
];

const DEDICATED_TITLES: [&str; 14] = [
    "assistant",
    "asst",
    "varsity assistant",
    "assistant varsity",
    "throws",
    "throw",
    "distance",
    "middle distance",
    "sprints",
    "sprint",
    "hurdles",
    "relays",
    "jumps",
    "jump",
];

const EVENT_TITLES: [&str; 14] = [
    "pole vault",
    "high jump",
    "long jump",
    "triple jump",
    "shot put",
    "discus",
    "javelin",
    "throws",
    "jumps",
    "jump",
    "distance",
    "sprints",
    "hurdles",
    "relays",
];

pub fn parse_role(label: &str) -> Option<CoachRole> {
    let lowered = label.to_ascii_lowercase();
    let normalised = lowered.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalised.is_empty() || NON_COACHING_EXACT.contains(&normalised.as_str()) {
        return None;
    }
    if NON_COACHING.iter().any(|token| normalised.contains(token)) {
        return None;
    }
    let assistant = ASSISTANT_TITLES
        .iter()
        .any(|title| normalised.contains(title));
    let directed = DIRECTOR_TITLES
        .iter()
        .any(|title| normalised.contains(title));
    let coached = normalised.contains("coach");
    if coached && HEAD_TITLES.iter().any(|title| normalised.contains(title)) {
        return Some(CoachRole::HeadCoach);
    }
    if directed
        && !assistant
        && !normalised.contains("assistant")
        && !normalised.contains("asst")
        && !normalised.contains("associate")
    {
        return Some(CoachRole::AthleticDirector);
    }
    if coached && assistant {
        return Some(CoachRole::AssistantCoach);
    }
    if coached && EVENT_TITLES.iter().any(|title| normalised.contains(title)) {
        return Some(CoachRole::AssistantCoach);
    }
    if EVENT_TITLES.contains(&normalised.as_str())
        || DEDICATED_TITLES.contains(&normalised.as_str())
    {
        return Some(CoachRole::AssistantCoach);
    }
    if coached {
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
