use super::map::SchoolTable;
use crate::{CrawlError, CrawlResult};

const MAX_SOURCE_ROWS: usize = 20_000;

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

pub fn parse_table(table_html: &str) -> CrawlResult<Option<SchoolTable>> {
    let mut rows: Vec<(String, String)> = Vec::new();

    for chunk in table_html.split_inclusive("</tr>") {
        if chunk.strip_suffix("</tr>").is_none() {
            break;
        }
        let Some(open) = chunk.find("<tr") else {
            continue;
        };
        let Some(row) = chunk.get(open..) else {
            continue;
        };

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

        if rows.len() >= MAX_SOURCE_ROWS {
            return Err(CrawlError::Schema {
                url: super::HOST.to_string(),
                detail: format!("CIAC staff table exceeds {MAX_SOURCE_ROWS} coach rows"),
            });
        }
        rows.push((sport, name));
    }

    if rows.is_empty() {
        return Ok(None);
    }

    Ok(Some(SchoolTable { rows }))
}

pub fn parse_directory(html: &str) -> CrawlResult<Vec<(String, SchoolTable)>> {
    const OPENING: &str = "<table class='DirectoryStaffTable'>";
    const CLOSING: &str = "</table>";

    let mut results: Vec<(String, SchoolTable)> = Vec::new();
    let mut total_rows: usize = 0;

    for table_match in html.match_indices(OPENING) {
        let table_start = table_match.0;

        let name = find_school_name(html, table_start);
        if name.is_empty() {
            continue;
        }

        let Some(from_table) = html.get(table_start..) else {
            continue;
        };
        let end = from_table
            .get(OPENING.len()..)
            .and_then(|after_open| after_open.find(CLOSING))
            .and_then(|close| OPENING.len().checked_add(close))
            .and_then(|close| close.checked_add(CLOSING.len()))
            .map_or(from_table.len(), |value| value);

        let Some(table_html) = from_table.get(..end) else {
            continue;
        };
        let school_table = match parse_table(table_html)? {
            Some(table) => table,
            None => SchoolTable { rows: Vec::new() },
        };
        total_rows = total_rows
            .checked_add(school_table.rows.len())
            .ok_or_else(|| CrawlError::Arithmetic {
                detail: "CIAC directory row accounting overflowed".to_string(),
            })?;
        if total_rows > MAX_SOURCE_ROWS {
            return Err(CrawlError::Schema {
                url: super::HOST.to_string(),
                detail: format!("CIAC directory exceeds {MAX_SOURCE_ROWS} coach rows"),
            });
        }
        results.push((name, school_table));
    }

    Ok(results)
}

fn find_school_name(html: &str, table_pos: usize) -> String {
    name_before(html, table_pos).map_or(Default::default(), core::convert::identity)
}

fn name_before(html: &str, table_pos: usize) -> Option<String> {
    let before = html.get(..table_pos)?;
    let detail = before.get(before.rfind("<div class='DirectoryDetail'>")?..)?;
    let content = detail.get(detail.rfind("<b>")?..)?.strip_prefix("<b>")?;
    let name = content.get(..content.find("</b>")?)?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

fn extract_td_text(row: &str) -> (String, String, String) {
    let mut cells: [String; 3] = Default::default();
    let mut rest = row;

    for cell in &mut cells {
        let Some(open) = rest.find("<td") else {
            break;
        };
        let Some(from_td) = rest.get(open..) else {
            break;
        };
        let Some((text, after)) = from_td.split_once("</td>") else {
            break;
        };
        *cell = text.to_string();
        rest = after;
    }

    let [sport, role, name] = cells;
    (sport, role, name)
}
