use crate::milesplit::wire::TeamRef;
use crate::net::FetchOutcome;
use crate::{CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use html5gum::Tokenizer;
use url::Url;

use crate::milesplit::parse::markup;
mod published;

#[derive(Clone, Copy)]
struct Owner {
    id: u64,
    jurisdiction: Option<UsJurisdiction>,
}

pub(super) fn validate_published(html: &str, team: &TeamRef) -> CrawlResult<bool> {
    let expected = requested_owner(team)?;
    published_owner(html, team, expected)
}

pub(super) fn validate_capture(
    html: &str,
    team: &TeamRef,
    capture: &FetchOutcome,
) -> CrawlResult<()> {
    let expected = requested_owner(team)?;
    match_owner(&capture.url, team, expected, true)?;
    if let Some(final_url) = capture.response_url.as_deref() {
        match_owner(final_url, team, expected, true)?;
    }
    let published = published_owner(html, team, expected)?;
    if capture.response_url.is_none() && !published {
        return Err(refuse(
            team,
            "roster capture has neither an observed final owner nor a published owner",
        ));
    }
    Ok(())
}

fn requested_owner(team: &TeamRef) -> CrawlResult<Owner> {
    let id = provider_id(&team.id).ok_or_else(|| {
        refuse(
            team,
            "requested team ID is not a canonical positive provider ID",
        )
    })?;
    let owner = url_owner(&team.url, false)
        .ok_or_else(|| refuse(team, "requested team URL does not identify a provider team"))?;
    if owner.id != id {
        return Err(refuse(
            team,
            "requested team URL conflicts with its provider ID",
        ));
    }
    let declared = team
        .city_state
        .split(',')
        .map(str::trim)
        .find_map(UsJurisdiction::from_code);
    if declared
        .zip(owner.jurisdiction)
        .is_some_and(|(left, right)| left != right)
    {
        return Err(refuse(
            team,
            "requested team location conflicts with its source jurisdiction",
        ));
    }
    Ok(Owner {
        id,
        jurisdiction: owner.jurisdiction.or(declared),
    })
}

fn provider_id(raw: &str) -> Option<u64> {
    if raw.starts_with('0') || !raw.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u64>().ok().filter(|id| *id > 0)
}

fn url_owner(raw: &str, roster: bool) -> Option<Owner> {
    let url = Url::parse(raw).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return None;
    }
    let host = url.host_str()?;
    let jurisdiction = match host {
        "milesplit.com" | "www.milesplit.com" => None,
        _ => Some(UsJurisdiction::from_code(
            host.strip_suffix(".milesplit.com")?,
        )?),
    };
    let mut segments = url.path_segments()?;
    if segments.next()? != "teams" {
        return None;
    }
    let token = segments.next()?;
    let id = provider_id(token.split_once('-').map_or(token, |(id, _)| id))?;
    match (segments.next(), segments.next()) {
        (Some("roster"), None | Some("")) if segments.next().is_none() => {}
        (None | Some(""), None) if !roster => {}
        _ => return None,
    }
    Some(Owner { id, jurisdiction })
}

fn match_owner(raw: &str, team: &TeamRef, expected: Owner, roster: bool) -> CrawlResult<()> {
    let owner = url_owner(raw, roster).ok_or_else(|| {
        refuse(
            team,
            "roster provenance URL does not identify a real provider owner",
        )
    })?;
    if owner.id != expected.id
        || owner
            .jurisdiction
            .zip(expected.jurisdiction)
            .is_some_and(|(left, right)| left != right)
    {
        return Err(refuse(
            team,
            "roster provenance identifies a conflicting provider team or jurisdiction",
        ));
    }
    Ok(())
}

fn published_owner(html: &str, team: &TeamRef, expected: Owner) -> CrawlResult<bool> {
    let base = Url::parse(&team.url).map_err(|_| refuse(team, "invalid requested team URL"))?;
    Tokenizer::new_with_emitter(html, markup::Markup::default()).try_fold(false, |found, token| {
        let declaration =
            token.map_err(|error| refuse(team, &format!("roster HTML input failed: {error}")))?;
        let bytes = match declaration {
            markup::Declaration::Url(bytes) => bytes,
            markup::Declaration::Script(bytes) => {
                let body = std::str::from_utf8(&bytes)
                    .map_err(|_| refuse(team, "published roster script is not UTF-8"))?;
                return published::document(body, &base, team, expected)
                    .map(|published| found || published);
            }
            markup::Declaration::Invalid => {
                return Err(refuse(team, "roster HTML nesting exceeds representation"));
            }
        };
        let raw = std::str::from_utf8(&bytes)
            .map_err(|_| refuse(team, "published roster owner URL is not UTF-8"))?;
        if raw.trim().is_empty() || raw.trim().starts_with(['?', '#']) {
            return Err(refuse(
                team,
                "published roster URL does not independently identify its owner",
            ));
        }
        let resolved = base
            .join(raw)
            .map_err(|_| refuse(team, "malformed published roster owner URL"))?;
        match_owner(resolved.as_str(), team, expected, false)?;
        Ok(true)
    })
}

fn refuse(team: &TeamRef, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: team.url.clone(),
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests;
