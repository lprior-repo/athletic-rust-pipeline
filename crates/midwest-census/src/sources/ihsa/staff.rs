//! Staff, coach and title parsing: honorific stripping, and the title → role/sport/gender
//! rules that decide which published titles are coaching roles at all.

use crate::model::{CoachRole, Gender, Sport};

/// Strip leading honorifics so "Mr. Barry Mink" and "Barry Mink" mint the same coach identity.
pub fn strip_honorific(value: &str) -> String {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let stripped = parts
        .iter()
        .take_while(|part| {
            matches!(
                part.trim_end_matches('.').to_ascii_lowercase().as_str(),
                "mr" | "mrs" | "ms" | "miss" | "dr" | "coach" | "coach." | "sir" | "rev"
            )
        })
        .count();
    match parts.get(stripped..) {
        Some(kept) if !kept.is_empty() => kept.join(" "),
        // The value held nothing but honorifics (or no tokens at all): keep the original text.
        _ => value.trim().to_string(),
    }
}

/// Parse the IHSA `DefaultTitle` into a `(Sport, Gender)` pair.
///
/// Returns `None` for non-coaching titles (AD, secretary, trainer, principal, etc.).
///
/// # Examples
/// * `"Boys Cross Country Head Coach"` → `(Some(CrossCountry), Some(Boys))`
/// * `"Girls Track & Field Head Coach"` → `(Some(OutdoorTrack), Some(Girls))`
/// * `"Boys Athletic Director"` → `(None, None)`
pub fn parse_coach_title(title: &str) -> Option<(Sport, Gender)> {
    let lowered = title.to_ascii_lowercase();
    if lowered.trim().is_empty() {
        return None;
    }

    // Office/medical roles are not coaches.
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

    // Must contain "coach" to be a coaching role.
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

/// Map a published role label onto our role vocabulary.
///
/// Returns `None` for roles that are neither a coaching role nor an athletic-director role
/// (secretaries, trainers, principals, etc.).
pub fn parse_role(title: &str) -> Option<CoachRole> {
    let lowered = title.to_ascii_lowercase();

    // Office, medical and building staff are published in the same tables as coaches, and their
    // labels ("Athletic Director Secretary", "AD Administrative Assistant", "Athletic Trainer")
    // contain the words we would otherwise classify on.
    // NON_COACHING runs FIRST so "Athletic Director Secretary" gets filtered out before the AD check.
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

    // AD roles — check AFTER NON_COACHING.
    // "Boys Athletic Director" → AD (contains "athletic director", no "assistant")
    // "Boys Athletic Director's Assistant" → NOT AD (also contains "assistant")
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
