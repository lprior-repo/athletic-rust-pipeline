pub(super) mod checked;

use super::map::{CoachRow, SchoolTable};
use census_domain::model::Sport;

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

fn extract_td_text(row: &str, index: usize) -> &str {
    row.split("</td>")
        .filter_map(|cell| cell.split_once("<td"))
        .nth(index)
        .and_then(|(_, cell)| cell.split_once('>'))
        .map_or("", |(_, content)| content)
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

fn parse_coach_name(cell: &str) -> String {
    let text = strip_tags(&decode_entities(cell));
    collapse_whitespace(&text)
}

fn parse_table_rows(table_html: &str) -> Vec<CoachRow> {
    table_html
        .split("</tr>")
        .filter(|row| row.contains("<tr"))
        .filter_map(parse_coach_row)
        .collect()
}

fn parse_coach_row(table_row: &str) -> Option<CoachRow> {
    let sport_label = parse_coach_name(extract_td_text(table_row, 0));
    let role = parse_coach_name(extract_td_text(table_row, 1));
    if !role.contains("Head Coach") {
        return None;
    }
    let sport = parse_sport_label(&sport_label)?;
    let coach_name = parse_coach_name(extract_td_text(table_row, 2));
    if coach_name.is_empty() {
        return None;
    }
    Some(CoachRow {
        sport_label,
        coach_name,
        phone: extract_phone_from_cell(extract_td_text(table_row, 3)),
        sport,
    })
}

fn extract_phone_from_cell(cell: &str) -> Option<String> {
    [("href=\"tel:", '"'), ("href='tel:", '\'')]
        .into_iter()
        .find_map(|(opening, closing)| {
            let (_, after) = cell.split_once(opening)?;
            let (phone, _) = after.split_once(closing)?;
            (!phone.trim().is_empty()).then(|| phone.to_string())
        })
}

pub fn parse_directory(html: &str) -> Vec<SchoolTable> {
    html.split("<details")
        .skip(1)
        .filter_map(|section| section.split_once("</details>"))
        .filter_map(|(section, _)| {
            let name = extract_school_name(section);
            (!name.is_empty()).then(|| SchoolTable {
                name,
                coach_rows: parse_table_rows(extract_table(section)),
            })
        })
        .collect()
}

fn extract_school_name(section: &str) -> String {
    for (opening, closing) in [("<summary>", "</summary>"), ("<b>", "</b>")] {
        if let Some(name) = tagged_text(section, opening, closing) {
            return name;
        }
    }

    String::new()
}

fn tagged_text(section: &str, opening: &str, closing: &str) -> Option<String> {
    let start = section.find(opening)?.checked_add(opening.len())?;
    let from_open = section.get(start..)?;
    let raw = from_open.get(..from_open.find(closing)?)?;
    let text = collapse_whitespace(&strip_tags(&decode_entities(raw)));
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn extract_table(section: &str) -> &str {
    const CLOSING: &str = "</table>";
    section
        .split_once("<table")
        .and_then(|(_, table)| table.split_once(CLOSING))
        .map_or("", |(table, _)| table)
}
