//! Pure parsing: the RIIL directory page into [`SchoolTable`] structs.
//!
//! Captured text in, parsed rows out; no I/O, no store access.
//!
//! Source shape: each `<details>` element contains a school's name (in `<summary>` and `<b>`),
//! contact details, and one `<table class="DirectoryStaffTable">` whose rows are staff/coach
//! entries with columns: Sport | Role | Name | Phone.

use super::map::{CoachRow, SchoolTable};
use census_domain::model::Sport;

// ── Helpers ────────────────────────────────────────────────────────────────

/// Remove HTML tags, keeping one space where a tag separated two words.
fn strip_tags(fragment: &str) -> String {
    let mut out = String::with_capacity(fragment.len());
    let mut in_tag = false;
    for ch in fragment.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                if !out.is_empty() && !out.ends_with(' ') {
                    out.push(' ');
                }
            }
            _ => {
                if !in_tag {
                    out.push(ch);
                }
            }
        }
    }
    out
}

/// Decode common HTML entities.
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

/// Collapse every run of whitespace to one space and trim the ends.
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

/// Extract text from the Nth `<td>` in a row (no tag stripping).
fn extract_td_text(row: &str, index: usize) -> String {
    let mut count = 0;
    let mut rest = row;
    while let Some(open) = rest.find("<td") {
        let Some(from_td) = rest.get(open..) else {
            break;
        };
        let Some((text, after)) = from_td.split_once("</td>") else {
            break;
        };
        if count == index {
            let content = text
                .strip_prefix("<td")
                .unwrap_or(text)
                .strip_prefix(">")
                .unwrap_or("");
            return content.to_string();
        }
        rest = after;
        count = count.saturating_add(1);
    }
    String::new()
}

// ── Sport parsing ──────────────────────────────────────────────────────────

/// Map a sport label to a `Sport` variant, or `None` for non-TC/XC sports.
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

/// Extract the coach name from a cell, stripping any phone anchor.
fn parse_coach_name(cell: &str) -> String {
    let text = strip_tags(&decode_entities(cell));
    collapse_whitespace(&text)
}

// ── Parsing ────────────────────────────────────────────────────────────────

/// Parse one school's `<table class='DirectoryStaffTable'>` into coach rows.
fn parse_table_rows(table_html: &str) -> Vec<CoachRow> {
    let mut rows = Vec::new();
    let table_rows: Vec<&str> = table_html
        .split("</tr>")
        .filter(|r| r.contains("<tr"))
        .collect();

    for table_row in table_rows {
        let sport_raw = extract_td_text(table_row, 0);
        let role_raw = extract_td_text(table_row, 1);
        let name_raw = extract_td_text(table_row, 2);
        let phone_raw = extract_td_text(table_row, 3);

        let sport_label = collapse_whitespace(&decode_entities(&sport_raw));
        let role = collapse_whitespace(&decode_entities(&role_raw));

        // Only coach rows: first cell (sport) must be non-empty
        if sport_label.is_empty() {
            continue;
        }

        // Filter to only head coach rows
        if !role.contains("Head Coach") {
            continue;
        }

        let coach_name = parse_coach_name(&name_raw);
        let phone = extract_phone_from_cell(&phone_raw);

        if let Some(sport) = parse_sport_label(&sport_label) {
            rows.push(CoachRow {
                sport_label,
                coach_name,
                phone,
                sport,
            });
        }
    }

    rows
}

/// Extract a phone number from a cell that may contain <a href='tel:...'>.
fn extract_phone_from_cell(cell: &str) -> Option<String> {
    if let Some(after) = cell.split_once("href=\"tel:") {
        let end = after.1.find('"').unwrap_or(after.1.len());
        return Some(after.1[..end].to_string());
    }
    if let Some(after) = cell.split_once("href='tel:") {
        let end = after.1.find('\'').unwrap_or(after.1.len());
        return Some(after.1[..end].to_string());
    }
    None
}

/// Parse the full directory page into one [`SchoolTable`] per `<details>` element.
pub fn parse_directory(html: &str) -> Vec<SchoolTable> {
    let mut schools = Vec::new();

    let sections: Vec<&str> = html.split("</details>").collect();

    for section in sections {
        let school_name = extract_school_name(section);

        if school_name.is_empty() {
            continue;
        }

        let table_html = extract_table(section);
        let coach_rows = parse_table_rows(table_html.as_str());

        schools.push(SchoolTable {
            name: school_name,
            coach_rows,
        });
    }

    schools
}

/// Extract the school name from a section (from <summary> or <b>).
fn extract_school_name(section: &str) -> String {
    // Try <summary> first
    if let Some(start) = section.find("<summary>") {
        let inner_start = start + 9; // len("<summary>")
        if let Some(end) = section[inner_start..].find("</summary>") {
            let end = inner_start + end;
            let raw = &section[inner_start..end];
            let text = strip_tags(&decode_entities(raw));
            let collapsed = collapse_whitespace(&text);
            if !collapsed.is_empty() {
                return collapsed;
            }
        }
    }

    // Fallback: look for <b> tag
    if let Some(start) = section.find("<b>") {
        let inner_start = start + 3; // len("<b>")
        if let Some(end) = section[inner_start..].find("</b>") {
            let end = inner_start + end;
            let raw = &section[inner_start..end];
            let text = strip_tags(&decode_entities(raw));
            let collapsed = collapse_whitespace(&text);
            if !collapsed.is_empty() {
                return collapsed;
            }
        }
    }

    String::new()
}

/// Extract the `<table>` content from a section.
fn extract_table(section: &str) -> String {
    if let Some(start) = section.find("<table") {
        if let Some(end) = section[start..].find("</table>") {
            let end = start + end + 8; // len("</table>")
            return section[start..end].to_string();
        }
    }
    String::new()
}
