use crate::{CrawlError, CrawlResult};
use regex::Regex;
use std::sync::LazyLock;

use super::wire::TeamRef;

static TEAM_ROW_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<tr>\s*<td>\s*<a href="(https?://(?:[a-z]{2}|www)\.milesplit\.com/teams/(\d+)(?:-([^"]+))?)">\s*([^<]+?)\s*</a>\s*</td>\s*<td>\s*([^<]*?)\s*</td>"#,
    )
});

static ATHLETE_ROW_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?s)<li class="athlete-row data-row">(.*?)</li>"#));

static ATHLETE_LINK_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"<a href="(https?://(?:[a-z0-9-]+\.)*milesplit\.com/athletes/(\d+)(?:-[^"]*)?)">([^<]*)</a>"#,
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

static PROFILE_NAME_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r#"<h1 id="athleteName"[^>]*>([^<]+)</h1>"#)
});

static PROFILE_SCHOOL_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r#"<span class="current-school"><a\s+href="([^"]+)">([^<]+)</a></span>"#)
});

static PROFILE_TEAM_ID_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"/teams/(\d+)"#));

static PROFILE_GRAD_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r#"<span class="grad-year">Class of (\d{4})</span>"#)
});

static PROFILE_CITY_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r#"<span class="city-state">([^<]*)</span>"#)
});

static PROFILE_GENDER_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#""gender":\s*"([^"]+)""#));

static PROFILE_CANONICAL_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r#"<link rel="canonical" href="([^"]+/athletes/(\d+)[^"]*)""#)
});

static PROFILE_TABLE_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r#"(?s)<table id="prTable".*?<tbody>(.*?)</tbody>"#)
});

static PROFILE_ROW_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r#"(?s)<tr data-pr-mode="([^"]*)" data-pr-level="([^"]*)"[^>]*>(.*?)</tr>"#)
});

static PROFILE_CELL_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?s)<td[^>]*>(.*?)</td>"#));

fn profile_name_regex() -> CrawlResult<&'static Regex> {
    PROFILE_NAME_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "PROFILE_NAME_REGEX",
            source: source.clone(),
        })
}

fn profile_school_regex() -> CrawlResult<&'static Regex> {
    PROFILE_SCHOOL_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "PROFILE_SCHOOL_REGEX",
            source: source.clone(),
        })
}

fn profile_team_id_regex() -> CrawlResult<&'static Regex> {
    PROFILE_TEAM_ID_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "PROFILE_TEAM_ID_REGEX",
            source: source.clone(),
        })
}

fn profile_grad_regex() -> CrawlResult<&'static Regex> {
    PROFILE_GRAD_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "PROFILE_GRAD_REGEX",
            source: source.clone(),
        })
}

fn profile_city_regex() -> CrawlResult<&'static Regex> {
    PROFILE_CITY_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "PROFILE_CITY_REGEX",
            source: source.clone(),
        })
}

fn profile_gender_regex() -> CrawlResult<&'static Regex> {
    PROFILE_GENDER_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "PROFILE_GENDER_REGEX",
            source: source.clone(),
        })
}

fn profile_canonical_regex() -> CrawlResult<&'static Regex> {
    PROFILE_CANONICAL_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "PROFILE_CANONICAL_REGEX",
            source: source.clone(),
        })
}

fn profile_table_regex() -> CrawlResult<&'static Regex> {
    PROFILE_TABLE_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "PROFILE_TABLE_REGEX",
            source: source.clone(),
        })
}

fn profile_row_regex() -> CrawlResult<&'static Regex> {
    PROFILE_ROW_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "PROFILE_ROW_REGEX",
            source: source.clone(),
        })
}

fn profile_cell_regex() -> CrawlResult<&'static Regex> {
    PROFILE_CELL_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "PROFILE_CELL_REGEX",
            source: source.clone(),
        })
}

mod team_index;
pub use team_index::{parse_team_index, TeamIndexRead};

pub mod profile;
pub use profile::{
    parse_profile, ParsedProfile, ProfileHeader, ProfilePersonalRecord, ProfileQuarantine,
    ProfileRowRejection, ProfileRowRejectionKind, ProfileVerdict,
};

pub(in crate::milesplit) mod markup;
pub(in crate::milesplit) mod roster;
pub(crate) use roster::parse_captured_roster_with_witness_v1;
pub use roster::{parse_captured_roster, parse_roster};

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

pub(in crate::milesplit) mod meet_index;

pub use meet_index::{has_next_page, parse_meet_index, parse_meet_result_files};

pub(in crate::milesplit) use entities::html_unescape;
