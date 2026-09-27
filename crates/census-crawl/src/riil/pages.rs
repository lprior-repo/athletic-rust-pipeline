
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

        if sport_label.is_empty() {
            continue;
        }

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

fn extract_phone_from_cell(cell: &str) -> Option<String> {
    if let Some(after) = cell.split_once("href=\"tel:") {
        let end = after.1.find('"').unwrap_or(after.1.len());
        return after.1.get(..end).map(str::to_string);
    }
    if let Some(after) = cell.split_once("href='tel:") {
        let end = after.1.find('\'').unwrap_or(after.1.len());
        return after.1.get(..end).map(str::to_string);
    }
    None
}

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

fn extract_table(section: &str) -> String {
    const CLOSING: &str = "</table>";

    let Some(start) = section.find("<table") else {
        return String::new();
    };
    let Some(from_table) = section.get(start..) else {
        return String::new();
    };
    let Some(table) = from_table
        .find(CLOSING)
        .and_then(|close| close.checked_add(CLOSING.len()))
        .and_then(|end| from_table.get(..end))
    else {
        return String::new();
    };

    table.to_string()
}
