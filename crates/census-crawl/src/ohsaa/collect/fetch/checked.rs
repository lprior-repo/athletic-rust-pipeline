use super::{PageKind, SearchResult};
use crate::net::FetchOutcome;
use crate::ohsaa::pages::{parse_ad_page, parse_coach_cell, parse_sport_label, published_absence};
use crate::ohsaa::parse::{decode_entities, strip_tags};
use census_domain::model::normalize_name;

mod markup;

pub(super) fn validate(
    sr: &SearchResult,
    kind: PageKind,
    capture: &FetchOutcome,
) -> Result<(), String> {
    validate_page(sr, kind, capture).map_err(str::to_string)
}

fn validate_page(
    sr: &SearchResult,
    kind: PageKind,
    capture: &FetchOutcome,
) -> Result<(), &'static str> {
    let html = std::str::from_utf8(&capture.body).map_err(|_| "page is not UTF-8")?;
    validate_owner(sr, html)?;
    if !html.contains("</body>") || !html.contains("</html>") {
        return Err("page document is unfinished");
    }
    match kind {
        PageKind::Sports => validate_sports(html),
        PageKind::AthleticDirector => validate_ad(html),
    }
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

fn validate_sports(html: &str) -> Result<(), &'static str> {
    let table = markup::table(html, "informationSportHeaderRow")?;
    markup::balanced(table)?;
    let target_rows = markup::rows(table).try_fold(0_usize, |count, row| {
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
            return Ok(count);
        }
        let [Some(label), Some(boys), Some(girls)] = markup::cells(row, "td")? else {
            return Err("sports row does not contain its three complete cells");
        };
        if parse_sport_label(&decode_entities(&strip_tags(label))).is_none() {
            return Ok(count);
        }
        validate_coach_cell(boys)?;
        validate_coach_cell(girls)?;
        Ok(count.saturating_add(1))
    })?;
    if target_rows == 0 {
        return Err("sports table publishes no supported sport rows or structured absence");
    }
    Ok(())
}

fn validate_coach_cell(cell: &str) -> Result<(), &'static str> {
    let value = decode_entities(&strip_tags(cell));
    if !cell.contains("class=\"fieldValue\"") || value.is_empty() {
        return Err("coach field is absent or empty rather than a published value");
    }
    if parse_coach_cell(cell).is_some() || published_absence(&value) {
        return Ok(());
    }
    Err("coach field is not a supported published value")
}

#[derive(Clone, Copy)]
enum DirectorState {
    Missing,
    AwaitingValue,
    Published,
}

fn validate_ad(html: &str) -> Result<(), &'static str> {
    let table = markup::table(html, "athleticDepartmentSubheader")?;
    markup::balanced(table)?;
    let state = markup::rows(table).try_fold(DirectorState::Missing, |state, row| {
        director_row(state, row?)
    })?;
    match state {
        DirectorState::Published if parse_ad_page(html).director.is_some() => Ok(()),
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
    validate_coach_cell(name)?;
    Ok(DirectorState::Published)
}
