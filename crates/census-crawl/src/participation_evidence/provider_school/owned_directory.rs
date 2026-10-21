use super::super::roster_source_issuance::current_roster_semantic_kernel_digest;
use super::super::{bound, hash, invalid};
use super::{directory, ProviderSchoolDirectoryProofV1};
use crate::milesplit::{Site, TeamRef};
use crate::net::FetchOutcome;
use crate::CrawlResult;
use census_domain::model::{AcquisitionDigest, CaptureDomainError, PublicSourceResourceV1};
use census_store::{CaptureError, StoreResult};
use chrono::{DateTime, Utc};

const MAX_DIRECTORY_BODY: usize = 8 * 1024 * 1024;

pub(crate) fn read_directory_roster_child(
    capture: &FetchOutcome,
    team_id: &str,
) -> CrawlResult<(TeamRef, ProviderSchoolDirectoryProofV1, AcquisitionDigest)> {
    PublicSourceResourceV1::milesplit_team_roster(team_id).map_err(domain)?;
    let kernel = current_roster_semantic_kernel_digest()?;
    let html = body(capture)?;
    let acquired_at = acquired_at(capture)?;
    let selected = directory::read_with(
        html,
        &capture.url,
        &capture.content_digest,
        acquired_at,
        |teams, site| select_native(teams, team_id, site),
    )?;
    Ok((
        selected.team,
        selected.proof,
        AcquisitionDigest::parse(kernel).map_err(domain)?,
    ))
}

fn body(capture: &FetchOutcome) -> StoreResult<&str> {
    bound(
        capture.body.len(),
        MAX_DIRECTORY_BODY,
        "owned directory body bytes",
    )?;
    if capture.body.is_empty()
        || capture.bytes != capture.body.len()
        || hash(&capture.body) != capture.content_digest
        || capture.method != "GET"
        || capture.status != 200
        || capture.response_url.as_deref() != Some(capture.url.as_str())
        || !supported_type(capture.content_type.as_deref())
    {
        return Err(invalid("owned directory original HTML capture differs"));
    }
    std::str::from_utf8(&capture.body).map_err(|_| invalid("owned directory body is not UTF-8"))
}

fn supported_type(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        value.eq_ignore_ascii_case("text/html")
            || value.eq_ignore_ascii_case("text/html; charset=utf-8")
    })
}

fn acquired_at(capture: &FetchOutcome) -> StoreResult<DateTime<Utc>> {
    let value = DateTime::parse_from_rfc3339(&capture.fetched_at)
        .map_err(|_| invalid("owned directory acquisition timestamp invalid"))?;
    if value.offset().local_minus_utc() != 0 {
        return Err(invalid("owned directory acquisition timestamp is not UTC"));
    }
    Ok(value.with_timezone(&Utc))
}

fn select_native(teams: &[TeamRef], native: &str, site: &Site) -> StoreResult<TeamRef> {
    let selected = teams
        .iter()
        .find(|team| team.id == native)
        .ok_or_else(|| invalid("provider school directory owner absent"))?;
    let roster_url = roster_url(&selected.url)?;
    PublicSourceResourceV1::milesplit_team_roster(native)
        .and_then(|resource| resource.validate_target(site.jurisdiction(), &roster_url))
        .map_err(domain)?;
    directory::select(teams, &roster_url, site)
}

fn roster_url(team_url: &str) -> StoreResult<String> {
    let bytes = team_url
        .len()
        .checked_add("/roster".len())
        .ok_or(census_store::StoreError::CounterOverflow)?;
    bound(bytes, 2048, "owned directory roster URL bytes")?;
    let mut url = String::new();
    url.try_reserve_exact(bytes)
        .map_err(|_| invalid("owned directory roster URL allocation refused"))?;
    url.push_str(team_url);
    url.push_str("/roster");
    Ok(url)
}

fn domain(error: CaptureDomainError) -> census_store::StoreError {
    CaptureError::Domain(error).into()
}
