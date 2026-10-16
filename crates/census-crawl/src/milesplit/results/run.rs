use super::super::map::ProviderSchools;
use super::super::owned::read_owned_meet;
use super::super::raw::RawPage;
use super::super::wire::ResultSetRef;
use super::{EntityCounts, Stats};
use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlError, CrawlResult};

mod acquired;
pub(super) mod capture;
mod evidence;
pub(super) mod metadata;
mod projection;
mod resource;
use resource::resource_capture;
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
        match Box::pin(self.read_unit(ctx, reference)).await {
            Err(
                error @ (CrawlError::Resource { .. }
                | CrawlError::OwnedParseResource(_)
                | CrawlError::OwnedCaptureResource(_)),
            ) => {
                if let Some(capture) = resource_capture(&error) {
                    self.stats.peak_capture_bytes =
                        self.stats.peak_capture_bytes.max(capture.bytes);
                }
                self.resource(ctx, &reference.url, &error)
            }
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
        meet.require_capture_context(ctx.capture)?;
        self.stats.peak_capture_bytes = self
            .stats
            .peak_capture_bytes
            .max(meet.outcome.capture().bytes);
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

    pub(super) fn release_owned(&mut self) {
        self.owned = None;
        self.metadata_capture = MetadataCapture::Unacquired;
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
