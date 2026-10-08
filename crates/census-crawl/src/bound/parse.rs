use census_domain::model::Gender;
use census_domain::model::Sport;

#[derive(Debug, Clone)]
pub struct CoachRow {
    pub role: String,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct PageInfo {
    pub title_school: String,
    pub title_sport: String,
}

#[derive(Debug, Clone, Default)]
pub struct ParseResult {
    pub page: PageInfo,
    pub coaches: Vec<CoachRow>,
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

fn trim_ws(s: &str) -> String {
    s.trim().to_string()
}

fn clean(value: &str) -> String {
    trim_ws(&strip_tags(value))
}

fn parse_title(html: &str) -> PageInfo {
    let mut info = PageInfo::default();
    let Some((_, after_open)) = html.split_once("<title>") else {
        return info;
    };
    let Some((raw, _)) = after_open.split_once("</title>") else {
        return info;
    };
    let text = clean(raw);
    let mut parts = text.split('|').skip(1);
    info.title_school = parts.next().map_or_else(String::new, trim_ws);
    info.title_sport = parts.next().map_or_else(String::new, trim_ws);
    info
}

fn cell_body(chunk: &str) -> Option<&str> {
    let (_, rest) = chunk.split_once('>')?;
    Some(rest.split_once("</td>").map_or(rest, |(body, _)| body))
}

fn row_cells(row: &str) -> Vec<&str> {
    let mut cells = Vec::new();
    for chunk in row.split("<td").skip(1) {
        let Some(cell) = cell_body(chunk) else {
            continue;
        };
        cells.push(cell);
        if cells.len() == 2 {
            break;
        }
    }
    cells
}

fn map_role(text: &str) -> Option<census_domain::model::CoachRole> {
    let lower = text.to_lowercase();
    if lower == "head coach" || lower == "co-head coach" {
        Some(census_domain::model::CoachRole::HeadCoach)
    } else if lower == "assistant coach" || lower == "volunteer coach" {
        Some(census_domain::model::CoachRole::AssistantCoach)
    } else {
        None
    }
}

fn is_header_row(role_text: &str) -> bool {
    let lower = role_text.to_lowercase();
    lower == "position" || lower == "name" || lower == "level" || lower.contains("level")
}

pub fn parse(html: &str) -> ParseResult {
    let mut result = ParseResult {
        page: parse_title(html),
        ..ParseResult::default()
    };

    if let Some((_, after_heading)) = html.split_once("Coaching Staff") {
        let table_section = after_heading
            .split_once("</table>")
            .map_or(after_heading, |(rows, _)| rows);

        for row in table_section.split("<tr") {
            let row = row.split_once('>').map_or(row, |(_, rest)| rest);
            let cells = row_cells(row);
            if cells.len() != 2 {
                continue;
            }
            let (Some(role_cell), Some(name_cell)) = (cells.first(), cells.get(1)) else {
                continue;
            };
            let role_text = clean(role_cell);
            let name = clean(name_cell);
            if name.is_empty() || is_header_row(&role_text) {
                continue;
            }
            if map_role(&role_text).is_none() {
                continue;
            }
            result.coaches.push(CoachRow {
                role: role_text,
                name,
            });
        }
    }

    result
}

pub fn parse_sport_label(label: &str) -> Option<Sport> {
    let l = label.trim().to_lowercase();
    if l.contains("cross country") {
        Some(Sport::CrossCountry)
    } else if l.contains("indoor track") {
        Some(Sport::IndoorTrack)
    } else if l.contains("outdoor track")
        || l.contains("track & field")
        || l.contains("track and field")
        || l.contains("trackfield")
    {
        Some(Sport::OutdoorTrack)
    } else {
        None
    }
}

pub fn parse_gender(label: &str) -> Gender {
    let l = label.trim().to_lowercase();
    if l.starts_with("boys") {
        Gender::Boys
    } else if l.starts_with("girls") {
        Gender::Girls
    } else {
        Gender::Mixed
    }
}
