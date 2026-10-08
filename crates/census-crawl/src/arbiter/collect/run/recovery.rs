use super::Run;
use crate::arbiter::MAX_PAGES;
use crate::net::FetchOutcome;
use crate::recording::RowBatch;
use crate::{CrawlError, CrawlResult};
use census_domain::model::SchoolId;
use census_store::Store;
use serde::{Deserialize, Serialize};

const PHASE: &str = "arbiter_coaches_incomplete_v2";
const KEY: &str = "pending";

#[derive(Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Recovery {
    responses: Vec<IncompleteResponse>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct IncompleteResponse {
    url: String,
    response_url: String,
    method: String,
    status: u16,
    content_digest: String,
    fetched_at: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    version: u8,
    owner_key: String,
    school_id: SchoolId,
    kind: String,
    failure: String,
    recovery: Recovery,
}

pub(super) fn phase(owner_key: &str) -> String {
    format!("{PHASE}:{owner_key}")
}

impl Recovery {
    pub(super) fn load(store: &Store, school_id: &SchoolId, owner_key: &str) -> CrawlResult<Self> {
        let phase = phase(owner_key);
        let mut payloads = store.journal_payloads(&phase)?;
        if payloads.len() > 1 {
            return Err(CrawlError::Schema {
                url: phase,
                detail: "multiple incomplete coach acquisition markers for one public owner"
                    .to_string(),
            });
        }
        let Some(payload) = payloads.pop() else {
            return Ok(Self::default());
        };
        let pending: Pending =
            serde_json::from_value(payload).map_err(|source| CrawlError::Decode {
                url: phase.clone(),
                source,
            })?;
        if pending.version != 2
            || pending.owner_key != owner_key
            || pending.school_id != *school_id
            || u64::try_from(pending.recovery.responses.len())
                .map_or(true, |count| count > MAX_PAGES)
        {
            return Err(CrawlError::Schema {
                url: phase,
                detail: "invalid public-owner incomplete coach acquisition marker".to_string(),
            });
        }
        Ok(pending.recovery)
    }

    pub(super) async fn fetch(&mut self, run: &Run<'_>, url: &str) -> CrawlResult<FetchOutcome> {
        let outcome = run.ctx.fetcher.get(url, &run.fetch).await?;
        let known_incomplete = outcome.from_cache
            && self.responses.iter().any(|response| {
                response.url == url
                    && response.response_url == outcome.url
                    && response.method == outcome.method
                    && response.status == outcome.status
                    && response.content_digest == outcome.content_digest
            });
        let outcome = if known_incomplete {
            let mut refresh = run.fetch.clone();
            refresh.refresh = true;
            run.ctx.fetcher.get(url, &refresh).await?
        } else {
            outcome
        };
        self.responses.retain(|response| response.url != url);
        Ok(outcome)
    }

    pub(super) fn remember(&mut self, url: &str, outcome: &FetchOutcome) -> CrawlResult<()> {
        self.responses.retain(|response| response.url != url);
        if u64::try_from(self.responses.len()).map_or(true, |count| count >= MAX_PAGES) {
            return Err(CrawlError::Schema {
                url: url.to_string(),
                detail: "incomplete coach response marker exceeds the page bound".to_string(),
            });
        }
        self.responses.try_reserve(1).map_err(|_| {
            crate::directory::acquisition::resource(
                "recovery allocation",
                1,
                usize::try_from(MAX_PAGES).map_or(usize::MAX, |value| value),
            )
        })?;
        self.responses.push(IncompleteResponse {
            url: url.to_string(),
            response_url: outcome.url.clone(),
            method: outcome.method.clone(),
            status: outcome.status,
            content_digest: outcome.content_digest.clone(),
            fetched_at: outcome.fetched_at.clone(),
        });
        Ok(())
    }

    pub(super) fn persist(
        &self,
        batch: &mut RowBatch<'_>,
        school_id: &SchoolId,
        owner_key: &str,
        error: &CrawlError,
    ) -> CrawlResult<()> {
        batch.journal_done(
            &phase(owner_key),
            KEY,
            &serde_json::json!({
                "kind": super::coaches::failure_kind(error),
                "version": 2,
                "owner_key": owner_key,
                "school_id": school_id,
                "failure": crate::directory::acquisition::detail(owner_key, error)?,
                "recovery": self,
            }),
        )
    }
}
