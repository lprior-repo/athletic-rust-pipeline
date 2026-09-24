//! Pure parsing: the CIAC staff directory page into school tables.
//!
//! Each school on the page is a `<table class='DirectoryStaffTable'>` preceded by a
//! `<div class='DirectoryDetail'>` carrying the school name in `<b>…</b>`.
//!
//! Within a table:
//! * Header row — `<th>Role</th><th>Name</th><th>Phone</th>`
//! * Coach rows — `<td>Sport</td><td>Head Coach</td><td>Coach Name</td><td>Phone</td>`
//! * Non-coach rows — `<td></td><td>Principal</td><td>Name</td><td>Phone</td>`
//!
//! No I/O, no store access.

use super::map::SchoolTable;

/// Map a raw sport label to a Sport variant, returning None for non-XC/TF rows.
pub fn parse_sport_label(label: &str) -> Option<census_domain::model::Sport> {
    let l = label.trim().to_ascii_lowercase();
    match l.as_str() {
        "boys cross country" | "girls cross country" => {
            Some(census_domain::model::Sport::CrossCountry)
        }
        "boys indoor track" | "girls indoor track" => {
            Some(census_domain::model::Sport::IndoorTrack)
        }
        "boys outdoor track"
        | "girls outdoor track"
        | "coed outdoor track"
        | "unified outdoor track" => Some(census_domain::model::Sport::OutdoorTrack),
        _ => None,
    }
}

/// Derive Gender from a sport label like "Boys Cross Country" or "Girls Indoor Track".
pub fn parse_gender(label: &str) -> census_domain::model::Gender {
    let l = label.trim().to_ascii_lowercase();
    if l.starts_with("boys") {
        census_domain::model::Gender::Boys
    } else if l.starts_with("girls") {
        census_domain::model::Gender::Girls
    } else {
        census_domain::model::Gender::Mixed
    }
}

/// Strip HTML tags from a string.
fn strip_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

/// A name a source publishes where a coach's would be: no person holds this row.
///
/// Two callers need the same answer — [`parse_table`] drops the row, and the mapper refuses to mint
/// a coach from one — so the rule lives in one place. A second copy would be a second opinion, and
/// a placeholder that slipped past the mapper would land in the workbook as a real coach.
pub(super) fn is_placeholder_name(name: &str) -> bool {
    let name = name.trim();
    if name.eq_ignore_ascii_case("Team of one (Nicholai Dalidowitz)") {
        return true;
    }
    matches!(
        name.to_lowercase().as_str(),
        "unknown" | "tba" | "vacant" | "no team" | "no team." | "no.team" | "tbd" | "tbd."
    )
}

/// Parse one `<table class='DirectoryStaffTable'>` block into a SchoolTable.
pub fn parse_table(table_html: &str) -> Option<SchoolTable> {
    let mut rows: Vec<(String, String)> = Vec::new();

    for chunk in table_html.split_inclusive("</tr>") {
        if chunk.strip_suffix("</tr>").is_none() {
            break;
        }
        let Some(open) = chunk.find("<tr") else {
            continue;
        };
        let row = chunk.get(open..)?;

        // Skip header row
        if row.contains("<th>") {
            continue;
        }

        let (sport, _role, name) = extract_td_text(row);

        let sport = strip_tags(&sport).trim().to_string();
        if sport.is_empty() {
            continue;
        }

        let name = strip_tags(&name).trim().to_string();
        if name.is_empty() {
            continue;
        }

        if is_placeholder_name(&name) {
            continue;
        }

        rows.push((sport, name));
    }

    if rows.is_empty() {
        return None;
    }

    Some(SchoolTable { rows })
}

/// Parse the full directory page HTML into a list of school tables.
pub fn parse_directory(html: &str) -> Vec<(String, SchoolTable)> {
    let mut results: Vec<(String, SchoolTable)> = Vec::new();

    for table_match in html.match_indices("<table class='DirectoryStaffTable'>") {
        let table_start = table_match.0;

        let name = find_school_name(html, table_start);
        if name.is_empty() {
            continue;
        }

        let mut end = table_start + 38;
        if let Some(close) = html[end..].find("</table>") {
            end += close + 8;
        } else {
            end = html.len();
        }

        let table_html = &html[table_start..end];
        if let Some(school_table) = parse_table(table_html) {
            results.push((name, school_table));
        }
    }

    results
}

/// Find the school name from the <b> tag in the preceding DirectoryDetail div.
fn find_school_name(html: &str, table_pos: usize) -> String {
    let before = &html[..table_pos];
    if let Some(detail_end) = before.rfind("<div class='DirectoryDetail'>") {
        let detail = &before[detail_end..];
        if let Some(b_start) = detail.rfind("<b>") {
            let content = &detail[b_start + 3..];
            if let Some(b_end) = content.find("</b>") {
                let name = content[..b_end].trim();
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
    }
    String::new()
}

/// Extract Sport, Role, Name from a row's first three <td> cells.
///
/// Returns raw text including the `<td...>` tag prefix, which callers should
/// strip_tags before using.
fn extract_td_text(row: &str) -> (String, String, String) {
    let mut cells: [String; 3] = Default::default();
    let mut rest = row;

    for cell in &mut cells {
        let Some(open) = rest.find("<td") else {
            break;
        };
        let from_td = &rest[open..];
        let Some((text, after)) = from_td.split_once("</td>") else {
            break;
        };
        *cell = text.to_string();
        rest = after;
    }

    let [sport, role, name] = cells;
    (sport, role, name)
}
