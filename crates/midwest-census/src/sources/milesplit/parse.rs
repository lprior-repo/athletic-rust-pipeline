//! The team index and roster readers: literal markup patterns in, `TeamRef`s and athletes out.
use crate::sources::{CrawlError, CrawlResult};
use census_domain::model::{flip_last_first, Gender, GradYear};
use regex::Regex;
use std::sync::LazyLock;

use super::wire::{Roster, RosterAthlete, TeamRef};

static TEAM_ROW_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<tr>\s*<td>\s*<a href="(https?://[a-z]{2}\.milesplit\.com/teams/(\d+)-([^"]+))">\s*([^<]+?)\s*</a>\s*</td>\s*<td>\s*([^<]*?)\s*</td>"#,
    )
});

static ATHLETE_ROW_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?s)<li class="athlete-row data-row">(.*?)</li>"#));

static ATHLETE_LINK_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r#"<a href="(https?://[a-z]{2}\.milesplit\.com/athletes/(\d+)-[^"]*)">([^<]+)</a>"#)
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

// Accessors for the literal patterns above: a failed compile is a programming error, so it comes
// back as a typed error the parsers hand to the caller — never a panic.
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

/// Parse the per-state team index page.
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
            .unwrap_or_default();
        let city_state = capture
            .get(5)
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
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
        // The page carries no URL of its own, so the shape failure is reported against its text.
        return Err(CrawlError::Invariant {
            detail: "team index contained no team rows (markup change or empty state page)"
                .to_string(),
        });
    }
    Ok(teams)
}

/// Parse a graded roster page.
pub fn parse_roster(html: &str, team: TeamRef) -> CrawlResult<Roster> {
    let row_regex = athlete_row_regex()?;
    let link = athlete_link_regex()?;
    let gender_cell = gender_cell_regex()?;
    let grad_cell = grad_cell_regex()?;
    let season_cell = season_cell_regex()?;

    let mut athletes = Vec::new();
    for row in row_regex.captures_iter(html) {
        let Some(row_match) = row.get(1) else {
            continue;
        };
        let Some(athlete) = roster_athlete(
            row_match.as_str(),
            link,
            gender_cell,
            grad_cell,
            season_cell,
        )?
        else {
            continue;
        };
        athletes.push(athlete);
    }
    Ok(Roster { team, athletes })
}

/// One `<li class="athlete-row data-row">` row as an athlete, or `None` for a row that carries no
/// usable athlete: no profile link, no roster name, or no parseable graduating year.
fn roster_athlete(
    row_html: &str,
    link: &Regex,
    gender_cell: &Regex,
    grad_cell: &Regex,
    season_cell: &Regex,
) -> CrawlResult<Option<RosterAthlete>> {
    let Some(athlete) = link.captures(row_html) else {
        return Ok(None);
    };
    let Some(url_match) = athlete.get(1) else {
        return Ok(None);
    };
    let Some(id_match) = athlete.get(2) else {
        return Ok(None);
    };
    let roster_name = athlete
        .get(3)
        .map(|m| html_unescape(m.as_str().trim()))
        .unwrap_or_default();
    if roster_name.is_empty() {
        return Ok(None);
    }
    let gender = gender_cell
        .captures(row_html)
        .and_then(|capture| Some(Gender::parse_milesplit(capture.get(1)?.as_str())))
        .unwrap_or(Gender::Unknown);
    let Some(grad_year) = grad_cell.captures(row_html).and_then(|capture| {
        let grad_str = capture.get(1)?.as_str().trim();
        let parsed = grad_str.parse::<i16>().ok()?;
        GradYear::new(parsed)
    }) else {
        return Ok(None);
    };
    let (indoor, outdoor, xc) = active_seasons(season_cell, row_html);

    Ok(Some(RosterAthlete {
        name: flip_last_first(&roster_name),
        roster_name,
        gender,
        grad_year,
        athlete_id: id_match.as_str().to_string(),
        profile_url: url_match.as_str().to_string(),
        indoor,
        outdoor,
        xc,
    }))
}

/// Which of the row's three season cells are active, as `(indoor, outdoor, xc)`.
///
/// Cells appear in header order: Indoor, Outdoor, XC. The `data-season-id` is informational
/// (1 = indoor, 2 = outdoor, 3 = XC); position is authoritative.
fn active_seasons(season_cell: &Regex, row_html: &str) -> (bool, bool, bool) {
    let mut seasons = Vec::new();
    for capture in season_cell.captures_iter(row_html) {
        let season_id: u8 = capture
            .get(1)
            .map(|m| m.as_str().parse::<u8>())
            .and_then(|r| r.ok())
            .unwrap_or(0);
        let active = capture.get(2).map(|m| m.as_str() == "yes").unwrap_or(false);
        seasons.push((season_id, active));
    }
    let indoor = seasons.first().map(|(_, active)| *active).unwrap_or(false);
    let outdoor = seasons.get(1).map(|(_, active)| *active).unwrap_or(false);
    let xc = seasons.get(2).map(|(_, active)| *active).unwrap_or(false);
    (indoor, outdoor, xc)
}

pub(super) fn html_unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}
