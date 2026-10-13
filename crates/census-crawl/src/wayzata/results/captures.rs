use super::super::{Options, PROVIDER};
use super::{budget, schema, tenant::ResolvedMeet, text_of, Page, REVISION, TENANT};
use crate::athleticlive::{event_doc_url, event_summary_url, parse_event_summary, ResultOptions};
use crate::athleticlive_athletes::MeetTarget;
use crate::net::cache::CacheMeta;
use crate::net::{FetchOptions, FetchOutcome, Fetcher};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::CanonicalMeet;
use census_domain::UsJurisdiction;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(super) struct CapturedResults {
    summary: FetchOutcome,
    summary_path: String,
    summary_meta: CacheMeta,
    pub(super) cached: Vec<(String, String, CacheMeta)>,
    refusals: Vec<String>,
}

pub(super) struct LedgerInputs<'a> {
    pub(super) slug: &'a str,
    pub(super) meet: &'a CanonicalMeet,
    pub(super) state: UsJurisdiction,
    pub(super) resolved: &'a ResolvedMeet,
    pub(super) documents: &'a [Value],
}

impl CapturedResults {
    pub(super) async fn fetch(
        ctx: &AdapterContext<'_>,
        options: &FetchOptions,
        meet_id: u64,
    ) -> CrawlResult<Self> {
        let summary_url = event_summary_url(meet_id);
        let summary = ctx.fetcher.get(&summary_url, options).await?;
        let (summary_path, summary_meta) = pinned(ctx, &summary_url, &summary)?;
        let ids = collect_event_ids(&summary, &summary_url)?;
        let mut cached = Vec::new();
        budget::reserve(&mut cached, ids.len(), budget::MAX_RESULT_DOCUMENTS)?;
        let mut refusals = Vec::new();
        for id in &ids {
            let url = event_doc_url(*id);
            match ctx.fetcher.get(&url, options).await {
                Ok(fetched) => {
                    let (path, meta) = pinned(ctx, &url, &fetched)?;
                    cached.push((path, url, meta));
                }
                Err(error) => refuse(&mut refusals, &CrawlError::Fetch(error))?,
            }
        }
        budget::check(
            "Wayzata capture metadata",
            cached.len().saturating_add(1),
            budget::MAX_RESULT_DOCUMENTS,
        )?;
        Ok(Self {
            summary,
            summary_path,
            summary_meta,
            cached,
            refusals,
        })
    }

    pub(super) fn prepare(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        meet: &CanonicalMeet,
        state: UsJurisdiction,
        meet_id: u64,
    ) -> CrawlResult<(ResultOptions, Vec<Value>)> {
        let mut metadata = BTreeMap::new();
        metadata.insert(
            self.summary_path.clone(),
            std::mem::take(&mut self.summary_meta),
        );
        for (path, _, meta) in &self.cached {
            metadata.insert(path.clone(), meta.clone());
        }
        let mut paths = Vec::new();
        budget::reserve(&mut paths, self.cached.len(), budget::MAX_RESULT_DOCUMENTS)?;
        for (path, _, _) in &self.cached {
            paths.push(path.clone());
        }
        let documents = captured_documents(&self.cached)?;
        let target = MeetTarget {
            athleticlive_meet_id: meet_id,
            meet_id: meet.id.as_str().to_string(),
            tenant: TENANT.to_string(),
            name: meet.name.clone(),
            state,
            date: meet.date.clone(),
        };
        let result_options = ResultOptions {
            meet: Some(target),
            summary: Some(self.summary_path.clone()),
            documents: paths,
            observed_on: options
                .observed_on
                .as_ref()
                .map_or_else(|| ctx.observed_on.clone(), Clone::clone),
            capture_metadata: metadata,
            ..ResultOptions::default()
        };
        Ok((result_options, documents))
    }

    pub(super) fn identity(&self, inputs: &LedgerInputs<'_>) -> Value {
        let LedgerInputs {
            slug,
            meet,
            state,
            resolved,
            documents,
        } = *inputs;
        json!({
            "role": "results_ledger",
            "revision": REVISION,
            "provider": PROVIDER,
            "tenant": TENANT,
            "slug": slug,
            "meet_id": meet.id.as_str(),
            "athleticlive_meet_id": resolved.id,
            "state": state.code(),
            "link_url": &resolved.link,
            "link_response_url": &resolved.response.response_url,
            "summary_url": &self.summary.url,
            "summary_sha256": &self.summary.content_digest,
            "summary_fetched_at": &self.summary.fetched_at,
            "documents": documents,
        })
    }

    pub(super) fn payload(
        &self,
        page: &Page<'_>,
        inputs: &LedgerInputs<'_>,
        report: &AdapterReport,
    ) -> Value {
        let LedgerInputs {
            slug,
            meet,
            state,
            resolved,
            documents,
        } = *inputs;
        json!({
            "role": "results_ledger",
            "revision": REVISION,
            "provider": PROVIDER,
            "tenant": TENANT,
            "slug": slug,
            "meet_id": meet.id.as_str(),
            "meet_name": &meet.name,
            "meet_date": &meet.date,
            "state": state.code(),
            "athleticlive_meet_id": resolved.id,
            "athletic_net_meet_id": &resolved.tenant.source.athletic_net_id,
            "tenant_meet_name": &resolved.tenant.source.name,
            "tenant_meet_start": &resolved.tenant.source.start,
            "tenant_meet_key": &resolved.tenant.id,
            "schedule_url": &page.fetched.url,
            "schedule_sha256": &page.fetched.content_digest,
            "link_url": &resolved.link,
            "link_response_url": &resolved.response.response_url,
            "link_fetched_at": &resolved.response.fetched_at,
            "summary_url": &self.summary.url,
            "summary_sha256": &self.summary.content_digest,
            "summary_fetched_at": &self.summary.fetched_at,
            "documents": documents,
            "refused_documents": &self.refusals,
            "new_performances": report.rows,
            "errors": report.errors,
            "rejections": report.rejections,
            "unresolved": &report.unresolved,
            "owed": &report.unfinished,
            "notes": &report.notes,
        })
    }
}

fn collect_event_ids(summary: &FetchOutcome, url: &str) -> CrawlResult<Vec<u64>> {
    let listed = parse_event_summary(url, text_of(summary, url)?)?;
    let mut ids = Vec::new();
    budget::reserve(&mut ids, listed.len(), budget::MAX_RESULT_DOCUMENTS)?;
    for event in &listed {
        if event.is_relay() {
            continue;
        }
        if let Some(event_id) = event.event_id() {
            ids.push(event_id);
        }
    }
    Ok(ids)
}

fn captured_documents(cached: &[(String, String, CacheMeta)]) -> CrawlResult<Vec<Value>> {
    let mut documents: Vec<Value> = Vec::new();
    budget::reserve(&mut documents, cached.len(), budget::MAX_RESULT_DOCUMENTS)?;
    for (_, url, meta) in cached {
        documents.push(json!({
            "url": url,
            "sha256": &meta.content_digest,
            "fetched_at": &meta.fetched_at,
            "bytes": meta.bytes,
        }));
    }
    Ok(documents)
}

fn refuse(refusals: &mut Vec<String>, error: &CrawlError) -> CrawlResult<()> {
    if refusals.len() >= budget::MAX_RESULT_REFUSALS {
        return Ok(());
    }
    budget::reserve(refusals, 1, budget::MAX_RESULT_REFUSALS)?;
    refusals.push(budget::detail(error)?);
    Ok(())
}

fn pinned(
    ctx: &AdapterContext<'_>,
    url: &str,
    fetched: &FetchOutcome,
) -> CrawlResult<(String, CacheMeta)> {
    if fetched.method != "GET"
        || fetched.status != 200
        || fetched.url != url
        || fetched.bytes != fetched.body.len()
    {
        return Err(schema(url, "published capture metadata is inconsistent"));
    }
    let key = Fetcher::key_for("GET", url, "");
    let (body_path, _) = ctx.fetcher.cache_paths(&key);
    let path = body_path.to_str().ok_or_else(|| CrawlError::Invariant {
        detail: "the capture cache path is not UTF-8".to_string(),
    })?;
    let meta = CacheMeta {
        url: fetched.url.clone(),
        response_url: fetched.response_url.clone(),
        method: fetched.method.clone(),
        status: fetched.status,
        content_digest: fetched.content_digest.clone(),
        bytes: fetched.bytes,
        fetched_at: fetched.fetched_at.clone(),
        content_type: fetched.content_type.clone(),
        ..CacheMeta::default()
    };
    Ok((path.to_string(), meta))
}
