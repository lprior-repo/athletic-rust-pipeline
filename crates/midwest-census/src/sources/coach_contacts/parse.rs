//! The dataset's published label vocabulary: the sport and role labels mapped onto our ontology,
//! and the string hygiene the row readers apply to published names.

use census_domain::model::{CoachRole, Gender, Sport};

pub(super) fn clean(value: &str) -> String {
    value.trim().to_string()
}

/// Strip leading honorifics so "Mr. Barry Mink" and "Barry Mink" mint the same coach identity.
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

/// Map a published sport label onto our ontology plus the gender side it covers.
///
/// Examples that must work: `Boys Track and Field`, `Varsity Head Coach - Girls Cross Country`,
/// `Boys Cross Country Head Coach`, `Girls Track & Field Head Coach`.
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

/// Map a published role label onto our role vocabulary. Returns `None` for roles that are neither a
/// coaching role nor an athletic-director role (secretaries, trainers, principals), which keeps the
/// coach table free of non-coaching staff.
pub fn parse_role(label: &str) -> Option<CoachRole> {
    let lowered = label.to_ascii_lowercase();
    // Office, medical and building staff are published in the same tables as coaches, and their
    // labels ("Athletic Director Secretary", "AD Administrative Assistant", "Athletic Trainer")
    // contain the words we would otherwise classify on.
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
