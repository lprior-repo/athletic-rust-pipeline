use super::map::{AdPage, CoachEntry, SearchResult};
use crate::net::FetchOutcome;
use crate::ohsaa::pages::{parse_ad_page, parse_coach_cell, parse_sport_label, published_absence};
use crate::ohsaa::parse::{decode_entities, strip_tags};
use census_domain::model::normalize_name;

mod markup;

#[derive(Debug)]
pub struct SportsPage {
    owner: SearchResult,
    capture: FetchOutcome,
    rows: Vec<SportRow>,
}

#[derive(Debug)]
pub struct DirectorPage {
    owner: SearchResult,
    capture: FetchOutcome,
    page: AdPage,
}

pub type SportRow = (String, Option<CoachEntry>, Option<CoachEntry>);

#[derive(Debug)]
pub struct PageError {
    pub capture: Box<FetchOutcome>,
    pub detail: String,
}

impl std::fmt::Display for PageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for PageError {}

impl SportsPage {
    pub(super) fn owner(&self) -> &SearchResult { &self.owner }
    pub(super) fn capture(&self) -> &FetchOutcome { &self.capture }
    pub(super) fn rows(&self) -> &[SportRow] { &self.rows }

    pub fn parse(owner: &SearchResult, capture: FetchOutcome) -> Result<Self, PageError> {
        let rows = document(owner, &capture).and_then(parse_sports_table);
        match rows {
            Ok(rows) => Ok(Self { owner: owner.clone(), capture, rows }),
            Err(detail) => Err(PageError { capture: Box::new(capture), detail: detail.to_string() }),
        }
    }
}

impl DirectorPage {
    pub(super) fn owner(&self) -> &SearchResult { &self.owner }
    pub(super) fn capture(&self) -> &FetchOutcome { &self.capture }
    pub(super) fn page(&self) -> &AdPage { &self.page }

    pub fn parse(owner: &SearchResult, capture: FetchOutcome) -> Result<Self, PageError> {
        let page = document(owner, &capture).and_then(parse_director);
        match page {
            Ok(page) => Ok(Self { owner: owner.clone(), capture, page }),
            Err(detail) => Err(PageError { capture: Box::new(capture), detail: detail.to_string() }),
        }
    }
}

fn document<'a>(owner: &SearchResult, capture: &'a FetchOutcome) -> Result<&'a str, &'static str> {
    let html = std::str::from_utf8(&capture.body).map_err(|_| "page is not UTF-8")?;
    validate_owner(owner, html)?;
    if !html.contains("</body>") || !html.contains("</html>") {
        return Err("page document is unfinished");
    }
    Ok(html)
}

fn validate_owner(sr: &SearchResult, html: &str) -> Result<(), &'static str> {
    let header = html
        .split("<div")
        .skip(1)
        .find_map(|candidate| {
            let (attributes, body) = candidate.split_once('>')?;
            if !attributes.contains("schoolHeader") {
                return None;
            }
            body.split_once("</div>").map(|(header, _)| header)
        })
        .ok_or("complete school header is absent")?;
    let (_, heading) = header.split_once("<h2").ok_or("school heading is absent")?;
    let (_, heading) = heading
        .split_once('>')
        .ok_or("school heading is unfinished")?;
    let (heading, _) = heading
        .split_once("</h2>")
        .ok_or("school heading is unfinished")?;
    let published = decode_entities(&strip_tags(heading));
    let (name, id) = published
        .rsplit_once('(')
        .ok_or("school heading has no owner ID")?;
    let id = id
        .strip_suffix(')')
        .ok_or("school heading owner ID is malformed")?;
    if id != sr.ohsaa_id || normalize_name(name) != normalize_name(&sr.name) {
        return Err("page belongs to a different school or a publisher error owner");
    }
    Ok(())
}

pub fn parse_sports_table(html: &str) -> Result<Vec<SportRow>, &'static str> {
    let table = markup::table(html, "informationSportHeaderRow")?;
    markup::balanced(table)?;
    let rows = markup::rows(table).try_fold(Vec::new(), |mut rows, row| {
        let row = row?;
        if row.contains("informationSportHeaderRow") {
            let headers = markup::cells(row, "th")?;
            let valid = headers
                .iter()
                .zip(["Sport", "Head Boys Coach", "Head Girls Coach"])
                .all(|(cell, expected)| cell.is_some_and(|cell| strip_tags(cell) == expected));
            if !valid {
                return Err("sports table has unsupported columns");
            }
            return Ok(rows);
        }
        let [Some(label), Some(boys), Some(girls)] = markup::cells(row, "td")? else {
            return Err("sports row does not contain its three complete cells");
        };
        let label = decode_entities(&strip_tags(label));
        if parse_sport_label(&label).is_none() {
            return Ok(rows);
        }
        let boys = checked_coach_cell(boys)?;
        let girls = checked_coach_cell(girls)?;
        rows.try_reserve(1).map_err(|_| "sports row allocation failed")?;
        rows.push((label, boys, girls));
        Ok(rows)
    })?;
    if rows.is_empty() {
        return Err("sports table publishes no supported sport rows or structured absence");
    }
    Ok(rows)
}

fn checked_coach_cell(cell: &str) -> Result<Option<CoachEntry>, &'static str> {
    let value = decode_entities(&strip_tags(cell));
    if !cell.contains("class=\"fieldValue\"") || value.is_empty() {
        return Err("coach field is absent or empty rather than a published value");
    }
    let entry = parse_coach_cell(cell);
    if entry.is_some() || published_absence(&value) {
        return Ok(entry);
    }
    Err("coach field is not a supported published value")
}

#[derive(Clone, Copy)]
enum DirectorState {
    Missing,
    AwaitingValue,
    Published,
}

fn parse_director(html: &str) -> Result<AdPage, &'static str> {
    let table = markup::table(html, "athleticDepartmentSubheader")?;
    markup::balanced(table)?;
    let state = markup::rows(table).try_fold(DirectorState::Missing, |state, row| {
        director_row(state, row?)
    })?;
    let page = parse_ad_page(table);
    match state {
        DirectorState::Published if page.director.is_some() => Ok(page),
        DirectorState::Published => Err("AD value does not match the supported published field"),
        DirectorState::Missing => Err("AD table has no Athletic Director field"),
        DirectorState::AwaitingValue => Err("AD field has no complete published value"),
    }
}

fn director_row(state: DirectorState, row: &str) -> Result<DirectorState, &'static str> {
    let cells = markup::cells(row, "td")?;
    let label = cells
        .first()
        .and_then(|cell| *cell)
        .map(|cell| decode_entities(&strip_tags(cell)));
    if label.as_deref() == Some("Athletic Director:") && row.contains("athleticDepartmentSubheader")
    {
        return match state {
            DirectorState::Missing => Ok(DirectorState::AwaitingValue),
            _ => Err("AD table contains repeated or incomplete director fields"),
        };
    }
    if !matches!(state, DirectorState::AwaitingValue) {
        return Ok(state);
    }
    let [Some(name), Some(_email), None] = cells else {
        return Err("AD value row does not contain its two complete cells");
    };
    checked_coach_cell(name)?;
    Ok(DirectorState::Published)
}
