use census_domain::model::{CoachRole, Gender, Sport};

pub const SCHOOL_ARRAY_START: &str = "source: [ ";

pub const SCHOOL_ARRAY_END: &str = "],";

pub const ARRAY_ENTRY: &str = r#"\{id: '([0-9]+)', name: '((?:[^'\\]|\\.)*)'\}"#;

pub const H2: &str = r"(?s)<h2[^>]*>(.*?)</h2>";

pub const CARD_HEADER: &str = r#"(?s)<div class="card-header[^"]*">\s*([^<]*?)\s*</div>"#;

pub const STAFF_ROW: &str = r#"(?s)<tr[^>]*class="[^"]*staffPerson[^"]*"[^>]*>(.*?)</tr>"#;

pub const CELL: &str = r"(?s)<td[^>]*>(.*?)</td>";

pub const STAFF_ID: &str = r#"class="staff([0-9A-Za-z]+)""#;

pub const MAIL_HIDE: &str = r#"mail_hide\("([0-9A-Za-z]+)",\s*"([^"]*)",\s*[0-9]+,\s*"([^"]*)"\)"#;

pub const TAG: &str = r"(?s)<[^>]*>";

pub fn array_body(text: &str) -> Option<(usize, usize)> {
    let start = text.find(SCHOOL_ARRAY_START)?;
    let body_start = start.saturating_add(SCHOOL_ARRAY_START.len());
    let end = text.get(body_start..)?.find(SCHOOL_ARRAY_END)?;
    Some((body_start, body_start.saturating_add(end)))
}

pub fn strip_tags(tag: &regex::Regex, raw: &str) -> String {
    tag.replace_all(raw, " ").trim().to_string()
}

pub fn unescape_js(raw: &str) -> String {
    raw.replace("\\'", "'")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
        .trim()
        .to_string()
}

pub fn split_name_and_city(raw: &str) -> (String, Option<(String, String)>) {
    let text = raw.trim();
    let Some(open) = text.rfind(" (") else {
        return (text.to_string(), None);
    };
    let Some(body) = text.strip_suffix(')') else {
        return (text.to_string(), None);
    };
    let Some(inner) = body.get(open.saturating_add(2)..) else {
        return (text.to_string(), None);
    };
    let mut parts = inner.split(',');
    let (Some(city), Some(state)) = (parts.next(), parts.next()) else {
        return (text.to_string(), None);
    };
    if parts.next().is_some() {
        return (text.to_string(), None);
    }
    let city = city.trim();
    let state = state.trim();
    if city.is_empty() || state.len() != 2 {
        return (text.to_string(), None);
    }
    let name = text.get(..open).unwrap_or("").trim().to_string();
    (name, Some((city.to_string(), state.to_string())))
}

pub fn decode_email(username: &str, reversed_domain: &str) -> Option<String> {
    if username.is_empty() {
        return None;
    }
    let domain: String = reversed_domain.chars().rev().collect();
    if !domain.contains('.') {
        return None;
    }
    Some(format!("{username}@{domain}"))
}

pub fn classify(header: &str) -> Option<(Option<Sport>, Gender)> {
    let text = header.trim();
    if text.is_empty() {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    let gender = if lower.starts_with("boys' and girls'") || lower.starts_with("girls' and boys'") {
        Gender::Mixed
    } else if lower.starts_with("boys'") {
        Gender::Boys
    } else if lower.starts_with("girls'") {
        Gender::Girls
    } else {
        Gender::Unknown
    };
    if lower.contains("cross country") {
        return Some((Some(Sport::CrossCountry), gender));
    }
    if lower.contains("track") {
        return Some((Some(Sport::OutdoorTrack), gender));
    }
    if lower.contains("administration") {
        return Some((None, Gender::Unknown));
    }
    None
}

pub fn role_of(title: &str) -> Option<CoachRole> {
    match title.trim() {
        "Head Coach" => Some(CoachRole::HeadCoach),
        "Assistant Coach" => Some(CoachRole::AssistantCoach),
        "Athletic Director" => Some(CoachRole::AthleticDirector),
        _ => None,
    }
}

pub fn header_for(cards: &[(usize, String)], offset: usize) -> &str {
    cards
        .iter()
        .rfind(|entry| entry.0 < offset)
        .map(|(_, header)| header.as_str())
        .unwrap_or("")
}
