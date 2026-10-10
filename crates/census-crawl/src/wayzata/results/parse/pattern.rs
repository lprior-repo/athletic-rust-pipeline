use crate::{CrawlError, CrawlResult};
use regex::{Captures, Match, Regex};
use std::sync::LazyLock;

static ROW: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<tr\b[^>]*>.*?</tr>"));
static EVENT_CELL: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)<td\b[^>]*\sclass\s*=\s*(?:"[^"]*\bevent[^"]*"|'[^']*\bevent[^']*')[^>]*>(.*?)</td>"#,
    )
});
static ATHLETE_CELL: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)<td\b[^>]*\sclass\s*=\s*(?:"[^"]*\bathlete[^"]*"|'[^']*\bathlete[^']*')[^>]*>(.*?)</td>"#,
    )
});
static TEAM_CELL: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)<td\b[^>]*\sclass\s*=\s*(?:"[^"]*\bteam[^"]*"|'[^']*\bteam[^']*')[^>]*>(.*?)</td>"#,
    )
});
static MARK_CELL: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"(?is)<td\b[^>]*\sclass\s*=\s*(?:"[^"]*\bmark[^"]*"|'[^']*\bmark[^']*')[^>]*>(.*?)</td>"#,
    )
});
static RESULTS_TABLE_OPEN: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r#"(?is)<table\b[^>]*\sclass\s*=\s*(?:"[^"]*\bresults[^"]*"|'[^']*\bresults[^']*')[^>]*>"#)
});
static TABLE_CLOSE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)</table\s*>"));

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
pub(super) fn event_cell() -> CrawlResult<&'static Regex> {
    get(&EVENT_CELL, "EVENT_CELL")
}
pub(super) fn athlete_cell() -> CrawlResult<&'static Regex> {
    get(&ATHLETE_CELL, "ATHLETE_CELL")
}
pub(super) fn team_cell() -> CrawlResult<&'static Regex> {
    get(&TEAM_CELL, "TEAM_CELL")
}
pub(super) fn mark_cell() -> CrawlResult<&'static Regex> {
    get(&MARK_CELL, "MARK_CELL")
}
pub(super) fn results_table_open() -> CrawlResult<&'static Regex> {
    get(&RESULTS_TABLE_OPEN, "RESULTS_TABLE_OPEN")
}
pub(super) fn table_close() -> CrawlResult<&'static Regex> {
    get(&TABLE_CLOSE, "TABLE_CLOSE")
}

pub(super) fn value<'a>(capture: &Captures<'a>) -> Option<Match<'a>> {
    capture.get(1)
}