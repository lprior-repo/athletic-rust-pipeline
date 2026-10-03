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

pub fn parse_team_index(html: &str) -> CrawlResult<Vec<TeamRef>> {
    let row_regex = team_row_regex()?;
    let mut teams = Vec::new();
    for capture in row_regex.captures_iter(html) {
        let Some(url_match) = capture.get(1) else {
            continue;
        };
        let Some(id_match) = capture.get(2) else {
            continue;
        };
        let Some(slug_match) = capture.get(3) else {
            continue;
        };
        let name = capture
            .get(4)
            .map(|m| m.as_str().trim().to_string())
            .map_or(Default::default(), core::convert::identity);
        let city_state = capture
            .get(5)
            .map(|m| m.as_str().trim().to_string())
            .map_or(Default::default(), core::convert::identity);
        if name.is_empty() {
            continue;
        }
        teams.push(TeamRef {
            id: id_match.as_str().to_string(),
            slug: slug_match.as_str().to_string(),
            url: url_match.as_str().to_string(),
            name,
            city_state,
        });
    }
    if teams.is_empty() {
        return Err(CrawlError::Invariant {
            detail: "team index contained no team rows (markup change or empty state page)"
                .to_string(),
        });
    }
    Ok(teams)
}

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
