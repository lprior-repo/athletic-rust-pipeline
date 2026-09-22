//! The three fetches: a state team index, one roster page, and one `/raw` result set — each
//! status-checked before parsing.
use crate::net::{FetchError, FetchOptions, Fetcher};
use crate::sources::{CrawlError, CrawlResult};

use super::parse::{parse_roster, parse_team_index};
use super::raw::{parse_raw, RawPage};
use super::wire::{ResultSetRef, Roster, Site, TeamRef};

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
