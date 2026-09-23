//! One `performance-list-row`: the cells a row publishes, individual and relay alike.
//!
//! A relay row publishes its members in a `col-athletes` cell and its mark for the team; an
//! individual row publishes one athlete and that athlete's own mark. Both carry the same cells
//! for place, team, year, mark, meet and date, so those are read once.

use super::date::{published_date, PublishedDate};
use super::html::{cell, links, text_of};
use super::mark::{conversion_note, converted_metres, published_mark, wind_mps, ParsedMark};
use super::route::{athlete_id, href_name, meet_id, parse_team_path};
use super::season::YearToken;
use census_domain::model::{flip_last_first, Gender};

/// One published row: an individual mark or a relay's mark.
///
/// `PartialEq` without `Eq`: two columns of a row are the host's own conversions
/// (`conv_metres`, `wind`), which the domain models as floats.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedRow {
    pub place: Option<u16>,
    /// The single athlete of an individual row; `None` on a relay row.
    pub athlete: Option<ParsedAthlete>,
    /// The members a relay row lists; empty on an individual row.
    pub relay_members: Vec<ParsedAthlete>,
    pub team: Option<ParsedTeam>,
    /// The grade the row's own `Year` column publishes, when it publishes one. The column is empty
    /// on rows the host tracks without a class year — those are the rows the `?year=` filter can
    /// still place (`map.rs`).
    pub year: Option<YearToken>,
    pub mark: Option<ParsedMark>,
    /// The host's own metric conversion of a field mark, on the tables that publish a `Conv` column.
    pub conv_metres: Option<f64>,
    /// The conversion the host states in the mark cell's `title` (`Converted from 9:17.02 for Track
    /// Size.`): the published mark is the host's conversion of an earlier one, and the note says so.
    pub converted_note: Option<String>,
    pub meet: Option<ParsedMeet>,
    /// The published meet date. The list states no season of its own, so this is the only column a
    /// row's mark can be dated by.
    pub date: Option<PublishedDate>,
    pub wind: Option<f64>,
}

/// A competitor a row names: the numeric id and the two name channels the row carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAthlete {
    pub id: Option<u64>,
    /// The link's display text, `Last, First` flipped to `First Last`. A relay row prints surnames
    /// here, so a relay member's full name comes from [`ParsedAthlete::href_name`].
    pub name: String,
    /// The route slug's own name (`/athletes/8779512/Franklin_Central/Rylan_Hainje.html`), which is
    /// always `First_Last`.
    pub href_name: Option<String>,
}

impl ParsedAthlete {
    /// The full name this row states: the display text when the row printed one, the route slug
    /// otherwise. A relay member's display text is a surname, which is why `map.rs` absorbs only the
    /// individual rows: a surname alone is not an athlete identity.
    pub fn full_name(&self) -> Option<&str> {
        if self.name.is_empty() {
            self.href_name.as_deref()
        } else {
            Some(self.name.as_str())
        }
    }
}

/// The team a row attributes its athlete to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTeam {
    /// The link's display text (`Lawrence Central`).
    pub name: String,
    /// The team route (`/teams/tf/Lawrence_Central_m.html`), which is what a roster walk fetches.
    pub path: String,
    pub gender: Option<Gender>,
}

/// The meet a row names, with the route that carries its id when the row links one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMeet {
    pub name: String,
    pub id: Option<u64>,
    pub path: Option<String>,
}

/// Read one `performance-list-row` chunk.
///
/// A relay row publishes its members in a `col-athletes` cell and its mark for the team; an
/// individual row publishes one athlete and that athlete's own mark. Both carry the same cells for
/// place, team, meet and date, so those are read once.
pub(super) fn parse_row(body: &str) -> ParsedRow {
    let place = cell(body, "Place")
        .map(text_of)
        .and_then(|text| text.parse::<u16>().ok());
    let (athlete, relay_members) = match cell(body, "Athletes") {
        Some(members) => (None, relay_members(members)),
        None => (cell(body, "Athlete").and_then(single_athlete), Vec::new()),
    };
    let mark_cell = cell(body, "Time").or_else(|| cell(body, "Mark"));
    let mark = mark_cell.and_then(published_mark);
    ParsedRow {
        place,
        athlete,
        relay_members,
        team: cell(body, "Team").and_then(single_team),
        year: cell(body, "Year")
            .map(text_of)
            .and_then(|text| YearToken::parse(&text)),
        mark,
        conv_metres: cell(body, "Conv").and_then(converted_metres),
        converted_note: mark_cell.and_then(conversion_note),
        meet: cell(body, "Meet").and_then(single_meet),
        date: cell(body, "Meet Date").and_then(published_date),
        wind: cell(body, "Wind").map(text_of).and_then(wind_mps),
    }
}

/// The one athlete an individual row publishes.
fn single_athlete(cell: &str) -> Option<ParsedAthlete> {
    let (href, text) = links(cell).into_iter().next()?;
    Some(ParsedAthlete {
        id: athlete_id(href),
        name: flip_last_first(&text),
        href_name: href_name(href),
    })
}

/// The members a relay row publishes, in the order the row lists them.
///
/// The cell prints surnames; the full name is on each route slug, so a member without a slug name is
/// dropped here rather than minted from a surname.
fn relay_members(cell: &str) -> Vec<ParsedAthlete> {
    let mut members = Vec::new();
    for (href, text) in links(cell) {
        let Some(name) = href_name(href) else {
            continue;
        };
        members.push(ParsedAthlete {
            id: athlete_id(href),
            name: flip_last_first(&text),
            href_name: Some(name),
        });
    }
    members
}

/// The team a row names, with the gender side its route file stem states.
fn single_team(cell: &str) -> Option<ParsedTeam> {
    let (href, text) = links(cell).into_iter().next()?;
    let path = parse_team_path(href)?;
    Some(ParsedTeam {
        name: text,
        path: href.to_string(),
        gender: path.gender,
    })
}

/// The meet a row names. The meet route is optional on a row: a mark the host has no result page for
/// (a national meet it only tracks the mark at) prints the meet's name without a link.
fn single_meet(cell: &str) -> Option<ParsedMeet> {
    let name = text_of(cell);
    if name.is_empty() {
        return None;
    }
    let link = links(cell).into_iter().next();
    Some(ParsedMeet {
        name,
        id: link.as_ref().and_then(|(href, _)| meet_id(href)),
        path: link.map(|(href, _)| href.to_string()),
    })
}
