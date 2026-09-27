mod columns;
mod identity;
mod map;
mod parse;

pub use crate::result_file::{ParsedEvent, ParsedMeet, ParsedRow, RelayLeg};
use crate::{CrawlError, CrawlResult};
use census_domain::model::SourceRef;
use regex::Regex;
use std::sync::LazyLock;

pub(crate) use columns::{
    columns_from_header, grade_from_token, looks_like_a_name, substring, tokens, Column, Token,
};
pub(crate) use map::NO_MARK;
use map::{event_header, header_meet};
pub use map::{hytek_event_kind, parse_field_mark, parse_time, round_marker};
use parse::{parse_legs, parse_row, place_prefix_regex, starts_like_a_row, Section};

static PRE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<pre[^>]*>(.*?)(?:</pre>|\z)"));
static PARA: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<p[^>]*>(.*?)</p>"));
static BREAK: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<br\s*/?>"));
static TAG: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"(?is)<[^>]*>"));

fn pre_regex() -> CrawlResult<&'static Regex> {
    PRE.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "PRE",
        source: source.clone(),
    })
}

fn para_regex() -> CrawlResult<&'static Regex> {
    PARA.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "PARA",
        source: source.clone(),
    })
}

fn break_regex() -> CrawlResult<&'static Regex> {
    BREAK.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "BREAK",
        source: source.clone(),
    })
}

fn tag_regex() -> CrawlResult<&'static Regex> {
    TAG.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "TAG",
        source: source.clone(),
    })
}

pub fn parse(lines: &[String], source: SourceRef) -> Option<ParsedMeet> {
    let (name, date, end_date) = header_meet(lines)?;
    let timer = lines.iter().find_map(|line| {
        line.strip_prefix("Licensed to ")
            .map(|rest| rest.split(" - ").next().unwrap_or(rest).trim().to_string())
    });

    let mut events: Vec<ParsedEvent> = Vec::new();
    let mut section: Option<Section> = None;
    let mut row_lines = 0usize;
    let mut skipped_rows = 0usize;
    let place_prefix = place_prefix_regex().ok();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if apply_marker(line, trimmed, &mut events, &mut section) {
            continue;
        }
        if extend_relay_legs(trimmed, &mut events, place_prefix) {
            continue;
        }
        let Some(event) = events.last_mut() else {
            continue;
        };
        let Some(section) = section.as_ref() else {
            continue;
        };
        let Some(row) = parse_row(line, &event.kind, section) else {
            if starts_like_a_row(trimmed) {
                skipped_rows = skipped_rows.saturating_add(1);
            }
            continue;
        };
        row_lines = row_lines.saturating_add(1);
        event.rows.push(row);
    }

    let _ = source;
    Some(ParsedMeet {
        name,
        date,
        end_date,
        timer,
        events,
        rows_parsed: row_lines,
        rows_skipped: skipped_rows,
    })
}

fn apply_marker(
    line: &str,
    trimmed: &str,
    events: &mut Vec<ParsedEvent>,
    section: &mut Option<Section>,
) -> bool {
    if let Some(parsed) = Section::from_header(line) {
        if let Some(event) = events.last_mut() {
            event.round = parsed.round.map(str::to_string);
        }
        *section = Some(parsed);
        return true;
    }
    if let Some(event) = event_header(trimmed) {
        events.push(event);
        return true;
    }
    if let Some(round) = round_marker(trimmed) {
        if let Some(event) = events.last_mut() {
            event.round = Some(round.to_string());
        }
        return true;
    }
    false
}

fn extend_relay_legs(
    trimmed: &str,
    events: &mut [ParsedEvent],
    place_prefix: Option<&Regex>,
) -> bool {
    if !place_prefix.is_some_and(|pattern| pattern.is_match(trimmed)) {
        return false;
    }
    if let Some(event) = events.last_mut() {
        if event.kind.is_relay() {
            if let Some(row) = event.rows.last_mut() {
                let legs = parse_legs(trimmed, row.legs.len());
                if !legs.is_empty() {
                    row.legs.extend(legs);
                }
            }
        }
    }
    true
}

pub fn lines_from_html(body: &str) -> Vec<String> {
    let pre = pre_regex().ok();
    let breaks = break_regex().ok();
    if let Some(capture) = pre.and_then(|pattern| pattern.captures(body)) {
        let inner = capture.get(1).map(|m| m.as_str()).unwrap_or_default();
        let spaced = match breaks {
            Some(pattern) => pattern.replace_all(inner, "\n"),
            None => std::borrow::Cow::Borrowed(inner),
        };
        return strip_tags(&spaced).lines().map(clean_line).collect();
    }
    let Ok(paragraph) = para_regex() else {
        return Vec::new();
    };
    paragraph
        .captures_iter(body)
        .map(|capture| {
            let inner = capture.get(1).map(|m| m.as_str()).unwrap_or_default();
            clean_line(&strip_tags(inner))
        })
        .collect()
}

pub fn lines_from_text(body: &str) -> Vec<String> {
    body.lines().map(clean_line).collect()
}

pub fn lines_from_pdf_text(text: &str) -> Vec<String> {
    text.replace('\u{c}', "\n")
        .lines()
        .map(clean_line)
        .collect()
}

fn strip_tags(inner: &str) -> String {
    let Ok(tag) = tag_regex() else {
        return inner.to_string();
    };
    tag.replace_all(inner, "").into_owned()
}

fn clean_line(raw: &str) -> String {
    let unescaped = raw
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace('\u{a0}', " ");
    unescaped.trim_end().to_string()
}

#[cfg(test)]
mod tests;
