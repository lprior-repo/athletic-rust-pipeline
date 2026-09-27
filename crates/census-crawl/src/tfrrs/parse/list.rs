use super::html::text_of;
use super::row::{parse_row, ParsedRow};
use census_domain::model::Gender;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedSection {
    pub label: String,
    pub gender: Option<Gender>,
    pub event_hnd: Option<u32>,
    pub rows: Vec<ParsedRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedList {
    pub sections: Vec<ParsedSection>,
}

pub fn parse_list_page(html: &str) -> ParsedList {
    let mut sections = Vec::new();
    for chunk in html.split("<div class=\"row gender_").skip(1) {
        sections.push(parse_section(chunk));
    }
    ParsedList { sections }
}

fn parse_section(chunk: &str) -> ParsedSection {
    let gender = section_gender(chunk);
    let event_hnd = chunk
        .find("standard_event_hnd_")
        .and_then(|start| chunk.get(start.checked_add("standard_event_hnd_".len())?..))
        .and_then(|rest| rest.split('"').next())
        .and_then(|digits| digits.parse::<u32>().ok());
    let label = section_label(chunk).unwrap_or_default();
    let mut rows = Vec::new();
    for body in chunk.split("<div class=\"performance-list-row").skip(1) {
        rows.push(parse_row(body));
    }
    ParsedSection {
        label,
        gender,
        event_hnd,
        rows,
    }
}

fn section_label(chunk: &str) -> Option<String> {
    let start = chunk.find("<h3")?;
    let after = chunk.get(start..)?;
    let body = after.get(after.find('>')?.checked_add(1)?..)?;
    let text = body.get(..body.find("</h3>")?)?;
    Some(strip_gender_parenthetical(&text_of(text)))
}

fn strip_gender_parenthetical(label: &str) -> String {
    let trimmed = label.trim();
    let Some(open) = trimmed.rfind('(') else {
        return trimmed.to_string();
    };
    let Some(inside) = open.checked_add(1).and_then(|offset| trimmed.get(offset..)) else {
        return trimmed.to_string();
    };
    let Some(word) = inside.strip_suffix(')') else {
        return trimmed.to_string();
    };
    let is_gender = matches!(
        word.trim().to_ascii_lowercase().as_str(),
        "men" | "women" | "m" | "w" | "boys" | "girls" | "boy" | "girl"
    );
    if is_gender {
        trimmed.get(..open).unwrap_or(trimmed).trim().to_string()
    } else {
        trimmed.to_string()
    }
}

fn section_gender(chunk: &str) -> Option<Gender> {
    match chunk.chars().next()? {
        'm' => Some(Gender::Boys),
        'f' => Some(Gender::Girls),
        _ => None,
    }
}
