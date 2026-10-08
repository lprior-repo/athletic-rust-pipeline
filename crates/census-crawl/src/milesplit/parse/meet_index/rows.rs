use super::super::super::wire::MeetRef;
use super::super::html_unescape;
use crate::{CrawlError, CrawlResult};
use regex::Regex;

const MAX_ROWS: usize = 512;
const MAX_BYTES: usize = 2 * 1024 * 1024;

struct Rules {
    id: &'static Regex,
    link: &'static Regex,
    day: &'static Regex,
    venue: &'static Regex,
}

impl Rules {
    fn new() -> CrawlResult<Self> {
        Ok(Self {
            id: super::meet_row_id_regex()?,
            link: super::meet_row_link_regex()?,
            day: super::meet_row_day_regex()?,
            venue: super::meet_row_venue_regex()?,
        })
    }

    fn project(&self, row: &str, month: Option<&str>) -> CrawlResult<MeetRef> {
        let id = self
            .id
            .captures(row)
            .and_then(|captures| captures.get(1))
            .ok_or_else(|| malformed("meet row lacks its provider id"))?;
        let link = self
            .link
            .captures(row)
            .ok_or_else(|| malformed("meet row lacks its owned link"))?;
        let url = link
            .get(1)
            .ok_or_else(|| malformed("meet row lacks its result locator"))?;
        let name = link
            .get(2)
            .ok_or_else(|| malformed("meet row lacks its published name"))?;
        if name.as_str().trim().is_empty() || url.as_str().trim().is_empty() {
            return Err(malformed("meet row has an empty name or result locator"));
        }
        let day = self.day.captures(row).and_then(|captures| captures.get(1));
        let venue = self
            .venue
            .captures(row)
            .and_then(|captures| captures.get(1));
        Ok(MeetRef {
            meet_id: own(id.as_str())?,
            name: html_unescape(name.as_str().trim()),
            date: month
                .zip(day)
                .and_then(|(month, day)| super::iso_date(month, day.as_str())),
            venue: venue.map_or_else(String::new, |venue| html_unescape(venue.as_str().trim())),
            results_url: own(url.as_str())?,
        })
    }
}

pub(super) fn parse(html: &str) -> CrawlResult<Vec<MeetRef>> {
    let expected = admission(html)?;
    let rules = Rules::new()?;
    let marker = super::meet_index_marker_regex()?;
    let mut rows = Vec::new();
    rows.try_reserve_exact(expected)
        .map_err(|_| capacity("meet index rows", expected, MAX_ROWS))?;
    let (rows, _) = marker
        .captures_iter(html)
        .try_fold((rows, None), |mut state, capture| {
            if let Some(month) = capture.get(1) {
                state.1 = Some(month.as_str());
                return Ok(state);
            }
            let row = capture
                .get(2)
                .ok_or_else(|| malformed("meet index marker is incomplete"))?;
            state.0.push(rules.project(row.as_str(), state.1)?);
            Ok::<_, CrawlError>(state)
        })?;
    if rows.len() != expected {
        return Err(malformed(
            "meet index contains an unterminated or unrecognized meet row",
        ));
    }
    Ok(rows)
}

fn admission(html: &str) -> CrawlResult<usize> {
    if html.len() > MAX_BYTES {
        return Err(capacity("meet index capture bytes", html.len(), MAX_BYTES));
    }
    let rows = html
        .matches(r#"<li class="meet-row""#)
        .take(MAX_ROWS + 1)
        .count();
    if rows > MAX_ROWS {
        return Err(capacity("meet index rows", rows, MAX_ROWS));
    }
    Ok(rows)
}

fn own(value: &str) -> CrawlResult<String> {
    let mut owned = String::new();
    owned
        .try_reserve_exact(value.len())
        .map_err(|_| capacity("meet index field bytes", value.len(), MAX_BYTES))?;
    owned.push_str(value);
    Ok(owned)
}

fn malformed(detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: "meet index".to_owned(),
        detail: detail.to_owned(),
    }
}

fn capacity(resource: &'static str, requested: usize, limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource,
        requested,
        limit,
    }
}
