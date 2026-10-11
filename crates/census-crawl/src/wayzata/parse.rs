use crate::{CrawlError, CrawlResult};
use census_domain::model::Sport;
use regex::Regex;

use super::BASE;

mod pattern;
use pattern::{
    aria, classes, date_cell, day, link, name_cell, row, tags, title, value, venue_cell,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleSport {
    Track,
    CrossCountry,
}

impl ScheduleSport {
    fn path(self) -> &'static str {
        match self {
            ScheduleSport::Track => "track",
            ScheduleSport::CrossCountry => "xc",
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            ScheduleSport::Track => "track",
            ScheduleSport::CrossCountry => "xc",
        }
    }

    pub(super) fn sport_for(self, month: u8) -> Sport {
        match self {
            ScheduleSport::CrossCountry => Sport::CrossCountry,
            ScheduleSport::Track if month <= 3 || month >= 11 => Sport::IndoorTrack,
            ScheduleSport::Track => Sport::OutdoorTrack,
        }
    }
}

pub fn schedule_url(sport: ScheduleSport, year: i16) -> String {
    format!("{BASE}/sports/{}/{year}/schedule", sport.path())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetRow {
    pub date: String,
    pub name: String,
    pub location: String,
    pub slug: Option<String>,
    pub aria_label: Option<String>,
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

pub fn schedule_rows(body: &str, year: i16) -> CrawlResult<Vec<MeetRow>> {
    super::budget::check(
        "Wayzata schedule bytes",
        body.len(),
        super::budget::MAX_PAGE_BYTES,
    )?;
    let body = schedule_body(body)?.map_or(body, core::convert::identity);
    rows(body, year)
}

pub(super) fn rows(body: &str, year: i16) -> CrawlResult<Vec<MeetRow>> {
    let pattern = row()?;
    let count = row_count(pattern, body)?;
    let mut rows = Vec::new();
    super::budget::reserve(&mut rows, count, super::budget::MAX_PAGE_ROWS)?;
    pattern
        .find_iter(body)
        .try_fold((None, rows), |(mut month, mut rows), found| {
            let text = found.as_str();
            let classes = classes(text)?;
            if classes
                .is_some_and(|classes| classes.split_whitespace().any(|class| class == "event-row"))
            {
                rows.push(parse_row(text, year, month)?);
            } else if classes.is_some_and(|classes| {
                classes
                    .split_whitespace()
                    .any(|class| class == "month-title")
            }) {
                super::budget::check(
                    "Wayzata month heading bytes",
                    text.len(),
                    super::budget::MAX_ROW_BYTES,
                )?;
                month = month_from_text(&tags()?.replace_all(text, " "));
            }
            Ok((month, rows))
        })
        .map(|(_, rows)| rows)
}

fn row_count(pattern: &Regex, body: &str) -> CrawlResult<usize> {
    let closed = pattern.find_iter(body).count();
    let starts = pattern::row_open()?.find_iter(body).count();
    if starts != closed {
        return Err(CrawlError::Schema {
            url: BASE.to_string(),
            detail: "schedule contains an incomplete event row".to_string(),
        });
    }
    let count = pattern.find_iter(body).try_fold(0usize, |count, found| {
        if classes(found.as_str())?
            .is_some_and(|classes| classes.split_whitespace().any(|class| class == "event-row"))
        {
            count
                .checked_add(1)
                .ok_or_else(|| super::budget::arithmetic("Wayzata row count"))
        } else {
            Ok(count)
        }
    })?;
    super::budget::check("Wayzata schedule rows", count, super::budget::MAX_PAGE_ROWS)?;
    Ok(count)
}

fn parse_row(text: &str, year: i16, month: Option<u8>) -> CrawlResult<MeetRow> {
    super::budget::check(
        "Wayzata row bytes",
        text.len(),
        super::budget::MAX_ROW_BYTES,
    )?;
    Ok(MeetRow {
        date: published_date(text, year, month)?,
        name: cell_text(name_cell()?, text)?,
        location: cell_text(venue_cell()?, text)?,
        slug: link()?
            .captures(text)
            .and_then(|capture| value(&capture))
            .map(|slug| bounded_text(slug.as_str()))
            .transpose()?
            .filter(|slug| !slug.is_empty()),
        aria_label: aria()?
            .captures(text)
            .and_then(|capture| value(&capture))
            .map(|label| normalize_whitespace(label.as_str()))
            .transpose()?
            .filter(|label| !label.is_empty()),
    })
}

fn published_date(text: &str, year: i16, month: Option<u8>) -> CrawlResult<String> {
    let raw = cell_text(date_cell()?, text)?;
    if raw.len() == 10 && chrono::NaiveDate::parse_from_str(&raw, "%Y-%m-%d").is_ok() {
        return Ok(raw);
    }
    if day()?.find_iter(&raw).count() != 1 {
        return Ok(raw);
    }
    let day = day()?
        .captures(&raw)
        .and_then(|capture| capture.get(1))
        .and_then(|day| day.as_str().parse::<u8>().ok());
    match (month, day) {
        (Some(month), Some(day)) => Ok(format!("{year:04}-{month:02}-{day:02}")),
        _ => Ok(raw),
    }
}

fn cell_text(cell: &Regex, row: &str) -> CrawlResult<String> {
    let Some(captures) = cell.captures(row) else {
        return Ok(String::new());
    };
    let inner = captures
        .get(1)
        .map(|cell| cell.as_str())
        .map_or(Default::default(), core::convert::identity);
    if let Some(title) = title()?.captures(inner).and_then(|capture| value(&capture)) {
        return normalize_whitespace(title.as_str());
    }
    normalize_whitespace(&tags()?.replace_all(inner, " "))
}

fn month_from_text(text: &str) -> Option<u8> {
    let heading = text.split_whitespace().next()?;
    MONTHS
        .iter()
        .position(|month| heading.eq_ignore_ascii_case(month))
        .and_then(|index| index.checked_add(1))
        .and_then(|month| u8::try_from(month).ok())
}

fn normalize_whitespace(text: &str) -> CrawlResult<String> {
    super::budget::check(
        "Wayzata field bytes",
        text.len(),
        super::budget::MAX_FIELD_BYTES,
    )?;
    let mut result = String::new();
    result.try_reserve(text.len()).map_err(|_| {
        super::budget::resource(
            "Wayzata field allocation",
            text.len(),
            super::budget::MAX_FIELD_BYTES,
        )
    })?;
    text.split_whitespace().for_each(|word| {
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(word);
    });
    Ok(result)
}

fn bounded_text(text: &str) -> CrawlResult<String> {
    super::budget::check(
        "Wayzata field bytes",
        text.len(),
        super::budget::MAX_FIELD_BYTES,
    )?;
    let mut value = String::new();
    value.try_reserve(text.len()).map_err(|_| {
        super::budget::resource(
            "Wayzata field allocation",
            text.len(),
            super::budget::MAX_FIELD_BYTES,
        )
    })?;
    value.push_str(text);
    Ok(value)
}

pub(super) fn schedule_body(body: &str) -> CrawlResult<Option<&str>> {
    let table = pattern::table_open()?
        .captures_iter(body)
        .find(|capture| {
            value(capture).is_some_and(|classes| {
                classes
                    .as_str()
                    .split_whitespace()
                    .any(|class| class == "schedule")
            })
        })
        .and_then(|capture| capture.get(0));
    let Some(table) = table else {
        return Ok(None);
    };
    let tail = body
        .get(table.end()..)
        .ok_or_else(|| super::budget::arithmetic("Wayzata table start"))?;
    let closing = pattern::table_close()?
        .find(tail)
        .ok_or_else(|| CrawlError::Schema {
            url: BASE.to_string(),
            detail: "schedule table is truncated".to_string(),
        })?;
    let end = table
        .end()
        .checked_add(closing.end())
        .ok_or_else(|| super::budget::arithmetic("Wayzata table end"))?;
    body.get(table.start()..end)
        .map(Some)
        .ok_or_else(|| super::budget::arithmetic("Wayzata table slice"))
}
