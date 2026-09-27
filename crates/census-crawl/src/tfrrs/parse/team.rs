
use super::html::{links, text_of};
use super::route::{athlete_id, href_name, href_school};
use super::season::{season_from_label, Season, YearToken};
use census_domain::model::flip_last_first;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterAthlete {
    pub id: Option<u64>,
    pub name: String,
    pub href_name: Option<String>,
    pub year: Option<YearToken>,
}

impl RosterAthlete {
    pub fn full_name(&self) -> Option<&str> {
        if self.name.is_empty() {
            self.href_name.as_deref()
        } else {
            Some(self.name.as_str())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRoster {
    pub athletes: Vec<RosterAthlete>,
    pub school: Option<String>,
    pub season: Option<Season>,
}

pub fn parse_team_page(html: &str) -> ParsedRoster {
    let season = selected_season(html);
    let mut athletes = Vec::new();
    let mut school = None;
    for row in roster_rows(html) {
        let link = links(row).into_iter().next();
        let cells = cell_texts(row);
        let year = cells.get(1).and_then(|cell| YearToken::parse(cell));
        let (id, href, text) = match &link {
            Some((href, text)) => (athlete_id(href), Some(*href), text.as_str()),
            None => (None, None, ""),
        };
        if school.is_none() {
            school = href.and_then(href_school);
        }
        athletes.push(RosterAthlete {
            id,
            name: flip_last_first(text),
            href_name: href.and_then(href_name),
            year,
        });
    }
    ParsedRoster {
        athletes,
        school,
        season,
    }
}

fn roster_rows(html: &str) -> Vec<&str> {
    let Some(heading) = html.find(">ROSTER</h3>") else {
        return Vec::new();
    };
    let Some(after_heading) = html.get(heading..) else {
        return Vec::new();
    };
    let Some(table_open) = after_heading.find("<table") else {
        return Vec::new();
    };
    let Some(after_open) = after_heading.get(table_open..) else {
        return Vec::new();
    };
    let Some(table) = after_open
        .find("</table>")
        .and_then(|end| after_open.get(..end))
    else {
        return Vec::new();
    };
    let header = match (table.find("<thead"), table.find("</thead>")) {
        (Some(open), Some(close)) if open < close => table.get(open..close).unwrap_or_default(),
        _ => return Vec::new(),
    };
    let header_text = text_of(header).to_ascii_uppercase();
    if !header_text.contains("NAME") || !header_text.contains("YEAR") {
        return Vec::new();
    }
    table
        .split("<tr")
        .skip(1)
        .filter(|row| row.contains("athletes/"))
        .collect()
}

fn cell_texts(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    for cell in row.split("<td").skip(1) {
        let text = cell
            .find('>')
            .and_then(|open| cell.get(open.checked_add(1)?..))
            .and_then(|body| body.get(..body.find("</td>")?))
            .map(text_of)
            .unwrap_or_default();
        cells.push(text);
    }
    cells
}

fn selected_season(html: &str) -> Option<Season> {
    let start = html.find("name=\"config_hnd\"")?;
    let after = html.get(start..)?;
    let select = after
        .find("</select>")
        .and_then(|end| after.get(..end))
        .unwrap_or(after);
    let selected = select
        .split("<option")
        .skip(1)
        .find(|option| option.contains("selected"))?;
    let label = option_label(selected)?;
    season_from_label(&label)
}

fn option_label(option: &str) -> Option<String> {
    let after = option.get(option.find('>')?.checked_add(1)?..)?;
    let body = after.get(..after.find("</option>")?)?;
    let label = text_of(body);
    (!label.is_empty()).then_some(label)
}
