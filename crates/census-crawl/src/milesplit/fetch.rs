use crate::net::{FetchError, FetchOptions, Fetcher};
use crate::{CrawlError, CrawlResult};

use super::parse::{has_next_page, parse_meet_index, parse_meet_result_files, parse_team_index};
use super::raw::{parse_raw, RawPage};
use super::roster::{RosterOutcome, RosterQuarantine, RosterVerdict};
use super::wire::{MeetRef, MeetResultFile, ResultSetRef, Season, Site, TeamRef};

pub async fn fetch_team_index(
    fetcher: &Fetcher,
    site: Site,
    options: &FetchOptions,
) -> CrawlResult<Vec<TeamRef>> {
    let outcome = fetcher.get(&site.teams_url(), options).await?;
    if outcome.status != 200 {
        return Err(CrawlError::Fetch(FetchError::Http {
            status: outcome.status,
            url: outcome.url.clone(),
        }));
    }
    parse_team_index(&outcome.text())
}

pub async fn fetch_roster(
    fetcher: &Fetcher,
    team: &TeamRef,
    options: &FetchOptions,
) -> CrawlResult<RosterOutcome> {
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
    let verdict = match outcome.status {
        404 => RosterVerdict::Quarantined {
            reason: RosterQuarantine::NotFound,
            rejected: Vec::new(),
        },
        200 => match std::str::from_utf8(&outcome.body) {
            Ok(body) => super::parse::roster::parse_captured_roster(body, team.clone(), &outcome)?,
            Err(_) => RosterVerdict::Quarantined {
                reason: RosterQuarantine::InvalidEncoding,
                rejected: Vec::new(),
            },
        },
        status => return Err(CrawlError::Fetch(FetchError::Http { status, url })),
    };
    Ok(RosterOutcome {
        capture: outcome,
        verdict,
    })
}

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

#[tracing::instrument(skip(fetcher, options))]
pub async fn fetch_owned_meet(
    fetcher: &Fetcher,
    reference: &ResultSetRef,
    options: &FetchOptions,
) -> CrawlResult<super::owned::OwnedMeetOutcome> {
    let capture = fetch_owned_capture(fetcher, reference, options).await?;
    let meet_id = owned_meet_id(reference)?;
    let verdict = if capture.status == 200 {
        super::owned::parse_owned_meet(&capture.body, meet_id)
    } else {
        super::owned::OwnedMeetVerdict::Refused {
            status: capture.status,
        }
    };
    Ok(super::owned::OwnedMeetOutcome { capture, verdict })
}

pub(super) fn owned_meet_id(reference: &ResultSetRef) -> CrawlResult<u64> {
    reference
        .meet_id
        .parse::<u64>()
        .ok()
        .filter(|id| {
            *id > 0
                && !reference.meet_id.starts_with('0')
                && reference.meet_id.bytes().all(|byte| byte.is_ascii_digit())
        })
        .ok_or_else(|| CrawlError::Schema {
            url: reference.url.clone(),
            detail: "invalid structured meet ID".to_string(),
        })
}

pub(super) fn owned_meet_url(reference: &ResultSetRef) -> CrawlResult<String> {
    let meet_id = owned_meet_id(reference)?;
    Ok(format!(
        "https://{}/api/v1/meets/{meet_id}/performances?isMeetPro=0&fields={}",
        reference.site.host(),
        super::owned::OWNED_FIELDS
    ))
}

#[tracing::instrument(skip(fetcher, options))]
pub(super) async fn fetch_owned_capture(
    fetcher: &Fetcher,
    reference: &ResultSetRef,
    options: &FetchOptions,
) -> CrawlResult<crate::net::FetchOutcome> {
    let url = owned_meet_url(reference)?;
    Ok(fetcher
        .get(
            &url,
            &FetchOptions {
                allow_not_found: true,
                ..options.clone()
            },
        )
        .await?)
}
