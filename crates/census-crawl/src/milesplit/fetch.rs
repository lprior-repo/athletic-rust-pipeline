//! The three fetches: a state team index, one roster page, and one `/raw` result set — each
//! status-checked before parsing.
use crate::net::{FetchError, FetchOptions, Fetcher};
use crate::{CrawlError, CrawlResult};

use super::parse::{
    has_next_page, parse_meet_index, parse_meet_result_files, parse_roster, parse_team_index,
};
use super::raw::{parse_raw, RawPage};
use super::wire::{MeetRef, MeetResultFile, ResultSetRef, Roster, Season, Site, TeamRef};

/// Fetch and parse the team index for a state site.
pub async fn fetch_team_index(
    fetcher: &Fetcher,
    site: Site,
    options: &FetchOptions,
) -> CrawlResult<Vec<TeamRef>> {
    let outcome = fetcher.get(&site.teams_url(), options).await?;
    if outcome.status != 200 {
        // A caller that set `allow_not_found` gets the 404 as an outcome rather than an error; the
        // status is still a fetch failure to every caller that asked for the index itself.
        return Err(CrawlError::Fetch(FetchError::Http {
            status: outcome.status,
            url: outcome.url.clone(),
        }));
    }
    parse_team_index(&outcome.text())
}

/// Fetch and parse one roster.
pub async fn fetch_roster(
    fetcher: &Fetcher,
    team: &TeamRef,
    options: &FetchOptions,
) -> CrawlResult<Roster> {
    let url = format!("{}/roster", team.url);
    let outcome = fetcher
        .get(
            &url,
            &FetchOptions {
                allow_not_found: true,
                ..options.clone()
            },
        )
        .await?;
    if outcome.status == 404 {
        return Ok(Roster {
            team: team.clone(),
            athletes: Vec::new(),
        });
    }
    if outcome.status != 200 {
        return Err(CrawlError::Fetch(FetchError::Http {
            status: outcome.status,
            url,
        }));
    }
    parse_roster(&outcome.text(), team.clone())
}

/// Fetch and parse one `/raw` result set: one request, the whole result set, no pagination.
pub async fn fetch_result_set(
    fetcher: &Fetcher,
    reference: &ResultSetRef,
    options: &FetchOptions,
) -> CrawlResult<RawPage> {
    let outcome = fetcher.get(&reference.url, options).await?;
    if outcome.status != 200 {
        return Err(CrawlError::Fetch(FetchError::Http {
            status: outcome.status,
            url: reference.url.clone(),
        }));
    }
    parse_raw(&outcome.text(), &reference.url)
}

/// One page of a state's results index, with whether the pager published a next page.
///
/// The index is the meet census's enumeration: one request per fifty meets, ordered by date, with
/// the pagination the site itself publishes (`rel="next"`). Nothing here walks into the meet — a
/// caller that wants results asks for them by the id and URL this returns.
/// Read a meet's results page and return the result files it lists.
///
/// One request. The page is the meet's own results URL — the same URL the meet census stores in
/// `source_meets.results_url` — so a whole meet costs this plus one request per result file.
pub async fn fetch_meet_result_files(
    fetcher: &Fetcher,
    results_url: &str,
    options: &FetchOptions,
) -> CrawlResult<Vec<MeetResultFile>> {
    let outcome = fetcher.get(results_url, options).await?;
    if outcome.status != 200 {
        return Err(CrawlError::Fetch(FetchError::Http {
            status: outcome.status,
            url: outcome.url.clone(),
        }));
    }
    parse_meet_result_files(&outcome.url, &outcome.text())
}

pub async fn fetch_meet_index(
    fetcher: &Fetcher,
    site: Site,
    season: Season,
    year: u16,
    page: u32,
    options: &FetchOptions,
) -> CrawlResult<(Vec<MeetRef>, bool)> {
    let url = site.results_url(season, year, page);
    let outcome = fetcher.get(&url, options).await?;
    if outcome.status != 200 {
        return Err(CrawlError::Fetch(FetchError::Http {
            status: outcome.status,
            url: outcome.url.clone(),
        }));
    }
    let body = outcome.text();
    let meets = parse_meet_index(&body)?;
    Ok((meets, has_next_page(&body)))
}
