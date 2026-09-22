//! The team page: its `ROSTER` table, and the season control it was published for.
//!
//! Only a table whose header states `NAME` and `YEAR` is read as a roster: a team page also
//! carries a meet-results table, and reading that one as a roster would publish meet columns as
//! grades.

use super::html::{links, text_of};
use super::route::{athlete_id, href_name, href_school};
use super::season::{season_from_label, Season, YearToken};
use census_domain::model::flip_last_first;

// -------------------------------------------------------------------------------------------------
// Team pages
// -------------------------------------------------------------------------------------------------

/// One athlete of a team page's `ROSTER` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterAthlete {
    pub id: Option<u64>,
    /// The link's display text, `Last, First` flipped to `First Last`.
    pub name: String,
    pub href_name: Option<String>,
    pub year: Option<YearToken>,
}

impl RosterAthlete {
    /// The full name the row states: the displayed name, the route slug otherwise.
    pub fn full_name(&self) -> Option<&str> {
        if self.name.is_empty() {
            self.href_name.as_deref()
        } else {
            Some(self.name.as_str())
        }
    }
}

/// A team page: the roster, the school and the season the page's season control states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRoster {
    pub athletes: Vec<RosterAthlete>,
    /// The school the roster's own athlete routes name (`Pembroke`).
    pub school: Option<String>,
    pub season: Option<Season>,
}

/// Read a team page: its season control, its school and its roster.
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

/// The `<tr>` chunks of the page's `ROSTER` table, or nothing when the page publishes no such table.
///
/// Only a table whose header states `NAME` and `YEAR` is accepted: a team page also carries a
/// meet-results table, and reading that one as a roster would publish meet columns as grades.
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

/// The text of every `<td>` cell in a table row, in order.
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

/// The season the page's own `config_hnd` control states.
///
/// The control lists every season the team has competed in, oldest last, with the page's own season
/// marked `selected`; the marked option is the one the roster was published for.
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

/// The text between an `<option …>` tag and its closing tag.
fn option_label(option: &str) -> Option<String> {
    let after = option.get(option.find('>')?.checked_add(1)?..)?;
    let body = after.get(..after.find("</option>")?)?;
    let label = text_of(body);
    (!label.is_empty()).then_some(label)
}
