use super::{journal, Writer};
use crate::{AdapterContext, CrawlError, CrawlResult};

impl Writer<'_, '_> {
    pub(super) fn finish(mut self, ctx: &AdapterContext<'_>) -> CrawlResult<()> {
        let summary = self.window.ordinal > 0;
        match self.flush(ctx, true) {
            Ok(()) => {}
            Err(error @ CrawlError::Resource { .. }) => return self.blocked(ctx, &error),
            Err(error) => return Err(error),
        }
        if summary {
            journal::commit(
                ctx,
                &journal::summary(self.input, self.metadata, &self.window)?,
            )?;
        }
        if !self.window.complete && self.stats.failure_count == self.failures_before {
            let detail = match self.input.acquired.failure() {
                Some(reason) => reason,
                None => "source-owned observations retained unresolved; exact provider school, published cohort, name or sport metadata unavailable".into(),
            };
            self.stats
                .failure(format!("{}: {detail}", self.input.reference.url))?;
        }
        Ok(())
    }

    pub(super) fn blocked(
        &mut self,
        ctx: &AdapterContext<'_>,
        error: &CrawlError,
    ) -> CrawlResult<()> {
        self.window.complete = false;
        let index = self.window.indices.first().copied();
        self.stats.resource_stop = crate::milesplit::results::ResourceStop::Unfinished(format!(
            "{}; owned capture {}; first pending owned row index {index:?}; {error}",
            self.input.reference.url, self.input.acquired.outcome.capture.content_digest
        ));
        let entry = journal::blocked(self.input, self.metadata, index, error)?;
        match journal::commit(ctx, &entry) {
            Ok(()) => {}
            Err(CrawlError::Resource { .. }) => self.stats.failure("recording is full; unfinished owned locator is report-only, no partial journal receipt was staged".into())?,
            Err(error) => return Err(error),
        }
        self.stats.failure(format!(
            "{}: {error}; unfinished owned locator {index:?}; previously admitted windows retained",
            self.input.reference.url
        ))?;
        match error {
            CrawlError::Resource {
                resource,
                requested,
                limit,
            } => Err(CrawlError::Resource {
                resource,
                requested: *requested,
                limit: *limit,
            }),
            _ => Err(CrawlError::Invariant {
                detail: "non-resource result window was stopped".into(),
            }),
        }
    }
}
