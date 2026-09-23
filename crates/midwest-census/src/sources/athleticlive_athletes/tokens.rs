//! Published-token readers: the grade, gender and sport vocabularies, and the school
//! year a row's meet date belongs to.

use census_domain::model::{Gender, Grade, SchoolYear, Sport};

/// Parse a grade token from the athlete index.
///
/// Returns `None` for blanks and for values outside the four high-school grades; grade 8 and below
/// are out of contract for this census.
pub fn grade_from_token(token: &str) -> Option<Grade> {
    let cleaned = token.trim().trim_start_matches('0').to_ascii_uppercase();
    let by_name = match cleaned.as_str() {
        "FR" | "FRESHMAN" => Some(9),
        "SO" | "SOPHOMORE" => Some(10),
        "JR" | "JUNIOR" => Some(11),
        "SR" | "SENIOR" => Some(12),
        _ => None,
    };
    if let Some(grade) = by_name {
        return Grade::new(grade);
    }
    cleaned.parse::<u8>().ok().and_then(Grade::new)
}

/// Grade year for a row's meet date. A date the adapter cannot place — unreadable, or carrying a
/// year no season may open in — falls back to the caller's school year rather than to a guess.
pub fn school_year_for_date(date: &str, fallback: SchoolYear) -> SchoolYear {
    let year = date.get(..4).and_then(|y| y.parse::<i16>().ok());
    let month = date.get(5..7).and_then(|m| m.parse::<u8>().ok());
    match (year, month) {
        (Some(year), Some(month)) if (1..=12).contains(&month) => {
            SchoolYear::containing(year, month).unwrap_or(fallback)
        }
        _ => fallback,
    }
}

/// Gender token as published by AthleticLIVE.
pub fn gender_from_token(token: &str) -> Gender {
    match token.trim().to_ascii_lowercase().as_str() {
        "male" | "m" | "boys" | "boy" => Gender::Boys,
        "female" | "f" | "girls" | "girl" => Gender::Girls,
        _ => Gender::Unknown,
    }
}

/// Sport of a row: the team's cross-country marker wins, otherwise the meet name decides, and the
/// meet month is the last resort (indoor meets run Dec-Mar, outdoor Apr-Jul).
pub fn sport_for(team_is_xc: bool, meet_name: &str, meet_date: &str) -> Sport {
    if team_is_xc {
        return Sport::CrossCountry;
    }
    let lowered = meet_name.to_ascii_lowercase();
    if lowered.contains("cross country") || lowered.contains("xc ") || lowered.ends_with(" xc") {
        return Sport::CrossCountry;
    }
    if lowered.contains("indoor") || lowered.contains("mits") {
        return Sport::IndoorTrack;
    }
    match meet_date.get(5..7).and_then(|m| m.parse::<u8>().ok()) {
        Some(month @ (12 | 1 | 2 | 3)) => {
            let _ = month;
            Sport::IndoorTrack
        }
        Some(_) => Sport::OutdoorTrack,
        None => Sport::OutdoorTrack,
    }
}
