//! Canonical mapping: parsed rows -> canonical entities from `census_domain::model`.
//!
//! Nothing here reads the network, the store or a file: it takes the parsed rows from
//! [`super::pages`] and returns the canonical types.
//!
//! School join key: the source publishes no numeric id, so the normalized school name
//! is the canonical join key. Every school minted from this adapter uses
//! `normalize_name(name)` as its natural key, which means two schools with the same
//! normalized name (e.g. "Central High School" and "Central Sr. High School") will
//! collide if their normalized forms are identical. The source does not publish
//! separate ids for such schools, so this is an inherent limitation.

use super::pages::CoachRow;
use super::{ASSOCIATION, HOST_WWW, SOURCE_ID, STATE};
use super::{DIRECTORY_PATH, STAFF_PATH};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};

// ── Parsed shapes ──────────────────────────────────────────────────────────

/// A parsed school with its name and FusionPoint ID (used only for URL construction).
///
/// The school's canonical join key is its normalized name — the source publishes no
/// stable numeric id for schools, so `normalize_name(name)` is the only deterministic
/// key we can use to deduplicate across adapters.
pub struct ParsedSchool {
    /// Display name as published.
    pub name: String,
    /// FusionPoint school id (used for staff page URL, not as canonical key).
    pub school_id: String,
}

/// One coach extracted from a staff table row.
#[derive(Debug, Clone)]
pub struct CoachEntry {
    /// Sport label as published (e.g. "Boys Cross Country").
    pub sport: String,
    /// Head-coach name.
    pub name: String,
}

/// Canonical entities for one school.
#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    /// FusionPoint school id used for journal keys and URL construction.
    pub source_school_id: String,
    pub coaches: Vec<CanonicalCoach>,
}

// ── Sport label mapping ────────────────────────────────────────────────────

/// Map a sport label to a Sport variant, or `None` for non-XC/TF sports.
pub fn parse_sport_label(label: &str) -> Option<Sport> {
    let cleaned = collapse_whitespace(&decode_entities(label));
    match cleaned.as_str() {
        "Boys Cross Country" | "Girls Cross Country" | "Coed Cross Country" => {
            Some(Sport::CrossCountry)
        }
        "Boys Indoor Track" | "Girls Indoor Track" | "Coed Indoor Track" | "Boys Indoor"
        | "Girls Indoor" | "Coed Indoor" => Some(Sport::IndoorTrack),
        "Boys Outdoor Track"
        | "Girls Outdoor Track"
        | "Coed Outdoor Track"
        | "Boys Outdoor"
        | "Girls Outdoor"
        | "Coed Outdoor" => Some(Sport::OutdoorTrack),
        _ => None,
    }
}

/// Map a sport label to gender.
fn sport_gender(label: &str) -> Gender {
    let label = decode_entities(label);
    let label = collapse_whitespace(&label);
    match label {
        l if l.starts_with("Boys") || l.starts_with("Coed") => Gender::Boys,
        l if l.starts_with("Girls") => Gender::Girls,
        _ => Gender::Mixed,
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn decode_entities(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#039;", "'")
        .replace("&nbsp;", " ")
}

fn collapse_whitespace(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut prev_space = false;
    for ch in value.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(ch);
            prev_space = false;
        }
    }
    result.trim().to_string()
}

fn strip_honorific(name: &str) -> String {
    let name = name.trim();
    for prefix in ["Coach ", "Mr. ", "Mrs. ", "Ms. ", "Dr. "] {
        if let Some(stripped) = name.strip_prefix(prefix) {
            return stripped.trim().to_string();
        }
    }
    name.to_string()
}

fn school_page_url(school_id: &str) -> String {
    format!("{HOST_WWW}{DIRECTORY_PATH}?SchoolID={}", school_id)
}

fn staff_page_url(school_id: &str) -> String {
    format!("{HOST_WWW}{STAFF_PATH}?SchoolID={}&tab=staff", school_id)
}

// ── Entity mapping ─────────────────────────────────────────────────────────

/// Build canonical school and coach entities from parsed directory entries and staff rows.
///
/// `all_entries` is the full directory (all schools), `school_entries` is the subset
/// we are processing (filtered by limit/names), and `staff_rows` is the coach data
/// for the current school.
pub fn school_entities(
    entry: &ParsedSchool,
    staff_rows: &[CoachRow],
    observed_on: &str,
) -> SchoolExtract {
    let page_url = school_page_url(&entry.school_id);
    let staff_url = staff_page_url(&entry.school_id);
    let (school, school_id) = school_entity(entry, page_url, observed_on);

    // Deduplicate: the source sometimes lists the same sport twice (e.g. two boys XC coaches), and
    // the first occurrence is the published one.
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    for row in staff_rows {
        let Some(sport) = parse_sport_label(&row.sport) else {
            continue;
        };
        if !seen.insert(format!("{sport:?}:{}", row.sport)) {
            continue;
        }
        coaches.push(coach_entity(
            entry,
            &school_id,
            row,
            sport,
            &staff_url,
            observed_on,
        ));
    }

    SchoolExtract {
        school,
        school_id,
        source_school_id: entry.school_id.clone(),
        coaches,
    }
}

/// Mint the school row one directory entry names.
fn school_entity(
    entry: &ParsedSchool,
    page_url: String,
    observed_on: &str,
) -> (CanonicalSchool, SchoolId) {
    let norm = normalize_name(&entry.name);
    let (mut school, school_id) = CanonicalSchool::new(STATE, &entry.name, &norm);
    school.association = Some(ASSOCIATION.to_string());
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ASSOCIATION.to_string(),
            },
            format!("mpa:{}", entry.school_id),
        )
        .with_url(page_url.clone()),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(page_url)),
        observed_on.to_string(),
    ));
    (school, school_id)
}

/// Mint the head coach one staff row names.
fn coach_entity(
    entry: &ParsedSchool,
    school_id: &SchoolId,
    row: &CoachRow,
    sport: Sport,
    staff_url: &str,
    observed_on: &str,
) -> CanonicalCoach {
    let gender = sport_gender(&row.sport);
    let mut coach = CanonicalCoach::new(
        school_id,
        strip_honorific(&row.coach),
        Some(sport),
        gender,
        CoachRole::HeadCoach,
    );
    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ASSOCIATION.to_string(),
            },
            format!(
                "coach:{}:{}:{}",
                entry.school_id,
                sport.stable_key(),
                gender.stable_key()
            ),
        )
        .with_url(staff_url.to_string()),
    );
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(staff_url.to_string())),
        observed_on.to_string(),
    ));
    coach
}
