use super::super::{Accumulator, Stats};
use super::acquired::AcquiredMeet;
use super::{ProviderSchools, RawPage, ResultSetRef};
use crate::net::FetchOutcome;
use crate::{CrawlError, CrawlResult};

pub(super) fn prepare(
    acquired: &AcquiredMeet,
    reference: &ResultSetRef,
    page: &RawPage,
    schools: &ProviderSchools,
    stats: &mut Stats,
) -> CrawlResult<(Accumulator, bool)> {
    validate_capture(&acquired.outcome.capture, reference)?;
    let mut projected = Accumulator::default();
    let Some(owned) = acquired.result_set(&reference.rsid) else {
        return Ok((projected, false));
    };
    if owned.indices.is_empty() {
        stats.result_sets_empty = stats.result_sets_empty.saturating_add(1);
    }
    let complete = owned.page.individual_parse_complete();
    crate::milesplit::map::absorb_result_set(
        page,
        reference,
        owned,
        schools,
        stats,
        &mut projected,
    )?;
    Ok((projected, complete))
}

fn validate_capture(capture: &FetchOutcome, reference: &ResultSetRef) -> CrawlResult<()> {
    let requested = crate::milesplit::fetch::owned_meet_url(reference)?;
    if capture.url != requested || capture.method != "GET" {
        return Err(CrawlError::Schema {
            url: reference.url.clone(),
            detail: "owned capture request does not match requested meet endpoint".into(),
        });
    }
    let Some(response) = &capture.response_url else {
        return Ok(());
    };
    let response = url::Url::parse(response).map_err(|error| CrawlError::Schema {
        url: reference.url.clone(),
        detail: format!("owned response URL did not parse: {error}"),
    })?;
    let path = format!("/api/v1/meets/{}/performances", reference.meet_id);
    if !matches!(response.scheme(), "http" | "https")
        || !response.username().is_empty()
        || response.password().is_some()
        || response.port().is_some()
        || response.fragment().is_some()
        || response.path() != path
        || !response.host_str().is_some_and(|host| {
            host == "www.milesplit.com"
                || host
                    .strip_suffix(".milesplit.com")
                    .is_some_and(|code| code.eq_ignore_ascii_case(reference.site.code()))
        })
    {
        return Err(CrawlError::Schema {
            url: reference.url.clone(),
            detail: "observed owned response URL does not match requested meet endpoint".into(),
        });
    }
    Ok(())
}
