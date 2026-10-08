use super::super::{Accumulator, Stats};
use super::acquired::AcquiredMeet;
use super::{ProviderSchools, RawPage, ResultSetRef};
use crate::milesplit::map::{OwnedResultSet, RowWriter};
use crate::net::FetchOutcome;
use crate::{CrawlError, CrawlResult};

pub(super) struct Input<'a> {
    pub(super) acquired: &'a AcquiredMeet,
    pub(super) reference: &'a ResultSetRef,
    pub(super) page: &'a RawPage,
    pub(super) schools: &'a ProviderSchools,
    pub(super) performance_as_of: chrono::NaiveDate,
}

#[derive(Debug)]
pub(super) enum Prepared {
    Complete(Accumulator),
    Unfinished {
        projected: Accumulator,
        error: CrawlError,
    },
}

pub(super) fn prepare(
    input: &Input<'_>,
    indices: &[usize],
    stats: &mut Stats,
) -> CrawlResult<Prepared> {
    validate_capture(&input.acquired.outcome.capture, input.reference)?;
    let mut projected = Accumulator::reserved(indices.len())?;
    let Some(owned) = input
        .acquired
        .result_set(&input.reference.rsid, input.performance_as_of)
    else {
        return Ok(Prepared::Complete(projected));
    };
    match crate::milesplit::map::absorb_result_set(
        input.page,
        input.reference,
        OwnedResultSet {
            capture: owned.capture,
            page: owned.page,
            indices,
            performance_as_of: input.performance_as_of,
        },
        &mut RowWriter {
            schools: input.schools,
            stats,
            accumulated: &mut projected,
        },
    ) {
        Ok(_) => Ok(Prepared::Complete(projected)),
        Err(
            error @ (CrawlError::EventIdentity(_)
            | CrawlError::Specification(_)
            | CrawlError::PerformanceDateUnknown { .. }),
        ) => Ok(Prepared::Unfinished { projected, error }),
        Err(error) => Err(error),
    }
}

fn validate_capture(capture: &FetchOutcome, reference: &ResultSetRef) -> CrawlResult<()> {
    let requested = crate::milesplit::fetch::owned_meet_url(reference)?;
    if capture.url != requested || capture.method != "GET" {
        return Err(schema(
            reference,
            "owned capture request does not match requested meet endpoint",
        ));
    }
    let Some(response) = &capture.response_url else {
        return Ok(());
    };
    let response = url::Url::parse(response).map_err(|error| {
        schema(
            reference,
            &format!("owned response URL did not parse: {error}"),
        )
    })?;
    if !matches_response(&response, reference) {
        return Err(schema(
            reference,
            "observed owned response URL does not match requested meet endpoint",
        ));
    }
    Ok(())
}

fn matches_response(response: &url::Url, reference: &ResultSetRef) -> bool {
    matches!(response.scheme(), "http" | "https")
        && response.username().is_empty()
        && response.password().is_none()
        && response.port().is_none()
        && response.fragment().is_none()
        && response.path() == format!("/api/v1/meets/{}/performances", reference.meet_id)
        && response.host_str().is_some_and(|host| {
            host == "www.milesplit.com"
                || host
                    .strip_suffix(".milesplit.com")
                    .is_some_and(|code| code.eq_ignore_ascii_case(reference.site.code()))
        })
}

fn schema(reference: &ResultSetRef, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: reference.url.clone(),
        detail: detail.into(),
    }
}
