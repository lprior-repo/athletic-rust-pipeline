use super::{parse, MeetRef, Season, Site};
use crate::net::{FetchError, FetchOptions, Fetcher};
use crate::{CrawlError, CrawlResult};
use std::num::NonZeroU32;

const MAX_INDEX_BYTES: usize = 2 * 1024 * 1024;

mod ownership;
#[cfg(test)]
mod tests;

pub struct MeetIndexRequest {
    pub site: Site,
    pub season: Season,
    pub year: u16,
    pub page: NonZeroU32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageContinuation {
    More,
    End,
}

pub struct MeetIndexPage {
    pub meets: Vec<MeetRef>,
    pub continuation: PageContinuation,
    pub from_cache: bool,
}

pub async fn fetch_meet_index(
    fetcher: &Fetcher,
    request: &MeetIndexRequest,
    options: &FetchOptions,
) -> CrawlResult<MeetIndexPage> {
    let url = request
        .site
        .results_url(request.season, request.year, request.page.get());
    let outcome = fetcher.get(&url, options).await?;
    if outcome.status != 200 {
        return Err(CrawlError::Fetch(FetchError::Http {
            status: outcome.status,
            url,
        }));
    }
    if outcome.body.len() > MAX_INDEX_BYTES {
        return Err(CrawlError::Resource {
            resource: "meet index capture bytes",
            requested: outcome.body.len(),
            limit: MAX_INDEX_BYTES,
        });
    }
    if outcome.url != url {
        return Err(schema(
            &outcome.url,
            "capture URL does not match the requested meet index",
        ));
    }
    let body =
        std::str::from_utf8(&outcome.body).map_err(|_| schema(&url, "meet index is not UTF-8"))?;
    validate_surface(body, &outcome.url)?;
    let meets = parse::parse_meet_index(body)?;
    let continuation = if parse::has_next_page(body) {
        PageContinuation::More
    } else {
        PageContinuation::End
    };
    Ok(MeetIndexPage {
        meets,
        continuation,
        from_cache: outcome.from_cache,
    })
}

pub(super) fn validate_surface(body: &str, url: &str) -> CrawlResult<()> {
    if body.len() > 8 * 1024 * 1024 {
        return Err(CrawlError::Resource {
            resource: "index representation bytes",
            requested: body.len(),
            limit: 8 * 1024 * 1024,
        });
    }
    let nodes = body
        .bytes()
        .filter(|byte| *byte == b'<')
        .take(65_537)
        .count();
    if nodes > 65_536 {
        return Err(CrawlError::Resource {
            resource: "index representation nodes",
            requested: nodes,
            limit: 65_536,
        });
    }
    ownership::validate(body, url)
}

fn schema(url: &str, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: url.to_owned(),
        detail: detail.to_owned(),
    }
}
