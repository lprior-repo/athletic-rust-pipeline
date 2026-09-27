
use super::pages::CoachRow;
use super::{ASSOCIATION, HOST_WWW, SOURCE_ID, STATE};
use super::{DIRECTORY_PATH, STAFF_PATH};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};

pub struct ParsedSchool {
    pub name: String,
    pub school_id: String,
}

#[derive(Debug, Clone)]
pub struct CoachEntry {
    pub sport: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    pub source_school_id: String,
    pub coaches: Vec<CanonicalCoach>,
}

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

fn sport_gender(label: &str) -> Gender {
    let label = decode_entities(label);
    let label = collapse_whitespace(&label);
    match label {
        l if l.starts_with("Boys") || l.starts_with("Coed") => Gender::Boys,
        l if l.starts_with("Girls") => Gender::Girls,
        _ => Gender::Mixed,
    }
}

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

pub fn school_entities(
    entry: &ParsedSchool,
    staff_rows: &[CoachRow],
    observed_on: &str,
) -> SchoolExtract {
    let page_url = school_page_url(&entry.school_id);
    let staff_url = staff_page_url(&entry.school_id);
    let (school, school_id) = school_entity(entry, page_url, observed_on);

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
