use super::ResultSetRef;
use crate::milesplit::raw::{parse_raw, RawPage};
use crate::net::{FetchError, FetchOutcome};
use crate::{AdapterContext, CrawlError, CrawlResult};
use regex::Regex;
use serde::Deserialize;
use std::borrow::Cow;
use std::sync::LazyLock;

type RegexResult<T, E = regex::Error> = Result<T, E>;

static LINKS: LazyLock<RegexResult<Regex>> = LazyLock::new(|| Regex::new(r"(?is)<link\b[^>]*>"));
static ATTRIBUTES: LazyLock<RegexResult<Regex>> =
    LazyLock::new(|| Regex::new(r#"(?i)\b(rel|href)\s*=\s*["']([^"']*)["']"#));

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
    check_url(&reference.url, reference, "requested raw URL")?;
    check_url(&capture.url, reference, "captured raw URL")?;
    if let Some(url) = &capture.response_url {
        check_url(url, reference, "observed raw response URL")?;
    }
    let html = std::str::from_utf8(&capture.body)
        .map_err(|_| schema(reference, "raw metadata is not UTF-8"))?;
    check_document(html, reference)?;
    parse_raw(html, &reference.url)
}

#[derive(Deserialize)]
struct PublishedOwner<'a> {
    #[serde(rename = "@type")]
    #[serde(borrow)]
    kind: Cow<'a, str>,
    #[serde(borrow)]
    url: Cow<'a, str>,
}

fn check_document(html: &str, reference: &ResultSetRef) -> CrawlResult<()> {
    let block = [
        "<script type=\"application/ld+json\">",
        "<script type='application/ld+json'>",
    ]
    .iter()
    .find_map(|open| {
        html.split_once(open)?
            .1
            .split_once("</script>")
            .map(|pair| pair.0)
    })
    .ok_or_else(|| schema(reference, "raw metadata has no published document owner"))?;
    let owner: PublishedOwner<'_> = serde_json::from_str(block).map_err(|error| {
        schema(
            reference,
            &format!("raw document owner did not decode: {error}"),
        )
    })?;
    if owner.kind != "SportsEvent" {
        return Err(schema(
            reference,
            "raw metadata document is not a SportsEvent",
        ));
    }
    check_url(&owner.url, reference, "published raw document URL")?;
    let links = regex(&LINKS, "MILESPLIT_RAW_LINKS")?;
    let attributes = regex(&ATTRIBUTES, "MILESPLIT_RAW_ATTRIBUTES")?;
    links.find_iter(html).try_for_each(|link| {
        let attribute = |name: &str| {
            attributes.captures_iter(link.as_str()).find_map(|capture| {
                capture
                    .get(1)?
                    .as_str()
                    .eq_ignore_ascii_case(name)
                    .then(|| capture.get(2).map(|value| value.as_str()))
                    .flatten()
            })
        };
        if attribute("rel").is_some_and(|rel| {
            rel.split_ascii_whitespace()
                .any(|v| v.eq_ignore_ascii_case("canonical"))
        }) {
            let url = attribute("href")
                .ok_or_else(|| schema(reference, "canonical raw link has no URL"))?;
            check_url(url, reference, "canonical raw document URL")?;
        }
        Ok(())
    })
}

fn regex(
    expression: &'static LazyLock<RegexResult<Regex>>,
    pattern: &'static str,
) -> CrawlResult<&'static Regex> {
    expression.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern,
        source: source.clone(),
    })
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
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.host_str().is_some_and(|host| {
            host == "www.milesplit.com"
                || host
                    .strip_suffix(".milesplit.com")
                    .is_some_and(|code| code.eq_ignore_ascii_case(reference.site.code()))
        })
    {
        return false;
    }
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
