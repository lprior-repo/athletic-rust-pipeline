
use super::date::{published_date, PublishedDate};
use super::html::{cell, links, text_of};
use super::mark::{conversion_note, converted_metres, published_mark, wind_mps, ParsedMark};
use super::route::{athlete_id, href_name, meet_id, parse_team_path};
use super::season::YearToken;
use census_domain::model::{flip_last_first, Gender};

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedRow {
    pub place: Option<u16>,
    pub athlete: Option<ParsedAthlete>,
    pub relay_members: Vec<ParsedAthlete>,
    pub team: Option<ParsedTeam>,
    pub year: Option<YearToken>,
    pub mark: Option<ParsedMark>,
    pub conv_metres: Option<f64>,
    pub converted_note: Option<String>,
    pub meet: Option<ParsedMeet>,
    pub date: Option<PublishedDate>,
    pub wind: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAthlete {
    pub id: Option<u64>,
    pub name: String,
    pub href_name: Option<String>,
}

impl ParsedAthlete {
    pub fn full_name(&self) -> Option<&str> {
        if self.name.is_empty() {
            self.href_name.as_deref()
        } else {
            Some(self.name.as_str())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTeam {
    pub name: String,
    pub path: String,
    pub gender: Option<Gender>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedMeet {
    pub name: String,
    pub id: Option<u64>,
    pub path: Option<String>,
}

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

fn single_athlete(cell: &str) -> Option<ParsedAthlete> {
    let (href, text) = links(cell).into_iter().next()?;
    Some(ParsedAthlete {
        id: athlete_id(href),
        name: flip_last_first(&text),
        href_name: href_name(href),
    })
}

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

fn single_team(cell: &str) -> Option<ParsedTeam> {
    let (href, text) = links(cell).into_iter().next()?;
    let path = parse_team_path(href)?;
    Some(ParsedTeam {
        name: text,
        path: href.to_string(),
        gender: path.gender,
    })
}

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
