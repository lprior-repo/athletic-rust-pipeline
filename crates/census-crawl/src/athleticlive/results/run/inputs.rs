use super::{counter_error, resource, Run, MAX_CAPTURE_RECORDS};
use crate::athleticlive::results::ResultOptions;
use crate::athleticlive::wire::{event_doc_url, event_summary_url};
use crate::{AdapterContext, CrawlResult};

impl Run {
    pub(in crate::athleticlive::results) fn read_captures(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &ResultOptions,
    ) -> CrawlResult<()> {
        let captures = options
            .documents
            .len()
            .checked_add(options.standings.len())
            .and_then(|count| count.checked_add(usize::from(options.summary.is_some())))
            .ok_or_else(counter_error)?;
        if captures > MAX_CAPTURE_RECORDS {
            return Err(resource("LIVE input captures", captures));
        }
        if let Some(path) = options.summary.as_deref() {
            self.read_once(
                ctx,
                path,
                options.capture_metadata.get(path),
                |run, body, metadata| run.read_summary(path, body, metadata),
            )?;
        } else {
            self.owe(&event_summary_url(self.target.athleticlive_meet_id))?;
        }
        self.read_documents(ctx, options)?;
        self.read_standings_captures(ctx, options)?;
        self.owe_missing_documents()
    }

    fn read_documents(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &ResultOptions,
    ) -> CrawlResult<()> {
        options
            .documents
            .iter()
            .enumerate()
            .try_for_each(|(index, path)| {
                if options.limit.is_some_and(|limit| index >= limit) {
                    self.owe(path)
                } else {
                    self.read_once(
                        ctx,
                        path,
                        options.capture_metadata.get(path),
                        |run, body, metadata| run.read_document(path, body, metadata),
                    )
                }
            })
    }

    fn read_standings_captures(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &ResultOptions,
    ) -> CrawlResult<()> {
        options
            .standings
            .iter()
            .enumerate()
            .try_for_each(|(index, row)| {
                if options.limit.is_some_and(|limit| index >= limit) {
                    self.owe(&row.path)
                } else {
                    self.read_once(
                        ctx,
                        &row.path,
                        options.capture_metadata.get(&row.path),
                        |run, body, metadata| run.read_standings(row, body, metadata),
                    )
                }
            })
    }

    fn owe_missing_documents(&mut self) -> CrawlResult<()> {
        let missing = self
            .listed
            .iter()
            .filter(|id| !self.read_documents.contains(id))
            .copied()
            .collect::<Vec<_>>();
        self.stats.events_unfetched = self
            .stats
            .events_unfetched
            .checked_add(missing.len())
            .ok_or_else(counter_error)?;
        missing
            .into_iter()
            .try_for_each(|id| self.owe(&event_doc_url(id)))
    }
}
