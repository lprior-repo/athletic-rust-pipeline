use super::super::map::ProviderSchools;
use super::super::owned::read_owned_meet;
use super::super::raw::RawPage;
use super::super::wire::ResultSetRef;
use super::{EntityCounts, Stats};
use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlError, CrawlResult};

mod acquired;
mod capture;
mod evidence;
mod metadata;
mod projection;
pub(super) mod receipt;
mod windows;
use acquired::AcquiredMeet;

pub(super) struct ActiveMeet {
    key: String,
    meet: AcquiredMeet,
}

enum Intake {
    Open,
    Blocked { url: String },
}

enum MetadataCapture {
    Unacquired,
    Acquired(serde_json::Value),
}

pub(super) struct Run {
    pub(super) schools: ProviderSchools,
    pub(super) owned: Option<ActiveMeet>,
    pub(super) stats: Stats,
    pub(super) counts: EntityCounts,
    pub(super) frontier: super::frontier::Frontier,
    intake: Intake,
    metadata_capture: MetadataCapture,
}

impl Run {
    pub(super) fn new(schools: ProviderSchools) -> Self {
        Self {
            schools,
            owned: None,
            stats: Stats::default(),
            counts: EntityCounts::default(),
            frontier: super::frontier::Frontier::default(),
            intake: Intake::Open,
            metadata_capture: MetadataCapture::Unacquired,
        }
    }

    #[tracing::instrument(skip(self, ctx))]
    pub(super) async fn request(
        &mut self,
        ctx: &AdapterContext<'_>,
        request: &super::ResultSetRequest,
        ordinal: usize,
    ) -> CrawlResult<()> {
        if let Intake::Blocked { url } = &self.intake {
            self.stats.unread = self.stats.unread.saturating_add(1);
            if self.stats.unread == 1 {
                self.stats.failure(format!(
                    "resource stop at {url}; subsequent supplied locators remain unread/unfinished"
                ))?;
            }
            self.frontier.unfinished(ordinal, &request.url)?;
            return Ok(());
        }
        let failures_before = self.stats.failure_count;
        self.read_request(ctx, request).await?;
        if failures_before != self.stats.failure_count {
            self.frontier.unfinished(ordinal, &request.url)?;
        }
        Ok(())
    }

    async fn read_request(
        &mut self,
        ctx: &AdapterContext<'_>,
        request: &super::ResultSetRequest,
    ) -> CrawlResult<()> {
        if request.url.len() > 4096 {
            self.owned = None;
            self.metadata_capture = MetadataCapture::Unacquired;
            return self.resource(
                ctx,
                &request.url,
                &CrawlError::Resource {
                    resource: "result request URL",
                    requested: request.url.len(),
                    limit: 4096,
                },
            );
        }
        match ResultSetRef::parse(&request.url)
            .or_else(|| ResultSetRef::parse_with_jurisdiction(&request.url, request.jurisdiction))
        {
            Some(reference) => self.read(ctx, &reference).await,
            None => self.reject(&request.url),
        }
    }

    #[tracing::instrument(skip(self, ctx))]
    pub(super) async fn read(
        &mut self,
        ctx: &AdapterContext<'_>,
        reference: &ResultSetRef,
    ) -> CrawlResult<()> {
        self.metadata_capture = MetadataCapture::Unacquired;
        match self.read_unit(ctx, reference).await {
            Err(error @ CrawlError::Resource { .. }) => self.resource(ctx, &reference.url, &error),
            outcome => outcome,
        }
    }

    #[tracing::instrument(skip(self, ctx))]
    async fn read_unit(
        &mut self,
        ctx: &AdapterContext<'_>,
        reference: &ResultSetRef,
    ) -> CrawlResult<()> {
        self.acquire_owned(ctx, reference).await?;
        let (mut captured, page) = match metadata::fetch_metadata(ctx, reference).await {
            Ok(captured) => captured,
            Err(error) if error.retryable() => return Err(error),
            Err(error) => return self.record_failure(ctx, reference, None, &error),
        };
        self.metadata_capture =
            MetadataCapture::Acquired(capture::metadata_provenance(reference, &captured)?);
        capture::archive_metadata(ctx, reference, &captured)?;
        captured.body = Vec::new();
        match page {
            Ok(mut page) => {
                page.meet.events = Vec::new();
                self.record_page(ctx, reference, &page, &captured)
            }
            Err(error @ CrawlError::Resource { .. }) => Err(error),
            Err(error) if error.retryable() => Err(error),
            Err(error) => self.record_failure(ctx, reference, Some(&captured), &error),
        }
    }

    #[tracing::instrument(skip(self, ctx))]
    async fn acquire_owned(
        &mut self,
        ctx: &AdapterContext<'_>,
        reference: &ResultSetRef,
    ) -> CrawlResult<()> {
        let key = format!("{}/{}", reference.site.code(), reference.meet_id);
        if matches!(&self.owned, Some(ActiveMeet { key: prior, .. }) if prior == &key) {
            return Ok(());
        }
        self.owned = None;
        let mut meet = AcquiredMeet::new(read_owned_meet(ctx, reference).await?)?;
        self.stats.peak_capture_bytes = self
            .stats
            .peak_capture_bytes
            .max(meet.outcome.capture.bytes);
        meet.release_body();
        self.owned = Some(ActiveMeet { key, meet });
        Ok(())
    }

    fn record_page(
        &mut self,
        ctx: &AdapterContext<'_>,
        reference: &ResultSetRef,
        page: &RawPage,
        metadata: &FetchOutcome,
    ) -> CrawlResult<()> {
        let acquired = acquired(&self.owned)?;
        let input = projection::Input {
            acquired,
            reference,
            page,
            schools: &self.schools,
            performance_as_of: ctx.performance_as_of,
        };
        windows::execute(ctx, &input, metadata, &mut self.stats, &mut self.counts)
    }

    fn record_failure(
        &mut self,
        ctx: &AdapterContext<'_>,
        reference: &ResultSetRef,
        metadata: Option<&FetchOutcome>,
        error: &CrawlError,
    ) -> CrawlResult<()> {
        let reason = format!(
            "{}: raw meet/season metadata unavailable: {error}",
            reference.url
        );
        let entry = receipt::failure(
            reference,
            acquired(&self.owned)?,
            metadata,
            &reason,
            ctx.performance_as_of,
        )?;
        let mut batch = ctx.write_batch();
        super::journal_changed(ctx, &mut batch, &entry.0, &entry.1)?;
        batch.commit()?;
        self.stats.failure(reason)
    }

    fn resource(
        &mut self,
        ctx: &AdapterContext<'_>,
        url: &str,
        error: &CrawlError,
    ) -> CrawlResult<()> {
        let url = url
            .get(..url.len().min(4096))
            .map_or("oversized URL", |url| url);
        let payload = self.resource_payload(url, error, ctx.performance_as_of);
        let key = format!("partial/resource/{}", capture::digest(&payload)?);
        let mut batch = ctx.write_batch();
        let staged =
            super::journal_changed(ctx, &mut batch, &key, &payload).and_then(|()| batch.commit());
        let receipt = match staged {
            Ok(()) => "partial receipt retained",
            Err(CrawlError::Resource { .. }) => {
                "partial receipt cannot fit; unfinished locator retained in report"
            }
            Err(error) => return Err(error),
        };
        self.intake = Intake::Blocked { url: url.into() };
        if matches!(&self.stats.resource_stop, super::ResourceStop::Open) {
            self.stats.resource_stop =
                super::ResourceStop::Unfinished(self.resource_locator(url, error, receipt));
        }
        self.release_owned();
        self.stats.failure(format!(
            "{url}: {error}; {receipt}; previously admitted effects retained"
        ))
    }

    fn resource_payload(
        &self,
        url: &str,
        error: &CrawlError,
        performance_as_of: chrono::NaiveDate,
    ) -> serde_json::Value {
        let owned = match &self.owned {
            Some(ActiveMeet { meet, .. }) => capture::provenance(&meet.outcome.capture),
            None => serde_json::Value::Null,
        };
        let raw = match &self.metadata_capture {
            MetadataCapture::Acquired(capture) => Some(capture),
            MetadataCapture::Unacquired => None,
        };
        serde_json::json!({
            "source_url": url, "owned_capture": owned, "raw_metadata_capture": raw,
            "performance_as_of": performance_as_of,
            "disposition": "resource_limit", "unfinished": true, "resource_error": error.to_string(),
        })
    }
    pub(super) fn release_owned(&mut self) {
        self.owned = None;
        self.metadata_capture = MetadataCapture::Unacquired;
    }

    fn resource_locator(&self, url: &str, error: &CrawlError, receipt: &str) -> String {
        let owned = match &self.owned {
            Some(ActiveMeet { meet, .. }) => meet.outcome.capture.content_digest.as_str(),
            None => "unacquired",
        };
        let raw = match &self.metadata_capture {
            MetadataCapture::Acquired(capture) => capture
                .get("content_digest")
                .and_then(serde_json::Value::as_str)
                .map_or("unacquired", |digest| digest),
            MetadataCapture::Unacquired => "unacquired",
        };
        format!("{url}; owned capture {owned}; raw metadata capture {raw}; {error}; {receipt}")
    }

    pub(super) fn reject(&mut self, entry: &str) -> CrawlResult<()> {
        self.stats
            .failure(format!("{entry}: not a /meets/<id>/results/<rsid>/raw URL"))
    }
}

fn acquired(owned: &Option<ActiveMeet>) -> CrawlResult<&AcquiredMeet> {
    match owned {
        Some(ActiveMeet { meet, .. }) => Ok(meet),
        None => Err(CrawlError::Invariant {
            detail: "result projection has no acquired source-owned meet".into(),
        }),
    }
}

#[cfg(test)]
mod owned_tests;
