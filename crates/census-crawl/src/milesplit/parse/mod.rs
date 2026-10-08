use crate::{CrawlError, CrawlResult};
use regex::Regex;
use std::sync::LazyLock;

use super::wire::TeamRef;

static TEAM_ROW_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<tr>\s*<td>\s*<a href="(https?://[a-z]{2}\.milesplit\.com/teams/(\d+)-([^"]+))">\s*([^<]+?)\s*</a>\s*</td>\s*<td>\s*([^<]*?)\s*</td>"#,
    )
});

static ATHLETE_ROW_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?s)<li class="athlete-row data-row">(.*?)</li>"#));

static ATHLETE_LINK_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"<a href="(https?://(?:[a-z0-9-]+\.)*milesplit\.com/athletes/(\d+)-[^"]*)">([^<]*)</a>"#,
    )
});

static GENDER_CELL_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"column-gender[^>]*>([^<]*)<"#));

static GRAD_CELL_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"column-grad-year[^>]*>([^<]*)<"#));

static SEASON_CELL_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<div class="data-point[^"]*"[^>]*data-season-id="(\d+)"[^>]*>\s*<svg[^>]*class="icon icon-(yes|no)""#,
    )
});

fn team_row_regex() -> CrawlResult<&'static Regex> {
    TEAM_ROW_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "TEAM_ROW_REGEX",
            source: source.clone(),
        })
}

fn athlete_row_regex() -> CrawlResult<&'static Regex> {
    ATHLETE_ROW_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "ATHLETE_ROW_REGEX",
            source: source.clone(),
        })
}

fn athlete_link_regex() -> CrawlResult<&'static Regex> {
    ATHLETE_LINK_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "ATHLETE_LINK_REGEX",
            source: source.clone(),
        })
}

fn gender_cell_regex() -> CrawlResult<&'static Regex> {
    GENDER_CELL_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "GENDER_CELL_REGEX",
            source: source.clone(),
        })
}

fn grad_cell_regex() -> CrawlResult<&'static Regex> {
    GRAD_CELL_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "GRAD_CELL_REGEX",
            source: source.clone(),
        })
}

fn season_cell_regex() -> CrawlResult<&'static Regex> {
    SEASON_CELL_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "SEASON_CELL_REGEX",
            source: source.clone(),
        })
}

mod team_index;
pub use team_index::{parse_team_index, TeamIndexRead};

pub(in crate::milesplit) mod markup;
pub(in crate::milesplit) mod roster;
pub use roster::parse_roster;

pub(super) static MEET_INDEX_MARKER_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(
    || {
        Regex::new(
            r#"(?s)<section class="meet-month" data-month="(\d{4}-\d{2})"|<li class="meet-row"(.*?)</li>"#,
        )
    },
);

pub(super) static MEET_ROW_ID_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"data-meet-id="(\d+)""#));

pub(super) static MEET_ROW_LINK_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"<a class="meet-row__name" href="([^"]+)">([^<]*)</a>"#));

pub(super) static MEET_ROW_DAY_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"<span class="meet-row__day">([^<]*)</span>"#));

pub(super) static MEET_ROW_VENUE_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"<span class="meet-row__venue">([^<]*)</span>"#));

mod entities;

pub(super) mod meet_index;

pub use meet_index::{has_next_page, parse_meet_index, parse_meet_result_files};

pub(in crate::milesplit) use entities::html_unescape;
