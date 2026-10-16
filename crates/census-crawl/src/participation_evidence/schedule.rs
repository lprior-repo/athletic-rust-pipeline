use super::{invalid, markup, RetainedHtmlCaptureV1};
use census_domain::model::{ProfileDate, SchoolYear};
use census_store::{StoreError, StoreResult};
use chrono::{Datelike, NaiveDate};

pub(super) struct Season {
    pub(super) school_year: SchoolYear,
    pub(super) competition: String,
    pub(super) venue: String,
    pub(super) scope: String,
    pub(super) locator: String,
}

pub(super) fn read(capture: &RetainedHtmlCaptureV1, day: &ProfileDate) -> StoreResult<Season> {
    let url =
        url::Url::parse(capture.requested_url()).map_err(|_| invalid("season URL invalid"))?;
    if url.host_str() != Some("dccshamrocks.net") || url.query().is_some() {
        return Err(invalid("unsupported in-season issuer"));
    }
    let edition = url
        .path()
        .strip_prefix("/sports/track-and-field/schedule/")
        .ok_or_else(|| invalid("unsupported school sport schedule path"))?;
    let year = school_year(edition, day)?;
    let html = capture.body()?;
    markup::canonical(&html, capture.requested_url())?;
    let heading = markup::single(
        markup::elements(&html, b"h2", None)?
            .into_iter()
            .filter(|item| item.html.contains("Track and Field Schedule"))
            .collect(),
    )?;
    if markup::text(heading.html)? != format!("{edition} Track and Field Schedule") {
        return Err(invalid("school schedule edition differs from source URL"));
    }
    let rows = markup::elements(&html, b"li", Some("sidearm-schedule-game"))?;
    let selected = rows
        .into_iter()
        .map(|row| {
            let date = markup::single(markup::elements(
                row.html,
                b"div",
                Some("sidearm-schedule-game-opponent-date"),
            )?)?;
            let span = markup::elements(date.html, b"span", None)?
                .into_iter()
                .next()
                .ok_or_else(|| invalid("schedule event day absent"))?;
            Ok((row, event_day(&markup::text(span.html)?, year)?))
        })
        .collect::<StoreResult<Vec<_>>>()?;
    let row = markup::single(
        selected
            .into_iter()
            .filter(|(_, date)| date == day)
            .map(|(row, _)| row)
            .collect(),
    )?;
    let game = row
        .attribute("data-game-id")
        .ok_or_else(|| invalid("schedule game identity absent"))?;
    if game.is_empty()
        || game.starts_with('0')
        || !game.bytes().all(|byte| byte.is_ascii_digit())
        || row.attribute("data-sport-id") != Some("15")
    {
        return Err(invalid("schedule game identity or sport differs"));
    }
    let competition = field(row.html, "sidearm-schedule-game-opponent-name")?;
    let venue = field(row.html, "sidearm-schedule-game-location")?;
    Ok(Season {
        school_year: year,
        competition,
        venue,
        scope: format!("{}#game-{game}", capture.requested_url()),
        locator: row.locator(),
    })
}

fn field(html: &str, class: &str) -> StoreResult<String> {
    let values = markup::elements(html, b"div", Some(class))?;
    let texts = values
        .iter()
        .map(|item| markup::text(item.html))
        .collect::<StoreResult<Vec<_>>>()?;
    let first = texts
        .first()
        .ok_or_else(|| invalid("schedule context absent"))?;
    if first.is_empty() || first.len() > 256 || texts.iter().any(|value| value != first) {
        return Err(invalid("schedule context is ambiguous"));
    }
    Ok(first.clone())
}

fn school_year(edition: &str, day: &ProfileDate) -> StoreResult<SchoolYear> {
    let year = SchoolYear::from_date(day.as_str())
        .ok_or_else(|| invalid("schedule day has no school year"))?;
    if edition != year.short() {
        return Err(invalid("schedule school year does not contain status day"));
    }
    Ok(year)
}

fn event_day(value: &str, year: SchoolYear) -> StoreResult<ProfileDate> {
    let (date, weekday) = value
        .split_once(" (")
        .ok_or_else(|| invalid("schedule event date unsupported"))?;
    let weekday = weekday
        .strip_suffix(')')
        .ok_or_else(|| invalid("schedule weekday unsupported"))?;
    let parsed = NaiveDate::parse_from_str(&format!("{} {date}", year.get()), "%Y %b %e")
        .map_err(|_| invalid("schedule event date invalid"))?;
    let actual_year = if parsed.month() >= 8 {
        year.get()
    } else {
        year.get()
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?
    };
    let parsed = parsed
        .with_year(i32::from(actual_year))
        .ok_or_else(|| invalid("schedule event date invalid"))?;
    if parsed.format("%a").to_string() != weekday {
        return Err(invalid("schedule weekday disagrees with edition date"));
    }
    ProfileDate::parse(&parsed.format("%Y-%m-%d").to_string())
        .map_err(|_| invalid("schedule event day unsupported"))
}
