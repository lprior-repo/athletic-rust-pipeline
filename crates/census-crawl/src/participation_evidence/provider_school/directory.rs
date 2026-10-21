use super::super::{invalid, markup, RetainedHtmlCaptureV1};
use super::{row, ProviderSchoolDirectoryProofV1};
use crate::milesplit::{parse_team_index, validate_retained_index_surface, Site, TeamRef};
use crate::{CollectionDisposition, CrawlResult};
use census_domain::UsJurisdiction;
use census_store::StoreResult;
use chrono::{DateTime, Utc};

pub(super) struct Selected {
    pub(super) team: TeamRef,
    pub(super) site: Site,
    pub(super) proof: ProviderSchoolDirectoryProofV1,
}

pub(super) fn read(capture: &RetainedHtmlCaptureV1, roster_url: &str) -> CrawlResult<Selected> {
    let html = capture.body()?;
    read_with(
        &html,
        capture.requested_url(),
        capture.body_sha256(),
        capture.acquired_at(),
        |teams, site| select(teams, roster_url, site),
    )
}

pub(super) fn read_with(
    html: &str,
    requested_url: &str,
    body_sha256: &str,
    acquired_at: DateTime<Utc>,
    select_owner: impl FnOnce(&[TeamRef], &Site) -> StoreResult<TeamRef>,
) -> CrawlResult<Selected> {
    let site = site(requested_url)?;
    validate_retained_index_surface(html, requested_url)?;
    markup::canonical(html, requested_url)?;
    let parsed = parse_team_index(html)?;
    if parsed.disposition != CollectionDisposition::Complete
        || parsed.errors != 0
        || !parsed.unfinished.is_empty()
    {
        return Err(invalid("provider school directory interpretation is incomplete").into());
    }
    let team = select_owner(&parsed.teams, &site)?;
    let spans = row::read(html, &team)?;
    Ok(Selected {
        team,
        site,
        proof: ProviderSchoolDirectoryProofV1 {
            capture_sha256: body_sha256.into(),
            acquired_at,
            disposition: parsed.disposition,
            parsed_teams: parsed.teams.len(),
            errors: parsed.errors,
            unfinished: parsed.unfinished,
            row: spans.row,
            row_sha256: spans.row_sha256,
            name: spans.name,
            city: spans.city,
            anchor: spans.anchor,
        },
    })
}

fn site(requested: &str) -> StoreResult<Site> {
    let url =
        url::Url::parse(requested).map_err(|_| invalid("provider school directory URL invalid"))?;
    let code = url
        .host_str()
        .and_then(|host| host.strip_suffix(".milesplit.com"))
        .and_then(UsJurisdiction::from_code)
        .ok_or_else(|| invalid("provider school directory jurisdiction missing"))?;
    let site = Site::for_jurisdiction(code);
    if requested != site.teams_url() {
        return Err(invalid(
            "provider school directory URL differs from exact public index",
        ));
    }
    Ok(site)
}

pub(super) fn select(teams: &[TeamRef], roster_url: &str, site: &Site) -> StoreResult<TeamRef> {
    let owner_url = roster_url
        .strip_suffix("/roster")
        .ok_or_else(|| invalid("provider school roster URL is not exact"))?;
    let mut candidates = teams.iter().filter(|team| team.url == owner_url);
    let selected = candidates
        .next()
        .ok_or_else(|| invalid("provider school directory owner absent"))?;
    if candidates.next().is_some()
        || teams.iter().filter(|team| team.id == selected.id).count() != 1
    {
        return Err(invalid("provider school directory owner is ambiguous"));
    }
    let url =
        url::Url::parse(&selected.url).map_err(|_| invalid("provider school team URL invalid"))?;
    if url.host_str() != Some(site.host().as_str()) {
        return Err(invalid(
            "provider school directory owner jurisdiction differs",
        ));
    }
    Ok(selected.clone())
}
