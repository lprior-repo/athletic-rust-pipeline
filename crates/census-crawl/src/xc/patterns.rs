//! The literal text patterns of the three cross-country layouts, and the accessors that keep a
//! failed compile a typed error instead of a panic.
use crate::{CrawlError, CrawlResult};
use regex::Regex;
use std::sync::LazyLock;

static PAGE_STAMP: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^\d{1,2}/\d{1,2}/\d{2,4},\s*\d{1,2}:\d{2}\s*(?:AM|PM)\s*"));
static DATE_NAMED: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)\b([A-Z][a-z]{2,8})\s+(\d{1,2}),\s*(\d{4})\b"));
static DATE_SLASH: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"\b(\d{1,2})/(\d{1,2})/(\d{4})\b"));
static SECTION_BANNER: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^(?:=+|\*+)\s*([A-Za-z0-9].*?)\s*(?:=+|\*+)$"));
static GENDER_HEADING: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)^(boys|girls|men|women)['\u{2019}]?(?:\s+(.+?))?\s*$"));
static DIVISION: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^Division\s+([0-9A-Za-z]+)\s*$"));
/// Team block heading: place, team points, team name, then the scoring summary in brackets.
static TEAM_BLOCK: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^\s*\d+\.\s+(\d+)\s+(\S.*?)\s*\(\s*\d"));
/// Block row: team position, overall place, name, grade, time — repeated per line, because the race
/// blocks print two runners side by side and the team tables that follow print one.
static BLOCK_ROW: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"(\d{1,4})\s+(\(\s*\d+\s*\)|\d{1,4})\s+([A-Za-z][A-Za-z.'\- ]*?)\s+(\d{1,2})\s+(\d{1,3}:\d{2}\.\d)",
    )
});
/// Padded grade table row: place, points, bib, name, school, gender, grade, time, pace.
static GRADE_TABLE_ROW: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"^\s*(\d+)\s+(\(\s*n/a\s*\)|\d+)\s+(\d+)\s+(.+?)\s{2,}(.+?)\s{2,}([MF])\s+(\d{1,2})\s+(\d{1,3}:\d{2}\.\d)\s+(\d+:\d{2})\s*$",
    )
});
/// Any heading that names a gender: `Boys Varsity`, `BOYS TEAM SCORE`, `Girls' 5000 Meter Run`.
static RACE_BANNER: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)^(boys|girls|men|women)['\u{2019}]?\s*(.*)$"));

pub(super) fn page_stamp() -> CrawlResult<&'static Regex> {
    PAGE_STAMP.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "PAGE_STAMP",
        source: source.clone(),
    })
}

pub(super) fn date_named() -> CrawlResult<&'static Regex> {
    DATE_NAMED.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "DATE_NAMED",
        source: source.clone(),
    })
}

pub(super) fn date_slash() -> CrawlResult<&'static Regex> {
    DATE_SLASH.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "DATE_SLASH",
        source: source.clone(),
    })
}

pub(super) fn section_banner() -> CrawlResult<&'static Regex> {
    SECTION_BANNER
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "SECTION_BANNER",
            source: source.clone(),
        })
}

pub(super) fn gender_heading() -> CrawlResult<&'static Regex> {
    GENDER_HEADING
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "GENDER_HEADING",
            source: source.clone(),
        })
}

pub(super) fn division_regex() -> CrawlResult<&'static Regex> {
    DIVISION.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "DIVISION",
        source: source.clone(),
    })
}

pub(super) fn team_block() -> CrawlResult<&'static Regex> {
    TEAM_BLOCK.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "TEAM_BLOCK",
        source: source.clone(),
    })
}

pub(super) fn block_row() -> CrawlResult<&'static Regex> {
    BLOCK_ROW.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "BLOCK_ROW",
        source: source.clone(),
    })
}

pub(super) fn grade_table_row_regex() -> CrawlResult<&'static Regex> {
    GRADE_TABLE_ROW
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "GRADE_TABLE_ROW",
            source: source.clone(),
        })
}

pub(super) fn race_banner() -> CrawlResult<&'static Regex> {
    RACE_BANNER
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "RACE_BANNER",
            source: source.clone(),
        })
}
