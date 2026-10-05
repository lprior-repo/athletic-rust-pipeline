use super::entities::decode_entities;
use census_domain::model::{Gender, Sport};
use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchoolLink {
    pub id: u64,
    pub name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchoolProfile {
    pub id: u64,
    pub name: String,
    pub address: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub league: String,
    pub phone: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoachRow {
    pub user_id: u64,
    pub sport_id: u64,
    pub name: String,
    pub sport: String,
    pub role: String,
    pub email: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FacultyRow {
    pub id: u64,
    pub name: String,
    pub role: String,
    pub email: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchoolDetails {
    pub profile: SchoolProfile,
    pub coaches: Vec<CoachRow>,
    pub faculties: Vec<FacultyRow>,
}

#[derive(Debug, Deserialize)]
struct WireDetails {
    #[serde(default)]
    school: WireSchool,
    #[serde(default)]
    coaches: Vec<WireCoach>,
    #[serde(rename = "athleticFaculties", default)]
    athletic_faculties: Vec<WireFaculty>,
}

#[derive(Debug, Default, Deserialize)]
struct WireSchool {
    #[serde(default)]
    id: Option<u64>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    address_line_1: Option<String>,
    #[serde(default)]
    city: Option<String>,
    #[serde(default)]
    physical_state: Option<String>,
    #[serde(default)]
    physical_zip: Option<String>,
    #[serde(default)]
    league_name: Option<String>,
    #[serde(default)]
    phone: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct WireCoach {
    #[serde(default)]
    id: Option<u64>,
    #[serde(default)]
    user_id: Option<u64>,
    #[serde(default)]
    sport_id: Option<u64>,
    #[serde(default)]
    firstname: Option<String>,
    #[serde(default)]
    lastname: Option<String>,
    #[serde(default)]
    sport: Option<String>,
    #[serde(default)]
    aft_name: Option<String>,
    #[serde(default)]
    email: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct WireFaculty {
    #[serde(default)]
    id: Option<u64>,
    #[serde(default)]
    firstname: Option<String>,
    #[serde(default)]
    lastname: Option<String>,
    #[serde(default)]
    aft_name: Option<String>,
    #[serde(default)]
    email: Option<String>,
}

pub fn parse_directory_links(html: &str) -> Vec<SchoolLink> {
    let mut links: Vec<SchoolLink> = Vec::new();
    let mut cursor = 0usize;
    while let Some(relative) = find_from(html, cursor, "data-id=\"") {
        let Some(at) = cursor.checked_add(relative) else {
            break;
        };
        let Some(button) = html.get(cursor..at).and_then(|head| head.rfind("<button")) else {
            cursor = at.saturating_add(9);
            continue;
        };
        let Some(tag_open) = cursor.checked_add(button) else {
            break;
        };
        let straddles = html
            .get(tag_open..at)
            .is_some_and(|head| head.contains("</button>"));
        if straddles {
            cursor = at.saturating_add(9);
            continue;
        }
        let Some(close) = find_from(html, at, ">") else {
            break;
        };
        let value_start = at.saturating_add(9);
        let Some(quote) = find_from(html, value_start, "\"") else {
            break;
        };
        let Some(value_end) = value_start.checked_add(quote) else {
            break;
        };
        let Some(body_close) = find_from(html, value_end, "</button>") else {
            cursor = value_end;
            continue;
        };
        let Some(body_end) = value_end.checked_add(body_close) else {
            break;
        };
        let id = html
            .get(value_start..value_end)
            .and_then(|value| value.parse::<u64>().ok());
        if let Some(id) = id {
            let body_start = at.saturating_add(close).saturating_add(1);
            let name = html
                .get(body_start..body_end)
                .map_or_else(String::new, |body| {
                    decode_entities(&collapse_whitespace(body))
                });
            if !name.is_empty() && !links.iter().any(|link| link.id == id) {
                links.push(SchoolLink { id, name });
            }
        }
        cursor = body_end;
        if cursor >= html.len() {
            break;
        }
    }
    links
}

fn find_from(haystack: &str, from: usize, needle: &str) -> Option<usize> {
    haystack.get(from..).and_then(|tail| tail.find(needle))
}

pub fn parse_school_details(json: &str) -> Option<SchoolDetails> {
    let wire: WireDetails = serde_json::from_str(json).ok()?;
    let profile = SchoolProfile {
        id: wire.school.id.map_or(0, |value| value),
        name: text(wire.school.name),
        address: text(wire.school.address_line_1),
        city: text(wire.school.city),
        state: text(wire.school.physical_state),
        zip: text(wire.school.physical_zip),
        league: text(wire.school.league_name),
        phone: text(wire.school.phone),
    };
    let coaches = wire.coaches.into_iter().map(coach_row).collect();
    let faculties = wire
        .athletic_faculties
        .into_iter()
        .map(faculty_row)
        .collect();
    Some(SchoolDetails {
        profile,
        coaches,
        faculties,
    })
}

pub fn parse_sport_and_gender(label: &str) -> Option<(Sport, Gender)> {
    let lowered = label.to_ascii_lowercase();
    let sport = if lowered.contains("cross country") {
        Sport::CrossCountry
    } else if lowered.contains("track & field") || lowered.contains("track and field") {
        Sport::OutdoorTrack
    } else {
        return None;
    };
    let gender = if lowered.contains(", boys") {
        Gender::Boys
    } else if lowered.contains(", girls") {
        Gender::Girls
    } else {
        Gender::Unknown
    };
    Some((sport, gender))
}

fn coach_row(wire: WireCoach) -> CoachRow {
    CoachRow {
        user_id: wire.user_id.or(wire.id).map_or(0, core::convert::identity),
        sport_id: wire.sport_id.map_or(0, core::convert::identity),
        name: join_name(wire.firstname.as_deref(), wire.lastname.as_deref()),
        sport: text(wire.sport),
        role: text(wire.aft_name),
        email: text(wire.email),
    }
}

fn faculty_row(wire: WireFaculty) -> FacultyRow {
    FacultyRow {
        id: wire.id.map_or(0, core::convert::identity),
        name: join_name(wire.firstname.as_deref(), wire.lastname.as_deref()),
        role: text(wire.aft_name),
        email: text(wire.email),
    }
}

fn join_name(first: Option<&str>, last: Option<&str>) -> String {
    let parts: Vec<&str> = [first, last]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect();
    parts.join(" ")
}

fn text(value: Option<String>) -> String {
    value.into_iter().collect()
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
