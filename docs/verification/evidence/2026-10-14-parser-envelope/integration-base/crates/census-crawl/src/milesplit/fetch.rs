use crate::net::{FetchError, FetchOptions, Fetcher};
use crate::{CrawlError, CrawlResult};

use super::boundary;
use super::parse::{parse_meet_result_files, parse_team_index, TeamIndexRead};
use super::raw::{parse_raw, RawPage};
use super::roster::{RosterOutcome, RosterQuarantine, RosterVerdict};
use super::wire::{MeetResultFile, ResultSetRef, Site, TeamRef};

pub async fn fetch_team_index(
    fetcher: &Fetcher,
    site: Site,
    options: &FetchOptions,
) -> CrawlResult<TeamIndexRead> {
    let outcome = fetcher.get(&site.teams_url(), options).await?;
    if outcome.status != 200 {
        return Err(CrawlError::Fetch(FetchError::Http {
            status: outcome.status,
            url: outcome.url.clone(),
        }));
    }
    if outcome.body.len() > 8 * 1024 * 1024 {
        return Err(CrawlError::Resource {
            resource: "team-index capture bytes",
            requested: outcome.body.len(),
            limit: 8 * 1024 * 1024,
        });
    }
    let body = std::str::from_utf8(&outcome.body).map_err(|_| CrawlError::Schema {
        url: outcome.url.clone(),
        detail: "team index is not UTF-8".to_string(),
    })?;
    super::index::validate_surface(body, &outcome.url)?;
    parse_team_index(body)
}

pub async fn fetch_roster(
    fetcher: &Fetcher,
    team: &TeamRef,
    options: &FetchOptions,
    hook: Option<&dyn boundary::Hook>,
) -> CrawlResult<RosterOutcome> {
    let url = format!("{}/roster", team.url);
    if let Some(hook) = hook {
        hook.reached(boundary::Point::BeforeSourceRequest).await?;
    }
    let outcome = fetcher
        .get(
            &url,
            &FetchOptions {
                allow_not_found: true,
                ..options.clone()
            },
        )
        .await?;
    if let Some(hook) = hook {
        hook.reached(boundary::Point::ResponseReceived).await?;
    }
    let verdict = match outcome.status {
        404 => RosterVerdict::Quarantined {
            reason: RosterQuarantine::NotFound,
            rejected: Vec::new(),
            unfinished: None,
        },
        200 => match std::str::from_utf8(&outcome.body) {
            Ok(body) => super::parse::roster::parse_captured_roster(body, team.clone(), &outcome)?,
            Err(_) => RosterVerdict::Quarantined {
                reason: RosterQuarantine::InvalidEncoding,
                rejected: Vec::new(),
                unfinished: None,
            },
        },
        status => return Err(CrawlError::Fetch(FetchError::Http { status, url })),
    };
    let roster = RosterOutcome {
        capture: outcome,
        verdict,
    };
    if let Some(hook) = hook {
        hook.reached(boundary::Point::CaptureCommitted).await?;
    }
    if let Some(roster) = roster.verdict.roster() {
        let chunk_size = boundary::CHUNK_ROWS;
        let chunk_count = roster.athletes.len().div_ceil(chunk_size);
        for index in 0..chunk_count {
            if let Some(hook) = hook {
                let index = u32::try_from(index).map_err(|_| CrawlError::Arithmetic {
                    detail: format!("page chunk index {index} exceeds the boundary point range"),
                })?;
                hook.reached(boundary::Point::PageChunk { index }).await?;
            }
        }
    }
    Ok(roster)
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

#[tracing::instrument(skip(ctx, options))]
pub async fn fetch_owned_meet(
    ctx: &crate::AdapterContext<'_>,
    reference: &ResultSetRef,
    options: &FetchOptions,
) -> CrawlResult<super::owned::OwnedMeetOutcome> {
    let acquired = fetch_owned_capture(ctx, reference, options).await?;
    let capture = acquired.capture;
    let meet_id = owned_meet_id(reference)?;
    let verdict = if capture.status == 200 {
        match super::owned::parse_owned_meet(&capture.body, meet_id) {
            Ok(page) => super::owned::OwnedMeetVerdict::Parsed(page),
            Err(super::owned::OwnedMeetParseError::Malformed(failure)) => {
                super::owned::OwnedMeetVerdict::Malformed {
                    detail: failure.to_string(),
                }
            }
            Err(super::owned::OwnedMeetParseError::Resource(failure)) => {
                return Err(
                    super::owned::retained_parse_resource(ctx.fetcher, capture, failure)?.into(),
                );
            }
        }
    } else {
        super::owned::OwnedMeetVerdict::Refused {
            status: capture.status,
        }
    };
    Ok(super::owned::OwnedMeetOutcome {
        capture,
        verdict,
        provenance: acquired.provenance,
    })
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

#[tracing::instrument(skip(ctx, options))]
pub(super) async fn fetch_owned_capture(
    ctx: &crate::AdapterContext<'_>,
    reference: &ResultSetRef,
    options: &FetchOptions,
) -> CrawlResult<crate::net::CapturedFetch> {
    let url = owned_meet_url(reference)?;
    let options = FetchOptions {
        allow_not_found: true,
        ..options.clone()
    };
    match ctx.capture {
        crate::CaptureContext::LegacyMetadata => {
            ctx.store.require_legacy_structured_capture()?;
            Ok(crate::net::CapturedFetch {
                capture: ctx.fetcher.get(&url, &options).await?,
                provenance: crate::net::CaptureProvenance::HistoricalAttemptUnknown,
            })
        }
        crate::CaptureContext::DurableV1(owner) => {
            let request = ctx.fetcher.acquisition_request(&url, &options)?;
            let work = owner.register(
                ctx.store,
                ctx.fetcher,
                reference.site.jurisdiction(),
                &reference.meet_id,
                request,
            )?;
            Ok(ctx
                .fetcher
                .get_acquisition(ctx.store, work, &options)
                .await?)
        }
    }
}
