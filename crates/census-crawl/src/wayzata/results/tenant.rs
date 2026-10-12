use super::super::BASE;
use super::{schema, search_query, search_url, text_of, RESULTS_HOST, SEARCH_INDEX};
use crate::net::{FetchOptions, FetchOutcome};
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::CanonicalMeet;
use serde::Deserialize;

pub(super) struct ResolvedMeet {
    pub(super) id: u64,
    pub(super) link: String,
    pub(super) response: FetchOutcome,
    pub(super) tenant: SearchHit,
}

impl ResolvedMeet {
    pub(super) async fn fetch(
        ctx: &AdapterContext<'_>,
        options: &FetchOptions,
        slug: &str,
    ) -> CrawlResult<Self> {
        let link = format!("{BASE}/links/{slug}");
        let response = ctx.fetcher.get(&link, options).await?;
        let id = meet_id_of(&response, &link)?;
        let tenant = qualify(ctx, options, id).await?;
        Ok(Self {
            id,
            link,
            response,
            tenant,
        })
    }

    pub(super) fn validate_date(&self, meet: &CanonicalMeet) -> CrawlResult<()> {
        let date = self
            .tenant
            .source
            .start
            .as_deref()
            .and_then(|start| chrono::DateTime::parse_from_rfc3339(start).ok())
            .map(|start| start.date_naive());
        let scheduled = chrono::NaiveDate::parse_from_str(&meet.date, "%Y-%m-%d").ok();
        if date.is_none() || date != scheduled {
            return Err(schema(
                &self.link,
                "tenant meet date does not match the published schedule",
            ));
        }
        Ok(())
    }
}

fn meet_id_of(fetched: &FetchOutcome, link: &str) -> CrawlResult<u64> {
    let resolved = fetched.response_url.as_deref().ok_or_else(|| {
        schema(
            link,
            "the provider link did not resolve to a results destination",
        )
    })?;
    let parsed = url::Url::parse(resolved)
        .map_err(|_| schema(link, "the results destination is not a URL"))?;
    if parsed.scheme() != "https" || parsed.host_str() != Some(RESULTS_HOST) {
        return Err(schema(
            link,
            "the provider link resolved outside the results origin",
        ));
    }
    let mut segments = parsed
        .path_segments()
        .ok_or_else(|| schema(link, "the results destination carries no path"))?;
    let head = segments.next();
    let id = segments.next().map_or("", core::convert::identity);
    if head != Some("meets") || segments.next().is_some() {
        return Err(schema(link, "the results destination is not a meet page"));
    }
    id.parse::<u64>()
        .map_err(|_| schema(link, "the results destination carries no numeric meet id"))
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    hits: SearchHits,
}

#[derive(Debug, Default, Deserialize)]
struct SearchHits {
    #[serde(default)]
    hits: Vec<SearchHit>,
}

#[derive(Debug, Deserialize)]
pub(super) struct SearchHit {
    #[serde(default, rename = "_index")]
    index: Option<String>,
    #[serde(default, rename = "_id")]
    pub(super) id: Option<String>,
    #[serde(default, rename = "_source")]
    pub(super) source: SearchSource,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct SearchSource {
    #[serde(default, rename = "i")]
    meet_id: Option<u64>,
    #[serde(default, rename = "ani")]
    pub(super) athletic_net_id: Option<u64>,
    #[serde(default, rename = "n")]
    pub(super) name: Option<String>,
    #[serde(default, rename = "sd")]
    pub(super) start: Option<String>,
}

async fn qualify(
    ctx: &AdapterContext<'_>,
    fetch_options: &FetchOptions,
    meet_id: u64,
) -> CrawlResult<SearchHit> {
    let url = search_url();
    let query = search_query(meet_id);
    let fetched = ctx.fetcher.post_json(&url, &query, fetch_options).await?;
    let response: SearchResponse =
        serde_json::from_str(text_of(&fetched, &url)?).map_err(|source| CrawlError::Decode {
            url: url.clone(),
            source,
        })?;
    response
        .hits
        .hits
        .into_iter()
        .find(|hit| {
            hit.index.as_deref() == Some(SEARCH_INDEX) && hit.source.meet_id == Some(meet_id)
        })
        .ok_or_else(|| {
            schema(
                &url,
                "the published tenant index carries no qualified meet hit",
            )
        })
}
