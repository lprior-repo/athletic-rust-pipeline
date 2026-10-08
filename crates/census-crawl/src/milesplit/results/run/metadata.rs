use super::ResultSetRef;
use crate::milesplit::raw::{parse_bound, RawPage};
use crate::net::{FetchError, FetchOutcome};
use crate::{AdapterContext, CrawlError, CrawlResult};

mod admission;

#[tracing::instrument(skip(ctx))]
pub(super) async fn fetch_metadata(
    ctx: &AdapterContext<'_>,
    reference: &ResultSetRef,
) -> CrawlResult<(FetchOutcome, CrawlResult<RawPage>)> {
    check_url(&reference.url, reference, "requested raw URL")?;
    let capture = ctx
        .fetcher
        .get(&reference.url, &ctx.fetch_options())
        .await?;
    let page = parse_capture(&capture, reference);
    Ok((capture, page))
}

pub(super) fn parse_capture(
    capture: &FetchOutcome,
    reference: &ResultSetRef,
) -> CrawlResult<RawPage> {
    check_capture(capture, reference)?;
    let html = std::str::from_utf8(&capture.body)
        .map_err(|_| schema(reference, "raw metadata is not UTF-8"))?;
    parse_bound(html, &reference.url, &|url, role| {
        check_url(url, reference, role)
    })
}

fn check_capture(capture: &FetchOutcome, reference: &ResultSetRef) -> CrawlResult<()> {
    if capture.status != 200 {
        return Err(FetchError::Http {
            status: capture.status,
            url: capture.url.clone(),
        }
        .into());
    }
    if capture.body.len() > crate::net::MAX_BODY_BYTES {
        return Err(FetchError::TooLarge {
            url: capture.url.clone(),
        }
        .into());
    }
    admission::check(&capture.body)?;
    check_url(&reference.url, reference, "requested raw URL")?;
    check_url(&capture.url, reference, "captured raw URL")?;
    if let Some(url) = &capture.response_url {
        check_url(url, reference, "observed raw response URL")?;
    }
    Ok(())
}

fn check_url(url: &str, reference: &ResultSetRef, role: &str) -> CrawlResult<()> {
    if matches_reference(url, reference) {
        return Ok(());
    }
    Err(schema(
        reference,
        &format!("{role} does not match requested meet/result-set"),
    ))
}

fn matches_reference(url: &str, reference: &ResultSetRef) -> bool {
    let Ok(url) = url::Url::parse(url) else {
        return false;
    };
    allowed_url(&url, reference) && matches_path(&url, reference)
}

fn allowed_url(url: &url::Url, reference: &ResultSetRef) -> bool {
    matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.host_str().is_some_and(|host| {
            host == "www.milesplit.com"
                || host
                    .strip_suffix(".milesplit.com")
                    .is_some_and(|code| code.eq_ignore_ascii_case(reference.site.code()))
        })
}

fn matches_path(url: &url::Url, reference: &ResultSetRef) -> bool {
    let Some(mut segments) = url.path_segments() else {
        return false;
    };
    if segments.next() != Some("meets") {
        return false;
    }
    let Some(meet) = segments.next() else {
        return false;
    };
    let meet = meet.split_once('-').map_or(meet, |pair| pair.0);
    if !positive_id(meet) || meet != reference.meet_id || segments.next() != Some("results") {
        return false;
    }
    let Some(result_set) = segments.next() else {
        return false;
    };
    positive_id(result_set)
        && result_set == reference.rsid
        && segments.next() == Some("raw")
        && segments.next().is_none()
}

fn positive_id(token: &str) -> bool {
    !token.starts_with('0')
        && token.bytes().all(|byte| byte.is_ascii_digit())
        && token.parse::<u64>().is_ok_and(|id| id > 0)
}

fn schema(reference: &ResultSetRef, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: reference.url.clone(),
        detail: detail.into(),
    }
}
