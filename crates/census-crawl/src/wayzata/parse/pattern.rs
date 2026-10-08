use crate::{CrawlError, CrawlResult};
use regex::{Captures, Match, Regex};
use std::sync::LazyLock;

static ROW: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<tr\b[^>]*>.*?</tr>"));
static ROW_OPEN: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<tr\b"));
static ROW_CLASS: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)\A<tr\b[^>]*\sclass\s*=\s*(?:"([^"]*)"|'([^']*)')"#));
static TABLE_OPEN: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r#"(?is)<table\b[^>]*\sclass\s*=\s*(?:"([^"]*)"|'([^']*)')[^>]*>"#)
});
static TABLE_CLOSE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)</table\s*>"));
static DATE_CELL: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)<td\b[^>]*\sclass\s*=\s*(?:"[^"]*\bdate\b[^"]*"|'[^']*\bdate\b[^']*')[^>]*>(.*?)</td>"#,
    )
});
static NAME_CELL: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)<td\b[^>]*\sclass\s*=\s*(?:"[^"]*\bawayteam\b[^"]*"|'[^']*\bawayteam\b[^']*')[^>]*>(.*?)</td>"#,
    )
});
static VENUE_CELL: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)<td\b[^>]*\sclass\s*=\s*(?:"[^"]*\bhometeam\b[^"]*"|'[^']*\bhometeam\b[^']*')[^>]*>(.*?)</td>"#,
    )
});
static TITLE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<span\b[^>]*\stitle\s*=\s*(?:"([^"]*)"|'([^']*)')"#));
static LINK: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r#"(?is)<a\b[^>]*\shref\s*=\s*(?:"/links/([^"/?#]+)"|'/links/([^'/?#]+)')"#)
});
static ARIA: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<a\b[^>]*\saria-label\s*=\s*(?:"([^"]*)"|'([^']*)')"#));
static TAGS: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"(?is)<[^>]*>"));
static DAY: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"(\d{1,2})"));

fn get(
    pattern: &'static LazyLock<Result<Regex, regex::Error>>,
    name: &'static str,
) -> CrawlResult<&'static Regex> {
    pattern.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: name,
        source: source.clone(),
    })
}

pub(super) fn row() -> CrawlResult<&'static Regex> {
    get(&ROW, "ROW")
}
pub(super) fn row_open() -> CrawlResult<&'static Regex> {
    get(&ROW_OPEN, "ROW_OPEN")
}
pub(super) fn table_open() -> CrawlResult<&'static Regex> {
    get(&TABLE_OPEN, "TABLE_OPEN")
}
pub(super) fn table_close() -> CrawlResult<&'static Regex> {
    get(&TABLE_CLOSE, "TABLE_CLOSE")
}
pub(super) fn date_cell() -> CrawlResult<&'static Regex> {
    get(&DATE_CELL, "DATE_CELL")
}
pub(super) fn name_cell() -> CrawlResult<&'static Regex> {
    get(&NAME_CELL, "NAME_CELL")
}
pub(super) fn venue_cell() -> CrawlResult<&'static Regex> {
    get(&VENUE_CELL, "VENUE_CELL")
}
pub(super) fn title() -> CrawlResult<&'static Regex> {
    get(&TITLE, "TITLE")
}
pub(super) fn link() -> CrawlResult<&'static Regex> {
    get(&LINK, "LINK")
}
pub(super) fn aria() -> CrawlResult<&'static Regex> {
    get(&ARIA, "ARIA")
}
pub(super) fn tags() -> CrawlResult<&'static Regex> {
    get(&TAGS, "TAGS")
}
pub(super) fn day() -> CrawlResult<&'static Regex> {
    get(&DAY, "DAY")
}

pub(super) fn value<'a>(capture: &Captures<'a>) -> Option<Match<'a>> {
    capture.get(1).or_else(|| capture.get(2))
}

pub(super) fn classes(text: &str) -> CrawlResult<Option<&str>> {
    Ok(get(&ROW_CLASS, "ROW_CLASS")?
        .captures(text)
        .and_then(|capture| value(&capture))
        .map(|classes| classes.as_str()))
}
