//! The performance-list page and its sections.
//!
//! The page is one document per list *view*: the same markup serves the unfiltered page (the
//! host's per-section top-N over every grade) and a `?year=<TOKEN>` view (that grade's own
//! ranking), so the sections are read the same way either way and the row's own `Year` cell
//! stays authoritative.

use super::html::text_of;
use super::row::{parse_row, ParsedRow};
use census_domain::model::Gender;

// -------------------------------------------------------------------------------------------------
// Performance lists
// -------------------------------------------------------------------------------------------------

/// One event × gender section of a performance list.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedSection {
    /// The event label with its gender parenthetical removed (`60 Meters`, `4 x 200 Relay`).
    pub label: String,
    pub gender: Option<Gender>,
    /// The host's own standard-event handle (`standard_event_hnd_46`), kept so a row's identity
    /// stays stable even if the printed label is reworded.
    pub event_hnd: Option<u32>,
    pub rows: Vec<ParsedRow>,
}

/// The parsed sections of one performance-list page.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedList {
    pub sections: Vec<ParsedSection>,
}

// -------------------------------------------------------------------------------------------------
// Performance-list parsing
// -------------------------------------------------------------------------------------------------

/// Read every section of a performance-list page.
///
/// The page is one document per list *view*: the same markup serves the unfiltered page (the host's
/// per-section top-N over every grade) and a `?year=<TOKEN>` view (that grade's own ranking), so the
/// sections are read the same way either way and the row's own `Year` cell stays authoritative.
pub fn parse_list_page(html: &str) -> ParsedList {
    let mut sections = Vec::new();
    for chunk in html.split("<div class=\"row gender_").skip(1) {
        sections.push(parse_section(chunk));
    }
    ParsedList { sections }
}

/// One `<div class="row gender_… standard_event_hnd_…">` block: its event, its gender side, its rows.
fn parse_section(chunk: &str) -> ParsedSection {
    let gender = section_gender(chunk);
    let event_hnd = chunk
        .find("standard_event_hnd_")
        .and_then(|start| chunk.get(start.checked_add("standard_event_hnd_".len())?..))
        .and_then(|rest| rest.split('"').next())
        .and_then(|digits| digits.parse::<u32>().ok());
    let label = section_label(chunk).unwrap_or_default();
    let mut rows = Vec::new();
    // The text before the first row is the section's own title and table header.
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

/// The section title (`60 Meters (Men)`), with a trailing gender parenthetical dropped.
fn section_label(chunk: &str) -> Option<String> {
    let start = chunk.find("<h3")?;
    let after = chunk.get(start..)?;
    let body = after.get(after.find('>')?.checked_add(1)?..)?;
    let text = body.get(..body.find("</h3>")?)?;
    Some(strip_gender_parenthetical(&text_of(text)))
}

/// Drop a trailing `(Men)`/`(Women)`/`(M)`/`(W)` from a section title, leaving the event label.
///
/// The gender itself is read from the section's own `gender_m`/`gender_f` class and from the team
/// slug, both of which are single-letter and unambiguous; this only removes the title's restatement
/// of it so the event label matches the ontology's compact keys.
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

/// The single-letter gender side a section wrapper states.
///
/// The split that reaches this parser has already consumed `row gender_`, so the chunk opens with
/// the side itself; the title's restatement of it (`60 Meters (Men)`) is only stripped from the
/// label, never read as the side.
fn section_gender(chunk: &str) -> Option<Gender> {
    match chunk.chars().next()? {
        'm' => Some(Gender::Boys),
        'f' => Some(Gender::Girls),
        _ => None,
    }
}
